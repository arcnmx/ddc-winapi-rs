#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/ddc-winapi/0.3.0-alpha.2/")]

//! Implementation of DDC/CI traits on Windows.
//!
//! # Example
//!
//! ```rust,no_run
//! # fn main() {
//! use ddc::Ddc;
//! use ddc_winapi::Monitor;
//!
//! for mut ddc in Monitor::enumerate().unwrap() {
//!     let mccs_version = ddc.get_vcp_feature(0xdf).unwrap();
//!     println!("MCCS version: {:04x}", mccs_version.maximum());
//! }
//! # }
//! ```

pub use windows::{
    core::Error as WinError,
    Win32::{Devices::Display::PHYSICAL_MONITOR, Foundation::HANDLE},
};
use {
    ddc::{Ddc, DdcHost, FeatureCode, TimingMessage, VcpValue},
    std::{borrow::Cow, fmt, mem, ptr},
    widestring::{WideCStr, WideStr},
    windows::{
        core::Result as WinResult,
        Win32::{
            Devices::Display::{
                CapabilitiesRequestAndCapabilitiesReply, DestroyPhysicalMonitor, GetCapabilitiesStringLength,
                GetNumberOfPhysicalMonitorsFromHMONITOR, GetPhysicalMonitorsFromHMONITOR, GetTimingReport,
                GetVCPFeatureAndVCPFeatureReply, SaveCurrentSettings, SetVCPFeature, MC_MOMENTARY, MC_SET_PARAMETER,
                MC_TIMING_REPORT, MC_VCP_CODE_TYPE,
            },
            Foundation::{BOOL, LPARAM, RECT},
            Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR},
        },
    },
};

// TODO: good luck getting EDID: https://social.msdn.microsoft.com/Forums/vstudio/en-US/efc46c70-7479-4d59-822b-600cb4852c4b/how-to-locate-the-edid-data-folderkey-in-the-registry-which-belongs-to-a-specific-physicalmonitor?forum=wdk

/// A handle to an attached monitor that allows the use of DDC/CI operations.
pub struct Monitor {
    monitor: PHYSICAL_MONITOR,
}

impl Monitor {
    /// Create a new monitor from the specified handle.
    pub unsafe fn new(monitor: PHYSICAL_MONITOR) -> Self {
        Monitor { monitor }
    }

    /// Enumerate all connected physical monitors.
    pub fn enumerate() -> WinResult<Vec<Self>> {
        enumerate_monitors()
            .and_then(|mon| {
                mon.into_iter()
                    .map(|mon| {
                        get_physical_monitors_from_hmonitor(mon)
                            .map(|mon| mon.into_iter().map(|mon| unsafe { Monitor::new(mon) }))
                    })
                    .collect::<WinResult<Vec<_>>>()
            })
            .map(|v| v.into_iter().flatten().collect())
    }

    /// Physical monitor description string.
    pub fn description(&self) -> String {
        let str_ptr = ptr::addr_of!(self.monitor.szPhysicalMonitorDescription);
        // TODO: Replace with is_aligned() once it's stable
        let desc = match (str_ptr as usize) & (mem::align_of::<u16>() - 1) {
            0 => Cow::Borrowed(unsafe { &*str_ptr }),
            _ => Cow::Owned(self.monitor.szPhysicalMonitorDescription),
        };
        match WideCStr::from_slice_truncate(&desc[..]) {
            Ok(cstr) => cstr.to_string_lossy(),
            Err(_) => WideStr::from_slice(&desc[..]).to_string_lossy(),
        }
    }

    /// Physical monitor winapi handle.
    pub fn handle(&self) -> HANDLE {
        self.monitor.hPhysicalMonitor
    }

    /// Retrieves a monitor's horizontal and vertical synchronization frequencies.
    pub fn winapi_get_timing_report(&self) -> WinResult<MC_TIMING_REPORT> {
        let mut report = Default::default();
        BOOL(unsafe { GetTimingReport(self.handle(), &mut report) }).ok()?;
        Ok(report)
    }

    /// Sets the value of a Virtual Control Panel (VCP) code for a monitor.
    pub fn winapi_set_vcp_feature(&self, code: u8, value: u32) -> WinResult<()> {
        BOOL(unsafe { SetVCPFeature(self.handle(), code, value) }).ok()?;
        Ok(())
    }

    /// Saves the current monitor settings to the display's nonvolatile storage.
    pub fn winapi_save_current_settings(&self) -> WinResult<()> {
        BOOL(unsafe { SaveCurrentSettings(self.handle()) }).ok()?;
        Ok(())
    }

