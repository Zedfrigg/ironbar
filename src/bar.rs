use crate::config::{BarPosition, MarginConfig, ModuleConfig};
use crate::modules::{
    create_module, set_widget_identifiers, wrap_widget, ModuleInfo, ModuleLocation,
};
use crate::popup::Popup;
use crate::{Config, Ironbar};
use color_eyre::Result;
use gtk::gdk::Monitor;
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, IconTheme, Orientation};
use std::cell::RefCell;
use std::rc::Rc;
use tracing::{debug, info};

#[derive(Debug, Clone)]
enum Inner {
    New { config: Option<Config> },
    Loaded { popup: Rc<RefCell<Popup>> },
}

#[derive(Debug, Clone)]
pub struct Bar {
    name: String,
    monitor_name: String,
    position: BarPosition,

    window: ApplicationWindow,

    content: gtk::Box,

    start: gtk::Box,
    center: gtk::Box,
    end: gtk::Box,

    inner: Inner,
}

impl Bar {
    pub fn new(app: &Application, monitor_name: String, config: Config) -> Self {
        let window = ApplicationWindow::builder().application(app).build();
        let name = config
            .name
            .clone()
            .unwrap_or_else(|| format!("bar-{}", Ironbar::unique_id()));

        window.set_widget_name(&name);

        let position = config.position;
        let orientation = position.get_orientation();

        let content = gtk::Box::builder()
            .orientation(orientation)
            .spacing(0)
            .hexpand(false)
            .name("bar");

        let content = if orientation == Orientation::Horizontal {
            content.height_request(config.height)
        } else {
            content.width_request(config.height)
        }
        .build();

        content.style_context().add_class("container");

        let start = create_container("start", orientation);
        let center = create_container("center", orientation);
        let end = create_container("end", orientation);

        content.add(&start);
        content.set_center_widget(Some(&center));
        content.pack_end(&end, false, false, 0);

        window.add(&content);

        window.connect_destroy_event(|_, _| {
            info!("Shutting down");
            gtk::main_quit();
            Inhibit(false)
        });

        Bar {
            name,
            monitor_name,
            position,
            window,
            content,
            start,
            center,
            end,
            inner: Inner::New {
                config: Some(config),
            },
        }
    }

    pub fn init(mut self, monitor: &Monitor) -> Result<Self> {
        let Inner::New { ref mut config } = self.inner else {
            return Ok(self);
        };

        let Some(config) = config.take() else {
            return Ok(self);
        };

        info!(
            "Initializing bar '{}' on '{}'",
            self.name, self.monitor_name
        );

        self.setup_layer_shell(config.anchor_to_edges, config.margin, monitor);

        let load_result = self.load_modules(config, monitor)?;

        self.show();

        self.inner = Inner::Loaded {
            popup: load_result.popup,
        };
        Ok(self)
    }

