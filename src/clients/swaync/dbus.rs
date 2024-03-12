//! # D-Bus interface proxy for: `org.erikreider.swaync.cc`
//!
//! This code was generated by `zbus-xmlgen` `4.0.1` from D-Bus introspection data.
//! Source: `Interface '/org/erikreider/swaync/cc' from service 'org.erikreider.swaync.cc' on session bus`.
//!
//! You may prefer to adapt it, instead of using it verbatim.
//!
//! More information can be found in the [Writing a client proxy] section of the zbus
//! documentation.
//!
//! This type implements the [D-Bus standard interfaces], (`org.freedesktop.DBus.*`) for which the
//! following zbus API can be used:
//!
//! * [`zbus::fdo::PropertiesProxy`]
//! * [`zbus::fdo::IntrospectableProxy`]
//! * [`zbus::fdo::PeerProxy`]
//!
//! Consequently `zbus-xmlgen` did not generate code for the above interfaces.
//!
//! [Writing a client proxy]: https://dbus2.github.io/zbus/client.html
//! [D-Bus standard interfaces]: https://dbus.freedesktop.org/doc/dbus-specification.html#standard-interfaces,

#[zbus::dbus_proxy(
    interface = "org.erikreider.swaync.cc",
    default_service = "org.erikreider.swaync.cc",
    default_path = "/org/erikreider/swaync/cc"
)]
trait SwayNc {
    /// AddInhibitor method
    fn add_inhibitor(&self, application_id: &str) -> zbus::Result<bool>;

    /// ChangeConfigValue method
    fn change_config_value(
        &self,
        name: &str,
        value: &zbus::zvariant::Value<'_>,
        write_to_file: bool,
        path: &str,
    ) -> zbus::Result<()>;

    /// ClearInhibitors method
    fn clear_inhibitors(&self) -> zbus::Result<bool>;

    /// CloseAllNotifications method
    fn close_all_notifications(&self) -> zbus::Result<()>;

    /// CloseNotification method
    fn close_notification(&self, id: u32) -> zbus::Result<()>;

    /// GetDnd method
    fn get_dnd(&self) -> zbus::Result<bool>;

    /// GetSubscribeData method
    fn get_subscribe_data(&self) -> zbus::Result<(bool, bool, u32, bool)>;

    /// GetVisibility method
    fn get_visibility(&self) -> zbus::Result<bool>;

    /// HideLatestNotifications method
    fn hide_latest_notifications(&self, close: bool) -> zbus::Result<()>;

    /// IsInhibited method
    fn is_inhibited(&self) -> zbus::Result<bool>;

    /// NotificationCount method
    fn notification_count(&self) -> zbus::Result<u32>;

    /// NumberOfInhibitors method
    fn number_of_inhibitors(&self) -> zbus::Result<u32>;

    /// ReloadConfig method
    fn reload_config(&self) -> zbus::Result<()>;

    /// ReloadCss method
    fn reload_css(&self) -> zbus::Result<bool>;

    /// RemoveInhibitor method
    fn remove_inhibitor(&self, application_id: &str) -> zbus::Result<bool>;

    /// SetDnd method
    fn set_dnd(&self, state: bool) -> zbus::Result<()>;

    /// SetVisibility method
    fn set_visibility(&self, visibility: bool) -> zbus::Result<()>;

    /// ToggleDnd method
    fn toggle_dnd(&self) -> zbus::Result<bool>;

    /// ToggleVisibility method
    fn toggle_visibility(&self) -> zbus::Result<()>;

    /// Subscribe signal
    #[dbus_proxy(signal)]
    fn subscribe(&self, count: u32, dnd: bool, cc_open: bool) -> zbus::Result<()>;

    /// SubscribeV2 signal
    #[dbus_proxy(signal)]
    fn subscribe_v2(
        &self,
        count: u32,
        dnd: bool,
        cc_open: bool,
        inhibited: bool,
    ) -> zbus::Result<()>;

    /// Inhibited property
    #[dbus_proxy(property)]
    fn inhibited(&self) -> zbus::Result<bool>;
    #[dbus_proxy(property)]
    fn set_inhibited(&self, value: bool) -> zbus::Result<()>;
}
