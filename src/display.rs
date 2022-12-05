#[cfg(doc)]
use windows::Win32;
use {
    crate::win32::wide_str_from_slice_truncated,
    std::{
        borrow::Cow,
        cmp::Ordering,
        fmt::{self, Debug, Display, Formatter},
        hash::{Hash, Hasher},
        mem,
        ops::Deref,
    },
    widestring::{widestr, WideCStr, WideStr},
    windows::{
        core::PCWSTR,
        Win32::Graphics::Gdi::{self, EnumDisplayDevicesW, DISPLAY_DEVICEW},
    },
};

/// Information representing a display or monitor device
///
/// This wraps a [`DISPLAY_DEVICE`][display_device].
///
/// See also: [`Win32::Graphics::Gdi::DISPLAY_DEVICEW`]
///
/// [display_device]: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-display_devicew
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
#[doc(alias = "DISPLAY_DEVICEW")]
#[doc(alias = "DISPLAY_DEVICE")]
pub struct DisplayDevice {
    info: DISPLAY_DEVICEW,
}

impl DisplayDevice {
    /// Enumerate all display devices
    ///
    /// This is a wrapper around [`EnumDisplayDevicesW`][enumdisplaydevicesw],
    /// with no `lpDevice` requested.
    ///
    /// [enumdisplaydevicesw]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw
    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn enumerate() -> impl Iterator<Item = Self> {
        Self::win32_enumerate().map(Self::from_win32)
    }

    /// Enumerate all [monitors](MonitorDevice) associated with this display device
    ///
    /// This is a wrapper around [`EnumDisplayDevicesW`][enumdisplaydevicesw],
    /// with [`self.name()`](Self::name) passed as `lpDevice`.
    ///
    /// [enumdisplaydevicesw]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaydevicesw
    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn enumerate_monitors<'a>(&'a self) -> impl Iterator<Item = MonitorDevice<'a>> + 'a {
        self.win32_enumerate_monitors(false)
            .map(Self::from_win32)
            .map(move |monitor| MonitorDevice::new(monitor, self))
    }

    /// [Enumerate all monitors](Self::enumerate_monitors) for every [display
    /// device](Self::enumerate)
    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn enumerate_all_monitors() -> impl Iterator<Item = MonitorDevice<'static>> {
        Self::enumerate().flat_map(|display| display.enumerate_monitors().map(|mon| mon.owned()).collect::<Vec<_>>())
    }

    /// Not used
    pub fn id<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_id().display()
    }

    /// Either the adapter or the monitor device's name
    pub fn name<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_name().display()
    }

    /// The device context string
    ///
    /// This is either a description of the display adapter or of the display monitor.
    pub fn string<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_string().display()
    }

    /// Reserved
    pub fn key<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_key().display()
    }

    /// Device state flags
    pub fn flags(&self) -> DisplayDeviceFlags {
        DisplayDeviceFlags::from_bits_truncate(self.win32_flags())
    }

    /// Whether this describes a display device or a [monitor device](MonitorDevice).
    pub fn is_monitor(&self) -> bool {
        self.win32_id().as_slice().starts_with(widestr!("MONITOR\\").as_slice())
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl DisplayDevice {
    pub const fn from_win32_ref(info: &DISPLAY_DEVICEW) -> &Self {
        unsafe { mem::transmute(info) }
    }

    pub const fn from_win32(info: DISPLAY_DEVICEW) -> Self {
        Self { info }
    }

    pub const fn into_win32(self) -> DISPLAY_DEVICEW {
        self.info
    }

    pub const fn win32_info(&self) -> &DISPLAY_DEVICEW {
        &self.info
    }

    pub fn win32_id(&self) -> &WideStr {
        wide_str_from_slice_truncated(&self.info.DeviceID)
    }

    pub fn win32_name_(&self) -> Option<&WideCStr> {
        WideCStr::from_slice_truncate(&self.info.DeviceName).ok()
    }

    pub fn win32_name(&self) -> &WideStr {
        wide_str_from_slice_truncated(&self.info.DeviceName)
    }

    pub fn win32_string(&self) -> &WideStr {
        wide_str_from_slice_truncated(&self.info.DeviceString)
    }

    pub fn win32_key(&self) -> &WideStr {
        wide_str_from_slice_truncated(&self.info.DeviceKey)
    }

    pub const fn win32_flags(&self) -> u32 {
        self.info.StateFlags
    }

    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn win32_enum(name: Option<&WideCStr>, index: u32, flags: u32) -> Option<DISPLAY_DEVICEW> {
        let mut info = DISPLAY_DEVICEW::default();
        info.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
        unsafe { EnumDisplayDevicesW(name.map(|s| PCWSTR(s.as_ptr())), index, &mut info, flags) }
            .ok()
            .map(|()| info)
            .ok()
    }

    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn win32_enumerate() -> impl Iterator<Item = DISPLAY_DEVICEW> {
        (0..)
            .map(|i| Self::win32_enum(None, i, 0))
            .take_while(|d| d.is_some())
            .filter_map(|d| d)
    }

    #[doc(alias = "EnumDisplayDevicesW")]
    pub fn win32_enumerate_monitors<'a>(&'a self, interface_name: bool) -> impl Iterator<Item = DISPLAY_DEVICEW> + 'a {
        const EDD_GET_DEVICE_INTERFACE_NAME: u32 = 1;
        let flags = match interface_name {
            true => EDD_GET_DEVICE_INTERFACE_NAME,
            false => 0,
        };
        (0..)
            .map(move |i| Self::win32_enum(Some(self.win32_name_().unwrap()), i, flags))
            .take_while(|d| d.is_some())
            .filter_map(|d| d)
    }
}