    /// Sets up GTK layer shell for a provided application window.
    fn setup_layer_shell(&self, anchor_to_edges: bool, margin: MarginConfig, monitor: &Monitor) {
        let win = &self.window;
        let position = self.position;

        gtk_layer_shell::init_for_window(win);
        gtk_layer_shell::set_monitor(win, monitor);
        gtk_layer_shell::set_layer(win, gtk_layer_shell::Layer::Top);
        gtk_layer_shell::auto_exclusive_zone_enable(win);
        gtk_layer_shell::set_namespace(win, env!("CARGO_PKG_NAME"));

        gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Top, margin.top);
        gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Bottom, margin.bottom);
        gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Left, margin.left);
        gtk_layer_shell::set_margin(win, gtk_layer_shell::Edge::Right, margin.right);

        let bar_orientation = position.get_orientation();

        gtk_layer_shell::set_anchor(
            win,
            gtk_layer_shell::Edge::Top,
            position == BarPosition::Top
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        gtk_layer_shell::set_anchor(
            win,
            gtk_layer_shell::Edge::Bottom,
            position == BarPosition::Bottom
                || (bar_orientation == Orientation::Vertical && anchor_to_edges),
        );
        gtk_layer_shell::set_anchor(
            win,
            gtk_layer_shell::Edge::Left,
            position == BarPosition::Left
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
        gtk_layer_shell::set_anchor(
            win,
            gtk_layer_shell::Edge::Right,
            position == BarPosition::Right
                || (bar_orientation == Orientation::Horizontal && anchor_to_edges),
        );
    }

    /// Loads the configured modules onto a bar.
    fn load_modules(&self, config: Config, monitor: &Monitor) -> Result<BarLoadResult> {
        let icon_theme = IconTheme::new();
        if let Some(ref theme) = config.icon_theme {
            icon_theme.set_custom_theme(Some(theme));
        }

        let app = &self.window.application().expect("to exist");

        macro_rules! info {
            ($location:expr) => {
                ModuleInfo {
                    app,
                    bar_position: config.position,
                    monitor,
                    output_name: &self.monitor_name,
                    location: $location,
                    icon_theme: &icon_theme,
                }
            };
        }

        // popup ignores module location so can bodge this for now
        let popup = Popup::new(&info!(ModuleLocation::Left), config.popup_gap);
        let popup = Rc::new(RefCell::new(popup));

        if let Some(modules) = config.start {
            let info = info!(ModuleLocation::Left);
            add_modules(&self.start, modules, &info, &popup)?;
        }

        if let Some(modules) = config.center {
            let info = info!(ModuleLocation::Center);
            add_modules(&self.center, modules, &info, &popup)?;
        }

        if let Some(modules) = config.end {
            let info = info!(ModuleLocation::Right);
            add_modules(&self.end, modules, &info, &popup)?;
        }

        let result = BarLoadResult { popup };

        Ok(result)
    }

    fn show(&self) {
        debug!("Showing bar: {}", self.name);

        // show each box but do not use `show_all`.
        // this ensures `show_if` option works as intended.
        self.start.show();
        self.center.show();
        self.end.show();
        self.content.show();
        self.window.show();
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn popup(&self) -> Rc<RefCell<Popup>> {
        match &self.inner {
            Inner::New { .. } => {
                panic!("Attempted to get popup of uninitialized bar. This is a serious bug!")
            }
            Inner::Loaded { popup } => popup.clone(),
        }
    }
}

/// Creates a `gtk::Box` container to place widgets inside.
fn create_container(name: &str, orientation: Orientation) -> gtk::Box {
    let container = gtk::Box::builder()
        .orientation(orientation)
        .spacing(0)
        .name(name)
        .build();

    container.style_context().add_class("container");
    container
}

#[derive(Debug)]
struct BarLoadResult {
    popup: Rc<RefCell<Popup>>,
}

/// Adds modules into a provided GTK box,
/// which should be one of its left, center or right containers.
fn add_modules(
    content: &gtk::Box,
    modules: Vec<ModuleConfig>,
    info: &ModuleInfo,
    popup: &Rc<RefCell<Popup>>,
) -> Result<()> {
    let orientation = info.bar_position.get_orientation();

    macro_rules! add_module {
        ($module:expr, $id:expr) => {{
            let common = $module.common.take().expect("common config to exist");
            let widget_parts = create_module(
                *$module,
                $id,
                common.name.clone(),
                &info,
                &Rc::clone(&popup),
            )?;
            set_widget_identifiers(&widget_parts, &common);

            let container = wrap_widget(&widget_parts.widget, common, orientation);
            content.add(&container);
        }};
    }

    for config in modules {
        let id = Ironbar::unique_id();
        match config {
            #[cfg(feature = "clipboard")]
            ModuleConfig::Clipboard(mut module) => add_module!(module, id),
            #[cfg(feature = "clock")]
            ModuleConfig::Clock(mut module) => add_module!(module, id),
            ModuleConfig::Custom(mut module) => add_module!(module, id),
            ModuleConfig::Focused(mut module) => add_module!(module, id),
            ModuleConfig::Label(mut module) => add_module!(module, id),
            ModuleConfig::Launcher(mut module) => add_module!(module, id),
            #[cfg(feature = "music")]
            ModuleConfig::Music(mut module) => add_module!(module, id),
            ModuleConfig::Script(mut module) => add_module!(module, id),
            #[cfg(feature = "sys_info")]
            ModuleConfig::SysInfo(mut module) => add_module!(module, id),
            #[cfg(feature = "tray")]
            ModuleConfig::Tray(mut module) => add_module!(module, id),
            #[cfg(feature = "upower")]
            ModuleConfig::Upower(mut module) => add_module!(module, id),
            #[cfg(feature = "workspaces")]
            ModuleConfig::Workspaces(mut module) => add_module!(module, id),
        }
    }

    Ok(())
}

pub fn create_bar(
    app: &Application,
    monitor: &Monitor,
    monitor_name: String,
    config: Config,
) -> Result<Bar> {
    let bar = Bar::new(app, monitor_name, config);
    bar.init(monitor)
}
