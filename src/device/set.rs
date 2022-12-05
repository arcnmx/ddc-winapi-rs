#[cfg(doc)]
use windows::Win32;
use {
    super::Info,
    crate::win32::{win32_enum, Guid},
    std::{
        fmt::{self, Debug, Formatter},
        mem,
    },
    windows::{
        core::Result as WinResult,
        Win32::Devices::DeviceAndDriverInstallation::{
            self, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo, SetupDiGetClassDevsExW, HDEVINFO,
            SP_DEVINFO_DATA,
        },
    },
};

/// A [device information set][devinfo] collects information about device setup classes.
///
/// This is a wrapper around a
/// [`HDEVINFO`](windows::Win32::Devices::DeviceAndDriverInstallation::HDEVINFO) handle.
///
/// [devinfo]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/device-information-sets
#[derive(PartialEq, Eq)]
#[repr(transparent)]
#[doc(alias = "HDEVINFO")]
pub struct InfoSet {
    handle: HDEVINFO,
}

impl InfoSet {
    /// Create a new set representing [display devices](super::DEVCLASS_DISPLAY)
    pub fn displays() -> WinResult<Self> {
        Self::new(super::DEVCLASS_DISPLAY, InfoSetFlags::PRESENT)
    }

    /// Create a new set representing [monitor devices](super::DEVCLASS_MONITOR)
    pub fn monitors() -> WinResult<Self> {
        Self::new(super::DEVCLASS_MONITOR, InfoSetFlags::PRESENT)
    }

    /// Enumerate the contained [device information](Info)
    ///
    /// This is a wrapper around [`SetupDiEnumDeviceInfo`][setupdienumdeviceinfo].
    ///
    /// [setupdienumdeviceinfo]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdienumdeviceinfo
    #[doc(alias = "SetupDiEnumDeviceInfo")]
    pub fn enumerate<'a>(&'a self) -> impl Iterator<Item = WinResult<Info<'a>>> + 'a {
        self.win32_enumerate()
            .map(move |res| res.map(|info| Info::new(self, info)))
    }

    /// Leak this handle to produce an [iterator](Self::enumerate) with a `'static` lifetime
    #[doc(alias = "SetupDiEnumDeviceInfo")]
    pub fn enumerate_static(self) -> impl Iterator<Item = WinResult<Info<'static>>> {
        let this = Box::leak(Box::new(self));
        this.enumerate()
    }

    /// Create a new handle that contains requested device information elements
    ///
    /// This is a wrapper around [`SetupDiGetClassDevsExW`][setupdigetclassdevsexw].
    ///
    /// [setupdigetclassdevsexw]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsexw
    #[doc(alias = "SetupDiGetClassDevsExW")]
    pub fn new(class: &Guid, flags: InfoSetFlags) -> WinResult<Self> {
        unsafe {
            SetupDiGetClassDevsExW(Some(class.as_ref()), None, None, flags.bits(), None, None, None)
                .map(|handle| Self::from_win32(handle))
        }
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl InfoSet {
    pub const fn from_win32_ref(handle: &HDEVINFO) -> &Self {
        unsafe { mem::transmute(handle) }
    }

    pub unsafe fn from_win32(handle: HDEVINFO) -> Self {
        Self { handle }
    }

    pub const fn win32_handle(&self) -> HDEVINFO {
        self.handle
    }

    #[doc(alias = "SetupDiEnumDeviceInfo")]
    pub fn win32_enum_(info_set: Option<&Self>, index: u32) -> WinResult<SP_DEVINFO_DATA> {
        let mut info = SP_DEVINFO_DATA::default();
        info.cbSize = mem::size_of::<SP_DEVINFO_DATA>() as _;
        unsafe { SetupDiEnumDeviceInfo(info_set.map(|set| set.win32_handle()), index, &mut info).ok() }.map(|()| info)
    }

    #[doc(alias = "SetupDiEnumDeviceInfo")]
    pub fn win32_enum(&self, index: u32) -> WinResult<SP_DEVINFO_DATA> {
        Self::win32_enum_(Some(self), index)
    }

    #[doc(alias = "SetupDiEnumDeviceInfo")]
    pub fn win32_enumerate<'a>(&'a self) -> impl Iterator<Item = WinResult<SP_DEVINFO_DATA>> + 'a {
        win32_enum(move |i| self.win32_enum(i))
    }
}

impl Drop for InfoSet {
    #[doc(alias = "SetupDiDestroyDeviceInfoList")]
    fn drop(&mut self) {
        let _ = unsafe { SetupDiDestroyDeviceInfoList(self.win32_handle()) };
    }
}

impl Debug for InfoSet {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("InfoSet").field(&self.handle).finish()
    }
}

impl AsRef<HDEVINFO> for InfoSet {
    fn as_ref(&self) -> &HDEVINFO {
        &self.handle
    }
}

bitflags::bitflags! {
    /// Flags used to filter [a new InfoSet](InfoSet::new)
    #[derive(Default)]
    pub struct InfoSetFlags: u32 {
        /// [Installed devices for all device setup classes or all device interface classes][digcf_allclasses]
        ///
        /// [digcf_allclasses]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsa#digcf_allclasses
        #[doc(alias = "DIGCF_ALLCLASSES")]
        const ALLCLASSES = DeviceAndDriverInstallation::DIGCF_ALLCLASSES;

        /// [Only the device that is associated with the system default device interface][digcf_default]
        ///
        /// [digcf_default]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsa#digcf_default
        #[doc(alias = "DIGCF_DEFAULT")]
        const DEFAULT = DeviceAndDriverInstallation::DIGCF_DEFAULT;

        /// [Devices that support device interfaces for the specified device interface classes][digcf_deviceinterface]
        ///
        /// [digcf_deviceinterface]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsa#digcf_deviceinterface
        #[doc(alias = "DIGCF_DEVICEINTERFACE")]
        const DEVICE_INTERFACE = DeviceAndDriverInstallation::DIGCF_DEVICEINTERFACE;

        /// [Only devices that are currently present in a system][digcf_present]
        ///
        /// [digcf_present]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsa#digcf_present
        #[doc(alias = "DIGCF_PRESENT")]
        const PRESENT = DeviceAndDriverInstallation::DIGCF_PRESENT;

        /// [Only devices that are a part of the current hardware profile][digcf_profile]
        ///
        /// [digcf_profile]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetclassdevsa#digcf_profile
        #[doc(alias = "DIGCF_PROFILE")]
        const PROFILE = DeviceAndDriverInstallation::DIGCF_PROFILE;
    }
}
