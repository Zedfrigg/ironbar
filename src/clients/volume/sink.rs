use super::{ArcMutVec, Client, Event, volume_to_percent};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInfo;
use libpulse_binding::context::subscribe::Operation;
use libpulse_binding::def::SinkState;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, instrument, trace};

#[derive(Debug, Clone)]
pub struct Sink {
    index: u32,
    pub name: String,
    pub volume: f64,
    pub muted: bool,
    pub active: bool,
}

impl From<&SinkInfo<'_>> for Sink {
    fn from(value: &SinkInfo) -> Self {
        Self {
            index: value.index,
            name: value
                .name
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default(),
            muted: value.mute,
            volume: volume_to_percent(value.volume),
            active: value.state == SinkState::Running,
        }
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sinks(&self) -> Arc<Mutex<Vec<Sink>>> {
        self.data.sinks.clone()
    }
}

pub fn on_event(
    context: &Arc<Mutex<Context>>,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
    op: Operation,
    i: u32,
) {
    let introspect = lock!(context).introspect();

    match op {
        Operation::New => {
            debug!("new sink");
            introspect.get_sink_info_by_index(i, {
                let sinks = sinks.clone();
                let tx = tx.clone();

                move |info| add(info, &sinks, &tx)
            });
        }
        Operation::Changed => {
            debug!("sink changed");
            introspect.get_sink_info_by_index(i, {
                let sinks = sinks.clone();
                let default_sink = default_sink.clone();
                let tx = tx.clone();

                move |info| update(info, &sinks, &default_sink, &tx)
            });
        }
        Operation::Removed => {
            debug!("sink removed");
            remove(i, sinks, tx);
        }
    }
}

pub fn add(info: ListResult<&SinkInfo>, sinks: &ArcMutVec<Sink>, tx: &broadcast::Sender<Event>) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("adding {info:?}");

    lock!(sinks).push(info.into());
    tx.send_expect(Event::AddSink(info.into()));
}

fn update(
    info: ListResult<&SinkInfo>,
    sinks: &ArcMutVec<Sink>,
    default_sink: &Arc<Mutex<Option<String>>>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("updating {info:?}");

    {
        let mut sinks = lock!(sinks);
        let Some(pos) = sinks.iter().position(|sink| sink.index == info.index) else {
            error!("received update to untracked sink input");
            return;
        };

        sinks[pos] = info.into();

        // update in local copy
        if !sinks[pos].active {
            if let Some(default_sink) = &*lock!(default_sink) {
                sinks[pos].active = &sinks[pos].name == default_sink;
            }
        }
    }

    let mut sink: Sink = info.into();

    // update in broadcast copy
    if !sink.active {
        if let Some(default_sink) = &*lock!(default_sink) {
            sink.active = &sink.name == default_sink;
        }
    }

    tx.send_expect(Event::UpdateSink(sink));
}

fn remove(index: u32, sinks: &ArcMutVec<Sink>, tx: &broadcast::Sender<Event>) {
    trace!("removing {index}");

    let sinks = lock!(sinks);

    if let Some(_pos) = sinks.iter().position(|s| s.index == index) {
        tx.send_expect(Event::RemoveSink);
    }
}
