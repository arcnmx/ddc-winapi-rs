#![deny(missing_docs)]
#![doc(html_root_url = "http://arcnmx.github.io/ddc-winapi-rs/")]

//! Implementation of DDC/CI traits on Windows.
//!
//! # Example
//!
//! ```rust,no_run
//! extern crate ddc;
//!
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

#[macro_use]
extern crate winapi;
extern crate ddc;
extern crate widestring;

use std::{io, ptr, mem, fmt};
use winapi::shared::windef::{HMONITOR, HDC, LPRECT};
use winapi::shared::minwindef::{LPARAM, BYTE, DWORD, BOOL, TRUE};
use winapi::um::winnt::HANDLE;
use widestring::WideCString;
use ddc::{Ddc, DdcHost, FeatureCode, VcpValue, TimingMessage};

// TODO: upstream this: https://github.com/retep998/winapi-rs/issues/503
#[path = "winapi/physicalmonitorenumerationapi.rs"]
mod physicalmonitorenumerationapi;
use physicalmonitorenumerationapi::*;
#[path = "winapi/lowlevelmonitorconfigurationapi.rs"]
mod lowlevelmonitorconfigurationapi;
use lowlevelmonitorconfigurationapi::*;

// TODO: good luck getting EDID: https://social.msdn.microsoft.com/Forums/vstudio/en-US/efc46c70-7479-4d59-822b-600cb4852c4b/how-to-locate-the-edid-data-folderkey-in-the-registry-which-belongs-to-a-specific-physicalmonitor?forum=wdk

/// A handle to an attached monitor that allows the use of DDC/CI operations.
pub struct Monitor {
    monitor: PHYSICAL_MONITOR,
}

impl Monitor {
    /// Create a new monitor from the specified handle.
    pub unsafe fn new(monitor: PHYSICAL_MONITOR) -> Self {
        Monitor {
            monitor: monitor,
        }
    }

    /// Enumerate all connected physical monitors.
    pub fn enumerate() -> io::Result<Vec<Self>> {
        enumerate_monitors().and_then(|mon|
            mon.into_iter().map(|mon|
                get_physical_monitors_from_hmonitor(mon).map(|mon|
                    mon.into_iter().map(|mon| unsafe { Monitor::new(mon) })
                )
            ).collect::<io::Result<Vec<_>>>()
        ).map(|v| v.into_iter().flat_map(|mon| mon).collect())
    }

    /// Physical monitor description string.
    pub fn description(&self) -> String {
        unsafe {
            WideCString::from_ptr_str(self.monitor.szPhysicalMonitorDescription.as_ptr())
                .to_string_lossy()
        }
    }

    /// Physical monitor winapi handle.
    pub fn handle(&self) -> HANDLE {
        self.monitor.hPhysicalMonitor
    }

    /// Retrieves a monitor's horizontal and vertical synchronization frequencies.
    pub fn winapi_get_timing_report(&self) -> io::Result<MC_TIMING_REPORT> {
        unsafe {
            let mut report = mem::zeroed();
            if GetTimingReport(self.handle(), &mut report) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(report)
            }
        }
    }

    /// Sets the value of a Virtual Control Panel (VCP) code for a monitor.
    pub fn winapi_set_vcp_feature(&self, code: BYTE, value: DWORD) -> io::Result<()> {
        unsafe {
            if SetVCPFeature(self.handle(), code, value) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    /// Saves the current monitor settings to the display's nonvolatile storage.
    pub fn winapi_save_current_settings(&self) -> io::Result<()> {
        unsafe {
            if SaveCurrentSettings(self.handle()) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }

    /// Retrieves the current value, maximum value, and code type of a Virtual
    /// Control Panel (VCP) code for a monitor.
    ///
    /// Returns `(vcp_type, current_value, max_value)`
    pub fn winapi_get_vcp_feature_and_vcp_feature_reply(&self, code: BYTE) -> io::Result<(MC_VCP_CODE_TYPE, DWORD, DWORD)> {
        unsafe {
            let mut ty = 0;
            let mut current = 0;
            let mut max = 0;
            if GetVCPFeatureAndVCPFeatureReply(self.handle(), code, &mut ty, &mut current, &mut max) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok((ty, current, max))
            }
        }
    }

    /// Retrieves the length of the buffer to pass to
    /// `winapi_capabilities_request_and_capabilities_reply`.
    pub fn winapi_get_capabilities_string_length(&self) -> io::Result<DWORD> {
        unsafe {
            let mut len = 0;
            if GetCapabilitiesStringLength(self.handle(), &mut len) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(len)
            }
        }
    }

    /// Retrieves a string describing a monitor's capabilities.
    ///
    /// This string is always ASCII and includes a terminating null character.
    pub fn winapi_capabilities_request_and_capabilities_reply(&self, string: &mut [u8]) -> io::Result<()> {
        unsafe {
            if CapabilitiesRequestAndCapabilitiesReply(self.handle(), string.as_mut_ptr() as *mut _, string.len() as _) != TRUE {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}

impl DdcHost for Monitor {
    type Error = io::Error;
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
        self.winapi_get_timing_report()
            .map(|timing| TimingMessage {
                timing_status: timing.bTimingStatusByte,
                horizontal_frequency: timing.dwHorizontalFrequencyInHZ as _,
                vertical_frequency: timing.dwVerticalFrequencyInHZ as _,
            })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        unsafe {
            DestroyPhysicalMonitor(self.handle());
        }
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
pub fn get_physical_monitors_from_hmonitor(monitor: HMONITOR) -> io::Result<Vec<PHYSICAL_MONITOR>> {
    unsafe {
        let mut len = 0;
        if GetNumberOfPhysicalMonitorsFromHMONITOR(monitor, &mut len) != TRUE {
            return Err(io::Error::last_os_error())
        }

        let mut monitors = vec![mem::zeroed::<PHYSICAL_MONITOR>(); len as usize];
        if GetPhysicalMonitorsFromHMONITOR(monitor, len, monitors.as_mut_ptr()) != TRUE {
            Err(io::Error::last_os_error())
        } else {
            Ok(monitors)
        }
    }
}

/// Enumerates all `HMONITOR`s using the `EnumDisplayMonitors` WinAPI call.
pub fn enumerate_monitors() -> io::Result<Vec<HMONITOR>> {
    unsafe extern "system" fn callback(monitor: HMONITOR, _hdc_monitor: HDC, _lprc: LPRECT, userdata: LPARAM) -> BOOL {
        let monitors: &mut Vec<HMONITOR> = mem::transmute(userdata);
        monitors.push(monitor);
        TRUE
    }

    let mut monitors = Vec::<HMONITOR>::new();
    if unsafe {
        let userdata = &mut monitors as *mut _;
        winapi::um::winuser::EnumDisplayMonitors(ptr::null_mut(), ptr::null(), Some(callback), userdata as _)
    } != TRUE {
        Err(io::Error::last_os_error())
    } else {
        Ok(monitors)
    }
}