impl Debug for DisplayDevice {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("DisplayDevice")
            .field("id", &self.win32_id())
            .field("name", &self.win32_name())
            .field("string", &self.win32_string())
            .field("key", &self.win32_key())
            .field("flags", &self.flags())
            .finish()
    }
}

impl Hash for DisplayDevice {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.flags().hash(state);
        self.win32_id().hash(state);
        self.win32_name().hash(state);
        self.win32_string().hash(state);
        self.win32_key().hash(state);
    }
}

impl PartialOrd for DisplayDevice {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        let lhs = (
            self.flags(),
            self.win32_id(),
            self.win32_name(),
            self.win32_string(),
            self.win32_key(),
        );
        let rhs = (
            rhs.flags(),
            rhs.win32_id(),
            rhs.win32_name(),
            rhs.win32_string(),
            rhs.win32_key(),
        );
        lhs.partial_cmp(&rhs)
    }
}

impl Ord for DisplayDevice {
    fn cmp(&self, rhs: &Self) -> Ordering {
        let lhs = (
            self.flags(),
            self.win32_id(),
            self.win32_name(),
            self.win32_string(),
            self.win32_key(),
        );
        let rhs = (
            rhs.flags(),
            rhs.win32_id(),
            rhs.win32_name(),
            rhs.win32_string(),
            rhs.win32_key(),
        );
        lhs.cmp(&rhs)
    }
}

impl AsRef<DISPLAY_DEVICEW> for DisplayDevice {
    fn as_ref(&self) -> &DISPLAY_DEVICEW {
        &self.info
    }
}

impl From<DisplayDevice> for DISPLAY_DEVICEW {
    fn from(info: DisplayDevice) -> Self {
        info.info
    }
}

impl From<DISPLAY_DEVICEW> for DisplayDevice {
    fn from(info: DISPLAY_DEVICEW) -> Self {
        Self::from_win32(info)
    }
}

impl<'a> From<MonitorDevice<'a>> for DisplayDevice {
    fn from(info: MonitorDevice<'a>) -> Self {
        info.monitor
    }
}

impl<'a> From<DisplayDevice> for Cow<'a, DisplayDevice> {
    fn from(info: DisplayDevice) -> Self {
        Cow::Owned(info)
    }
}

