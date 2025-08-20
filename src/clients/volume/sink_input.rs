use super::{ArcMutVec, Client, Event};
use crate::channels::SyncSenderExt;
use crate::lock;
use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::Context;
use libpulse_binding::context::introspect::SinkInputInfo;
use libpulse_binding::context::subscribe::Operation;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use tracing::{debug, error, instrument, trace};

#[derive(Debug, Clone)]
pub struct SinkInput {
    pub index: u32,
}

impl From<&SinkInputInfo<'_>> for SinkInput {
    fn from(value: &SinkInputInfo) -> Self {
        Self { index: value.index }
    }
}

impl Client {
    #[instrument(level = "trace")]
    pub fn sink_inputs(&self) -> Arc<Mutex<Vec<SinkInput>>> {
        self.data.sink_inputs.clone()
    }
}

pub fn on_event(
    context: &Arc<Mutex<Context>>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
    op: Operation,
    i: u32,
) {
    let introspect = lock!(context).introspect();

    match op {
        Operation::New => {
            debug!("new sink input");
            introspect.get_sink_input_info(i, {
                let inputs = inputs.clone();
                let tx = tx.clone();

                move |info| add(info, &inputs, &tx)
            });
        }
        Operation::Changed => {
            debug!("sink input changed");
            introspect.get_sink_input_info(i, {
                let inputs = inputs.clone();
                let tx = tx.clone();

                move |info| update(info, &inputs, &tx)
            });
        }
        Operation::Removed => {
            debug!("sink input removed");
            remove(i, inputs, tx);
        }
    }
}

pub fn add(
    info: ListResult<&SinkInputInfo>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("adding {info:?}");

    lock!(inputs).push(info.into());
    tx.send_expect(Event::AddInput);
}

fn update(
    info: ListResult<&SinkInputInfo>,
    inputs: &ArcMutVec<SinkInput>,
    tx: &broadcast::Sender<Event>,
) {
    let ListResult::Item(info) = info else {
        return;
    };

    trace!("updating {info:?}");

    {
        let mut inputs = lock!(inputs);
        let Some(pos) = inputs.iter().position(|input| input.index == info.index) else {
            error!("received update to untracked sink input");
            return;
        };

        inputs[pos] = info.into();
    }

    tx.send_expect(Event::UpdateInput);
}

fn remove(index: u32, inputs: &ArcMutVec<SinkInput>, tx: &broadcast::Sender<Event>) {
    let inputs = lock!(inputs);

    trace!("removing {index}");

    if let Some(_pos) = inputs.iter().position(|s| s.index == index) {
        tx.send_expect(Event::RemoveInput);
    }
}
