#[cfg(doc)]
use windows::Win32;
use {
    crate::{
        win32::{borrow_unaligned, wide_str_from_slice_truncated},
        Output,
    },
    ddc::{Ddc, DdcHost, FeatureCode, TimingMessage, VcpValue},
    std::{
        borrow::Cow,
        cmp::Ordering,
        fmt::{self, Debug, Display, Formatter},
        hash::{Hash, Hasher},
        mem, ptr,
    },
    widestring::{WideStr, WideString},
    windows::{
        core::{Error, Result as WinResult},
        Win32::{
            Devices::Display::{
                CapabilitiesRequestAndCapabilitiesReply, DestroyPhysicalMonitor, GetCapabilitiesStringLength,
                GetTimingReport, GetVCPFeatureAndVCPFeatureReply, SaveCurrentSettings, SetVCPFeature, MC_MOMENTARY,
                MC_SET_PARAMETER, MC_TIMING_REPORT, MC_VCP_CODE_TYPE, PHYSICAL_MONITOR,
            },
            Foundation::{BOOL, HANDLE},
        },
    },
};

/// A handle to an attached monitor that allows the use of DDC/CI operations.
///
/// This is a wrapper around a [`PHYSICAL_MONITOR`][physicalmonitor].
///
/// See also: [`Win32::Devices::Display::PHYSICAL_MONITOR`]
///
/// [physicalmonitor]: https://learn.microsoft.com/en-us/windows/win32/api/physicalmonitorenumerationapi/ns-physicalmonitorenumerationapi-physical_monitor
#[derive(PartialEq, Eq)]
#[repr(align(2))] // not critical, but let's hope the compiler is nice to us in practice and allows us to realign a packed struct
#[doc(alias = "PHYSICAL_MONITOR")]
pub struct Monitor {
    monitor: PHYSICAL_MONITOR,
}

impl Monitor {
    /// Enumerate all connected physical monitors.
    ///
    /// This is a convenience wrapper around [`Output::enumerate`]
    /// and [`Output::enumerate_monitors`].
    pub fn enumerate() -> WinResult<impl Iterator<Item = WinResult<Self>>> {
        Output::enumerate().map(|outputs| {
            outputs.flat_map(|output| {
                let (err, monitors) = match output.enumerate_monitors() {
                    Ok(monitors) => (None, Some(monitors)),
                    Err(e) => (Some(e), None),
                };
                err.into_iter().map(Err).chain(monitors.into_iter().flatten().map(Ok))
            })
        })
    }

