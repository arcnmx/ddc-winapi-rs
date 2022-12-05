#[cfg(doc)]
use windows::Win32;
use {
    crate::{win32::wide_str_from_slice_truncated, DisplayDevice, Monitor},
    std::{
        fmt::{self, Debug, Display, Formatter},
        mem, ptr,
    },
    widestring::WideStr,
    windows::{
        core::Result as WinResult,
        Win32::{
            Devices::Display::{
                GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR, PHYSICAL_MONITOR,
            },
            Foundation::{BOOL, LPARAM, RECT},
            Graphics::Gdi::{EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW},
        },
    },
};

/// A handle that represents a Windows desktop display
///
/// This is a wrapper around [`HMONITOR`][hmonitor].
///
/// See also: [`Win32::Graphics::Gdi::HMONITOR`]
///
/// [hmonitor]: https://learn.microsoft.com/en-us/windows/win32/gdi/hmonitor-and-the-device-context
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
#[doc(alias = "HMONITOR")]
pub struct Output {
    handle: HMONITOR,
}

impl Output {
    /// [Enumerates display monitors][enumdisplaymonitors]
    ///
    /// [enumdisplaymonitors]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaymonitors
    #[doc(alias = "EnumDisplayMonitors")]
    pub fn enumerate() -> WinResult<impl Iterator<Item = Self>> {
        Self::win32_enumerate().map(|m| m.into_iter().map(Self::from_win32))
    }

    /// Retrieve information about a display monitor
    #[doc(alias = "GetMonitorInfoW")]
    pub fn info(&self) -> WinResult<OutputInfo> {
        self.win32_monitor_info().map(OutputInfo::from_win32)
    }

    /// Retrieves the [physical monitors][Monitor] associated with this monitor.
    #[doc(alias = "GetPhysicalMonitorsFromHMONITOR")]
    pub fn enumerate_monitors(&self) -> WinResult<impl Iterator<Item = Monitor>> {
        self.win32_physical_monitors()
            .map(|m| m.into_iter().map(|h| unsafe { Monitor::from_win32(h) }))
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl Output {
    pub const fn win32_handle(&self) -> HMONITOR {
        self.handle
    }

    pub const fn from_win32(handle: HMONITOR) -> Self {
        Self { handle }
    }

    /// Enumerates all `HMONITOR`s using the `EnumDisplayMonitors` WinAPI call.
    #[doc(alias = "EnumDisplayMonitors")]
    pub fn win32_enumerate() -> WinResult<Vec<HMONITOR>> {
        unsafe extern "system" fn callback(
            handle: HMONITOR,
            _hdc_monitor: HDC,
            _lprc: *mut RECT,
            userdata: LPARAM,
        ) -> BOOL {
            let monitors: &mut Vec<HMONITOR> = mem::transmute(userdata);
            monitors.push(handle);
            BOOL::from(true)
        }

        let mut monitors = Vec::<HMONITOR>::new();
        let userdata = LPARAM(&mut monitors as *mut _ as _);
        unsafe { EnumDisplayMonitors(None, None, Some(callback), userdata) }.ok()?;
        Ok(monitors)
    }

    #[doc(alias = "GetMonitorInfoW")]
    pub fn win32_monitor_info(&self) -> WinResult<MONITORINFOEXW> {
        let mut out = MONITORINFOEXW::default();
        out.monitorInfo.cbSize = mem::size_of::<MONITORINFOEXW>() as _;
        unsafe { GetMonitorInfoW(self.handle, ptr::addr_of_mut!(out.monitorInfo)).ok() }.map(|()| out)
    }

    #[doc(alias = "GetPhysicalMonitorsFromHMONITOR")]
    pub fn win32_physical_monitors(&self) -> WinResult<Vec<PHYSICAL_MONITOR>> {
        let mut len = 0;
        BOOL(unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(self.handle, &mut len) }).ok()?;

        let mut monitors = vec![PHYSICAL_MONITOR::default(); len as usize];
        BOOL(unsafe { GetPhysicalMonitorsFromHMONITOR(self.handle, &mut monitors) }).ok()?;

        Ok(monitors)
    }
}

impl Debug for Output {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("Output");
        debug.field("handle", &self.handle);
        if let Ok(info) = self.info() {
            debug
                .field("primary", &info.is_primary())
                .field("device", &info.win32_device_name());
        }
        debug.finish()
    }
}

impl From<Output> for HMONITOR {
    fn from(output: Output) -> Self {
        output.win32_handle()
    }
}

impl From<HMONITOR> for Output {
    fn from(handle: HMONITOR) -> Self {
        Self::from_win32(handle)
    }
}

impl AsRef<HMONITOR> for Output {
    fn as_ref(&self) -> &HMONITOR {
        &self.handle
    }
}

/// Information about an [Output]
///
/// This is a wrapper around [`MONITORINFOEX`][monitorinfoex]
///
/// See also: [`Win32::Graphics::Gdi::MONITORINFOEXW`]
///
/// [monitorinfoex]: https://learn.microsoft.com/en-us/windows/win32/api/winuser/ns-winuser-monitorinfoexw
#[derive(Copy, Clone)]
#[repr(transparent)]
#[doc(alias = "MONITORINFOEXW")]
#[doc(alias = "MONITORINFOEX")]
pub struct OutputInfo {
    info: MONITORINFOEXW,
}

impl OutputInfo {
    /// Whether this is the primary display monitor
    pub fn is_primary(&self) -> bool {
        const MONITORINFOF_PRIMARY: u32 = 1;
        self.info.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0
    }

    /// A string that specifies the device name of the monitor being used
    pub fn device_name<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_device_name().display()
    }

    /// Whether this [output](Output) is part of the specified [display device](DisplayDevice)
    ///
    /// Note that a [monitor device](crate::MonitorDevice)
    /// **should not** be passed to this function
    pub fn device_matches_display(&self, display: &DisplayDevice) -> bool {
        self.win32_device_name() == display.win32_name()
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl OutputInfo {
    pub const fn from_win32_ref(info: &MONITORINFOEXW) -> &Self {
        unsafe { mem::transmute(info) }
    }

    pub const fn from_win32(info: MONITORINFOEXW) -> Self {
        Self { info }
    }

    pub const fn win32_info(&self) -> &MONITORINFOEXW {
        &self.info
    }

    pub const fn win32_monitor_area(&self) -> RECT {
        self.info.monitorInfo.rcMonitor
    }

    pub const fn win32_work_area(&self) -> RECT {
        self.info.monitorInfo.rcWork
    }

    pub fn win32_device_name(&self) -> &WideStr {
        wide_str_from_slice_truncated(&self.info.szDevice)
    }
}

impl Debug for OutputInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("OutputInfo")
            .field("primary", &self.is_primary())
            .field("device_name", &self.win32_device_name())
            .field("monitor_area", &self.win32_monitor_area())
            .field("work_area", &self.win32_work_area())
            .finish()
    }
}

impl AsRef<MONITORINFOEXW> for OutputInfo {
    fn as_ref(&self) -> &MONITORINFOEXW {
        &self.info
    }
}

impl AsRef<MONITORINFO> for OutputInfo {
    fn as_ref(&self) -> &MONITORINFO {
        &self.info.monitorInfo
    }
}

impl From<OutputInfo> for MONITORINFOEXW {
    fn from(info: OutputInfo) -> Self {
        info.info
    }
}

impl From<MONITORINFOEXW> for OutputInfo {
    fn from(info: MONITORINFOEXW) -> Self {
        Self::from_win32(info)
    }
}