    /// Retrieves the current value, maximum value, and code type of a Virtual
    /// Control Panel (VCP) code for a monitor.
    ///
    /// Returns `(vcp_type, current_value, max_value)`
    pub fn winapi_get_vcp_feature_and_vcp_feature_reply(&self, code: u8) -> WinResult<(MC_VCP_CODE_TYPE, u32, u32)> {
        let mut ty = MC_VCP_CODE_TYPE::default();
        let mut current = 0;
        let mut max = 0;
        BOOL(unsafe {
            GetVCPFeatureAndVCPFeatureReply(self.handle(), code, Some(&mut ty), &mut current, Some(&mut max))
        })
        .ok()?;
        Ok((ty, current, max))
    }

    /// Retrieves the length of the buffer to pass to
    /// `winapi_capabilities_request_and_capabilities_reply`.
    pub fn winapi_get_capabilities_string_length(&self) -> WinResult<u32> {
        let mut len = 0;
        BOOL(unsafe { GetCapabilitiesStringLength(self.handle(), &mut len) }).ok()?;
        Ok(len)
    }

    /// Retrieves a string describing a monitor's capabilities.
    ///
    /// This string is always ASCII and includes a terminating null character.
    pub fn winapi_capabilities_request_and_capabilities_reply(&self, string: &mut [u8]) -> WinResult<()> {
        BOOL(unsafe { CapabilitiesRequestAndCapabilitiesReply(self.handle(), string) }).ok()?;
        Ok(())
    }
}

impl DdcHost for Monitor {
    type Error = WinError;
}

impl Ddc for Monitor {
    fn capabilities_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        let mut str = vec![0u8; self.winapi_get_capabilities_string_length()? as usize];
        self.winapi_capabilities_request_and_capabilities_reply(&mut str)
            .map(|_| {
                let len = str.len();
                if len > 0 {
                    str.truncate(len - 1); // remove trailing null byte
                }
                str
            })
    }

    fn get_vcp_feature(&mut self, code: FeatureCode) -> Result<VcpValue, Self::Error> {
        self.winapi_get_vcp_feature_and_vcp_feature_reply(code)
            .map(|(ty, cur, max)| VcpValue {
                ty: match ty {
                    MC_SET_PARAMETER => 0,
                    MC_MOMENTARY => 1,
                    _ => 0, // shouldn't be reachable?
                },
                mh: (max >> 8) as _,
                ml: max as _,
                sh: (cur >> 8) as _,
                sl: cur as _,
            })
    }

    fn set_vcp_feature(&mut self, code: FeatureCode, value: u16) -> Result<(), Self::Error> {
        self.winapi_set_vcp_feature(code, value as _)
    }

    fn save_current_settings(&mut self) -> Result<(), Self::Error> {
        self.winapi_save_current_settings()
    }

    fn get_timing_report(&mut self) -> Result<TimingMessage, Self::Error> {
        self.winapi_get_timing_report().map(|timing| TimingMessage {
            timing_status: timing.bTimingStatusByte,
            horizontal_frequency: timing.dwHorizontalFrequencyInHZ as _,
            vertical_frequency: timing.dwVerticalFrequencyInHZ as _,
        })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = unsafe { DestroyPhysicalMonitor(self.handle()) };
    }
}

impl fmt::Debug for Monitor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Monitor")
            .field("handle", &self.handle())
            .field("description", &self.description())
            .finish()
    }
}

/// WinAPI `GetPhysicalMonitorsFromHMONITOR`
pub fn get_physical_monitors_from_hmonitor(monitor: HMONITOR) -> WinResult<Vec<PHYSICAL_MONITOR>> {
    let mut len = 0;
    unsafe { GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut len) }?;

    let mut monitors = vec![PHYSICAL_MONITOR::default(); len as usize];
    unsafe { GetPhysicalMonitorsFromHMONITOR(monitor, &mut monitors) }?;

    Ok(monitors)
}

/// Enumerates all `HMONITOR`s using the `EnumDisplayMonitors` WinAPI call.
pub fn enumerate_monitors() -> WinResult<Vec<HMONITOR>> {
    unsafe extern "system" fn callback(
        monitor: HMONITOR,
        _hdc_monitor: HDC,
        _lprc: *mut RECT,
        userdata: LPARAM,
    ) -> BOOL {
        let monitors: &mut Vec<HMONITOR> = &mut *(userdata.0 as *mut Vec<HMONITOR>);
        monitors.push(monitor);
        BOOL::from(true)
    }

    let mut monitors = Vec::<HMONITOR>::new();
    let userdata = LPARAM(ptr::addr_of_mut!(monitors) as _);
    unsafe { EnumDisplayMonitors(None, None, Some(callback), userdata) }.ok()?;
    Ok(monitors)
}