impl<'a> From<&'a DisplayDevice> for Cow<'a, DisplayDevice> {
    fn from(info: &'a DisplayDevice) -> Self {
        Cow::Borrowed(info)
    }
}

/// A monitor device paired with its parent [display device](DisplayDevice)
#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash)]
#[doc(alias = "DISPLAY_DEVICEW")]
#[doc(alias = "DISPLAY_DEVICE")]
pub struct MonitorDevice<'a> {
    /// The monitor device
    pub monitor: DisplayDevice,
    /// The monitor's parent display device
    pub display: Cow<'a, DisplayDevice>,
}

impl<'a> MonitorDevice<'a> {
    /// Manually construct a new monitor device,
    /// with an associated [display device](DisplayDevice)
    pub fn new<D: Into<Cow<'a, DisplayDevice>>>(monitor: DisplayDevice, display: D) -> Self {
        Self {
            monitor,
            display: display.into(),
        }
    }

    /// A reference to the monitor's associated [display device](DisplayDevice)
    pub fn display(&self) -> &DisplayDevice {
        &self.display
    }

    /// Remove the pesky lifetime by copying the display device inline
    pub fn owned(&self) -> MonitorDevice<'static> {
        MonitorDevice {
            monitor: self.monitor.clone(),
            display: Cow::Owned(match self.display {
                Cow::Borrowed(d) => d.to_owned(),
                Cow::Owned(d) => d,
            }),
        }
    }
}

impl<'a> Deref for MonitorDevice<'a> {
    type Target = DisplayDevice;

    fn deref(&self) -> &Self::Target {
        &self.monitor
    }
}

bitflags::bitflags! {
    /// The [`StateFlags` field][stateflags] of [`DisplayDevice::flags`]
    ///
    /// [stateflags]: https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-display_devicew#members
    #[derive(Default)]
    pub struct DisplayDeviceFlags: u32 {
        /// Specifies whether a monitor is presented as being "on" by the respective GDI view
        ///
        /// *Windows Vista*: [`EnumDisplayDevices`](DisplayDevice::enumerate_monitors) will only enumerate monitors that can be presented as being "on."
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_ACTIVE`]
        #[doc(alias = "DISPLAY_DEVICE_ACTIVE")]
        const ACTIVE = Gdi::DISPLAY_DEVICE_ACTIVE;

        /// Represents a pseudo device used to mirror application drawing for remoting or other purposes
        ///
        /// An invisible pseudo monitor is associated with this device.
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_MIRRORING_DRIVER`]
        #[doc(alias = "DISPLAY_DEVICE_MIRRORING_DRIVER")]
        const MIRRORING_DRIVER = Gdi::DISPLAY_DEVICE_MIRRORING_DRIVER;

        /// The device has more display modes than its output devices support
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_MODESPRUNED`]
        #[doc(alias = "DISPLAY_DEVICE_MODESPRUNED")]
        const MODESPRUNED = Gdi::DISPLAY_DEVICE_MODESPRUNED;

        /// The primary desktop is on the device
        ///
        /// For a system with a single display card, this is always set.
        /// For a system with multiple display cards, only one device can have this set.
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_PRIMARY_DEVICE`]
        #[doc(alias = "DISPLAY_DEVICE_PRIMARY_DEVICE")]
        const PRIMARY_DEVICE = Gdi::DISPLAY_DEVICE_PRIMARY_DEVICE;

        /// The device is removable
        ///
        /// It cannot be the primary display
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_REMOVABLE`]
        #[doc(alias = "DISPLAY_DEVICE_REMOVABLE")]
        const REMOVABLE = Gdi::DISPLAY_DEVICE_REMOVABLE;

        /// The device is VGA compatible
        ///
        /// See also: [`Gdi::DISPLAY_DEVICE_VGA_COMPATIBLE`]
        #[doc(alias = "DISPLAY_DEVICE_VGA_COMPATIBLE")]
        const VGA_COMPATIBLE = Gdi::DISPLAY_DEVICE_VGA_COMPATIBLE;
    }
}
