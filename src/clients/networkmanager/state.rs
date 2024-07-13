use color_eyre::Result;

use crate::clients::networkmanager::dbus::{
    AccessPointDbusProxyBlocking, ActiveConnectionDbusProxyBlocking, DeviceDbusProxyBlocking,
    DeviceState, DeviceType, DeviceWirelessDbusProxyBlocking,
};
use crate::clients::networkmanager::PathMap;

#[derive(Clone, Debug)]
pub struct State {
    pub wired: WiredState,
    pub wifi: WifiState,
    pub cellular: CellularState,
    pub vpn: VpnState,
}

#[derive(Clone, Debug)]
pub enum WiredState {
    Connected,
    Disconnected,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum WifiState {
    Connected(WifiConnectedState),
    Disconnected,
    Disabled,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct WifiConnectedState {
    pub ssid: String,
    /// Strength in percentage, from 0 to 100.
    pub strength: u8,
}

#[derive(Clone, Debug)]
pub enum CellularState {
    Connected,
    Disconnected,
    Disabled,
    NotPresent,
    Unknown,
}

#[derive(Clone, Debug)]
pub enum VpnState {
    Connected(VpnConnectedState),
    Disconnected,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct VpnConnectedState {
    pub name: String,
}

pub(super) fn determine_wired_state(
    devices: &PathMap<DeviceDbusProxyBlocking>,
) -> Result<WiredState> {
    let mut present = false;
    let mut connected = false;

    for device in devices.values() {
        if device.device_type()? == DeviceType::Ethernet {
            present = true;
            if device.state()?.is_enabled() {
                connected = true;
                break;
            }
        }
    }

    if connected {
        Ok(WiredState::Connected)
    } else if present {
        Ok(WiredState::Disconnected)
    } else {
        Ok(WiredState::NotPresent)
    }
}

pub(super) fn determine_wifi_state(
    dbus_connection: &zbus::blocking::Connection,
    devices: &PathMap<DeviceDbusProxyBlocking>,
) -> Result<WifiState> {
    let mut present = false;
    let mut enabled = false;
    let mut connected = None;

    for device in devices.values() {
        if device.device_type()? == DeviceType::Wifi {
            present = true;
            if device.state()?.is_enabled() {
                enabled = true;

                let wireless_device = DeviceWirelessDbusProxyBlocking::builder(dbus_connection)
                    .path(device.path().clone())?
                    .build()?;
                let primary_access_point_path = wireless_device.active_access_point()?;
                if primary_access_point_path.as_str() != "/" {
                    connected = Some(
                        AccessPointDbusProxyBlocking::builder(dbus_connection)
                            .path(primary_access_point_path)?
                            .build()?,
                    );
                    break;
                }
            }
        }
    }

    if let Some(access_point) = connected {
        let ssid = access_point
            .ssid()
            .map(|x| String::from_utf8_lossy(&x).to_string())
            .unwrap_or_else(|_| "unkown".into());
        Ok(WifiState::Connected(WifiConnectedState {
            ssid,
            strength: access_point.strength().unwrap_or(0),
        }))
    } else if enabled {
        Ok(WifiState::Disconnected)
    } else if present {
        Ok(WifiState::Disabled)
    } else {
        Ok(WifiState::NotPresent)
    }
}

pub(super) fn determine_cellular_state(
    devices: &PathMap<DeviceDbusProxyBlocking>,
) -> Result<CellularState> {
    let mut present = false;
    let mut enabled = false;
    let mut connected = false;

    for device in devices.values() {
        if device.device_type()? == DeviceType::Modem {
            present = true;
            if device.state()?.is_enabled() {
                enabled = true;
                if device.state()? == DeviceState::Activated {
                    connected = true;
                    break;
                }
            }
        }
    }

    if connected {
        Ok(CellularState::Connected)
    } else if enabled {
        Ok(CellularState::Disconnected)
    } else if present {
        Ok(CellularState::Disabled)
    } else {
        Ok(CellularState::NotPresent)
    }
}

pub(super) fn determine_vpn_state(
    active_connections: &PathMap<ActiveConnectionDbusProxyBlocking>,
) -> Result<VpnState> {
    for connection in active_connections.values() {
        match connection.type_()?.as_str() {
            "vpn" | "wireguard" => {
                return Ok(VpnState::Connected(VpnConnectedState {
                    name: "unknown".into(),
                }));
            }
            _ => {}
        }
    }
    Ok(VpnState::Disconnected)
}
