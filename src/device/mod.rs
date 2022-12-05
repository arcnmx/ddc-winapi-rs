//! [SetupApi][si]: [Device Information][di]
//!
//! Exposes information about display devices and drivers, including display information such as
//! cached EDID data.
//!
//! [si]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/setupapi
//! [di]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/using-device-installation-functions#device-information-functions

// NOTE: relevant info can be found online if you know where to look:
// https://www.winvistatips.com/threads/how-to-read-monitors-edid-information.181727/

pub use self::{
    info::Info,
    prop::{InfoPropertyValue, Property, PropertyKey, PropertyType, PropertyTypeMod},
    set::InfoSet,
};
use {crate::win32::Guid, windows::Win32::Devices::DeviceAndDriverInstallation};

mod info;
mod prop;
mod set;

/// A [display device class](DeviceAndDriverInstallation::GUID_DEVCLASS_DISPLAY)
/// to be used with [`InfoSet::new`]
pub const DEVCLASS_DISPLAY: &'static Guid = Guid::from_win32_ref(&DeviceAndDriverInstallation::GUID_DEVCLASS_DISPLAY);
/// A [monitor device class](DeviceAndDriverInstallation::GUID_DEVCLASS_MONITOR)
/// to be used with [`InfoSet::new`]
pub const DEVCLASS_MONITOR: &'static Guid = Guid::from_win32_ref(&DeviceAndDriverInstallation::GUID_DEVCLASS_MONITOR);