    /// Physical monitor description string.
    #[doc(alias = "szPhysicalMonitorDescription")]
    pub fn description<'a>(&'a self) -> impl Display + Debug + 'a {
        self.win32_description().to_string_lossy() // TODO: wrap .display()
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl Monitor {
    /// Create a new monitor from the specified handle.
    pub unsafe fn from_win32(monitor: PHYSICAL_MONITOR) -> Self {
        Monitor { monitor }
    }

    pub const fn from_win32_ref(monitor: &PHYSICAL_MONITOR) -> &Self {
        debug_assert!(mem::size_of::<Self>() == mem::size_of::<PHYSICAL_MONITOR>());
        unsafe { mem::transmute(monitor) }
    }

    pub const fn win32_info(&self) -> &PHYSICAL_MONITOR {
        &self.monitor
    }

    /// Physical monitor winapi handle.
    #[doc(alias = "hPhysicalMonitor")]
    pub const fn win32_handle(&self) -> HANDLE {
        self.monitor.hPhysicalMonitor
    }

    #[doc(alias = "szPhysicalMonitorDescription")]
    pub fn win32_description_slice(&self) -> Cow<[u16; 128]> {
        borrow_unaligned(&self.monitor, ptr::addr_of!(self.monitor.szPhysicalMonitorDescription))
    }

    #[doc(alias = "szPhysicalMonitorDescription")]
    pub fn win32_description(&self) -> Cow<WideStr> {
        match self.win32_description_slice() {
            Cow::Borrowed(s) => Cow::Borrowed(wide_str_from_slice_truncated(s)),
            Cow::Owned(s) => Cow::Owned(WideString::from(wide_str_from_slice_truncated(&s))),
        }
    }

    /// Retrieves a monitor's horizontal and vertical synchronization frequencies.
    #[doc(alias = "GetTimingReport")]
    pub fn win32_get_timing_report(&self) -> WinResult<MC_TIMING_REPORT> {
        let mut report = Default::default();
        BOOL(unsafe { GetTimingReport(self.win32_handle(), &mut report) }).ok()?;
        Ok(report)
    }

    /// Sets the value of a Virtual Control Panel (VCP) code for a monitor.
    #[doc(alias = "SetVCPFeature")]
    pub fn win32_set_vcp_feature(&self, code: u8, value: u32) -> WinResult<()> {
        BOOL(unsafe { SetVCPFeature(self.win32_handle(), code, value) }).ok()?;
        Ok(())
    }

    /// Saves the current monitor settings to the display's nonvolatile storage.
    #[doc(alias = "SaveCurrentSettings")]
    pub fn win32_save_current_settings(&self) -> WinResult<()> {
        BOOL(unsafe { SaveCurrentSettings(self.win32_handle()) }).ok()?;
        Ok(())
    }

    /// Retrieves the current value, maximum value, and code type of a Virtual
    /// Control Panel (VCP) code for a monitor.
    ///
    /// Returns `(vcp_type, current_value, max_value)`
    #[doc(alias = "GetVCPFeatureAndVCPFeatureReply")]
    pub fn win32_get_vcp_feature_and_vcp_feature_reply(&self, code: u8) -> WinResult<(MC_VCP_CODE_TYPE, u32, u32)> {
        let mut ty = MC_VCP_CODE_TYPE::default();
        let mut current = 0;
        let mut max = 0;
        BOOL(unsafe {
            GetVCPFeatureAndVCPFeatureReply(self.win32_handle(), code, Some(&mut ty), &mut current, Some(&mut max))
        })
        .ok()?;
        Ok((ty, current, max))
    }

    /// Retrieves the length of the buffer to pass to
    /// `win32_capabilities_request_and_capabilities_reply`.
    #[doc(alias = "GetCapabilitiesStringLength")]
    pub fn win32_get_capabilities_string_length(&self) -> WinResult<u32> {
        let mut len = 0;
        BOOL(unsafe { GetCapabilitiesStringLength(self.win32_handle(), &mut len) }).ok()?;
        Ok(len)
    }

    /// Retrieves a string describing a monitor's capabilities.
    ///
    /// This string is always ASCII and includes a terminating null character.
    #[doc(alias = "CapabilitiesRequestAndCapabilitiesReply")]
    pub fn win32_capabilities_request_and_capabilities_reply(&self, string: &mut [u8]) -> WinResult<()> {
        BOOL(unsafe { CapabilitiesRequestAndCapabilitiesReply(self.win32_handle(), string) }).ok()?;
        Ok(())
    }

    #[doc(alias = "CapabilitiesRequestAndCapabilitiesReply")]
    pub fn win32_capabilities(&self) -> WinResult<Vec<u8>> {
        let mut str = vec![0u8; self.win32_get_capabilities_string_length()? as usize];
        self.win32_capabilities_request_and_capabilities_reply(&mut str)
            .map(|_| {
                let _trailing_null = str.pop();
                str
            })
    }
}

impl DdcHost for Monitor {
    type Error = Error;
}

impl Ddc for Monitor {
    fn capabilities_string(&mut self) -> Result<Vec<u8>, Self::Error> {
        self.win32_capabilities()
    }

    fn get_vcp_feature(&mut self, code: FeatureCode) -> Result<VcpValue, Self::Error> {
        self.win32_get_vcp_feature_and_vcp_feature_reply(code)
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
        self.win32_set_vcp_feature(code, value as _)
    }

    fn save_current_settings(&mut self) -> Result<(), Self::Error> {
        self.win32_save_current_settings()
    }

    fn get_timing_report(&mut self) -> Result<TimingMessage, Self::Error> {
        self.win32_get_timing_report().map(|timing| TimingMessage {
            timing_status: timing.bTimingStatusByte,
            horizontal_frequency: timing.dwHorizontalFrequencyInHZ as _,
            vertical_frequency: timing.dwVerticalFrequencyInHZ as _,
        })
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        let _ = unsafe { DestroyPhysicalMonitor(self.win32_handle()) };
    }
}

impl Debug for Monitor {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Monitor")
            .field("handle", &self.win32_handle())
            .field("description", &self.description())
            .finish()
    }
}

impl AsRef<PHYSICAL_MONITOR> for Monitor {
    fn as_ref(&self) -> &PHYSICAL_MONITOR {
        &self.monitor
    }
}

impl PartialOrd for Monitor {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        self.win32_handle().0.partial_cmp(&rhs.win32_handle().0)
    }
}

impl Ord for Monitor {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.win32_handle().0.cmp(&rhs.win32_handle().0)
    }
}

impl Hash for Monitor {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.win32_handle().0.hash(state);
        self.win32_description_slice().hash(state);
    }
}

// TODO: impl , PartialOrd, Ord, Hash
