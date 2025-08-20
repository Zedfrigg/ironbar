use crate::channels::{AsyncSenderExt, BroadcastReceiverExt};
use crate::clients::volume::{self, Event};
use crate::config::CommonConfig;
use crate::gtk_helpers::IronbarGtkExt;
use crate::modules::{
    Module, ModuleInfo, ModuleParts, ModuleUpdateEvent, PopupButton, WidgetContext,
};
use crate::{lock, module_impl, spawn};
use gtk::prelude::*;
use gtk::{Button, Image};
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::trace;

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct VolumeModule {
    #[serde(default = "default_icon_size")]
    icon_size: i32,

    // -- Common --
    /// See [common options](module-level-options#common-options).
    #[serde(flatten)]
    pub common: Option<CommonConfig>,
}

const fn default_icon_size() -> i32 {
    24
}

#[derive(Debug, Clone)]
pub enum Update {}

impl Module<Button> for VolumeModule {
    type SendMessage = Event;
    type ReceiveMessage = Update;

    module_impl!("volume");

    fn spawn_controller(
        &self,
        _info: &ModuleInfo,
        context: &WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _rx: mpsc::Receiver<Self::ReceiveMessage>,
    ) -> color_eyre::Result<()>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let client = context.client::<volume::Client>();

        {
            let client = client.clone();
            let mut rx = client.subscribe();
            let tx = context.tx.clone();

            spawn(async move {
                // init
                let sinks = {
                    let sinks = client.sinks();
                    let sinks = lock!(sinks);
                    sinks.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial syncs: {sinks:?}");

                let inputs = {
                    let inputs = client.sink_inputs();
                    let inputs = lock!(inputs);
                    inputs.iter().cloned().collect::<Vec<_>>()
                };

                trace!("initial inputs: {inputs:?}");

                for sink in sinks {
                    tx.send_update(Event::AddSink(sink)).await;
                }

                for _input in inputs {
                    tx.send_update(Event::AddInput).await;
                }

                // recv loop
                while let Ok(event) = rx.recv().await {
                    trace!("received event: {event:?}");
                    tx.send_update(event).await;
                }
            });
        }

        Ok(())
    }

    fn into_widget(
        self,
        context: WidgetContext<Self::SendMessage, Self::ReceiveMessage>,
        _info: &ModuleInfo,
    ) -> color_eyre::Result<ModuleParts<Button>>
    where
        <Self as Module<Button>>::SendMessage: Clone,
    {
        let button = Button::new();

        {
            let tx = context.tx.clone();

            button.connect_clicked(move |button| {
                tx.send_spawn(ModuleUpdateEvent::TogglePopup(button.popup_id()));
            });
        }

        let rx = context.subscribe();

        let image_icon = Image::new();
        image_icon.add_class("icon");
        button.set_image(Some(&image_icon));

        rx.recv_glib_async((), move |(), event| {
            let image_icon = image_icon.clone();
            let image_provider = context.ironbar.image_provider();

            async move {
                match event {
                    Event::AddSink(sink) | Event::UpdateSink(sink) if sink.active => {
                        image_provider
                            .load_into_image_silent(
                                &determine_volume_icon(sink.muted, sink.volume),
                                self.icon_size,
                                false,
                                &image_icon,
                            )
                            .await;
                    }
                    _ => {}
                }
            }
        });

        Ok(ModuleParts::new(button, None))
    }
}

fn determine_volume_icon(muted: bool, volume: f64) -> String {
    let icon_variant = if muted {
        "muted"
    } else if volume <= 33.3333 {
        "low"
    } else if volume <= 66.6667 {
        "medium"
    } else {
        "high"
    };
    format!("audio-volume-{icon_variant}-symbolic")
}
