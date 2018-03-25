#![allow(dead_code, non_snake_case, non_camel_case_types)]
// Copyright © 2015-2017 winapi-rs developers
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// All files in the project carrying such notice may not be copied, modified, or distributed
// except according to those terms.
use winapi::shared::minwindef::{BOOL, DWORD, LPDWORD};
use winapi::shared::windef::HMONITOR;
use winapi::um::winnt::{HANDLE, WCHAR};
pub type _BOOL = BOOL;
pub const PHYSICAL_MONITOR_DESCRIPTION_SIZE: usize = 128;
STRUCT!{struct PHYSICAL_MONITOR {
    hPhysicalMonitor: HANDLE,
    szPhysicalMonitorDescription: [WCHAR; PHYSICAL_MONITOR_DESCRIPTION_SIZE],
}}
pub type LPPHYSICAL_MONITOR = *mut PHYSICAL_MONITOR;
#[link(name = "dxva2")]
extern "system" {
    pub fn GetNumberOfPhysicalMonitorsFromHMONITOR(
        hMonitor: HMONITOR,
        pdwNumberOfPhysicalMonitor: LPDWORD,
    ) -> _BOOL;
    pub fn GetPhysicalMonitorsFromHMONITOR(
        hMonitor: HMONITOR,
        dwPhysicalMonitorArraySize: DWORD,
        pPhysicalMonitorArray: LPPHYSICAL_MONITOR,
    ) -> _BOOL;
    pub fn DestroyPhysicalMonitor(
        hMonitor: HANDLE,
    ) -> _BOOL;
    pub fn DestroyPhysicalMonitors(
        dwPhysicalMonitorArraySize: DWORD,
        pPhysicalMonitorArray: LPPHYSICAL_MONITOR,
    ) -> _BOOL;
}
