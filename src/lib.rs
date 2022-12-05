#![warn(missing_docs)]
#![doc(html_root_url = "https://docs.rs/ddc-winapi/0.3.0/")]
#![cfg_attr(feature = "doc", feature(doc_cfg))]

//! Implementation of DDC/CI traits on Windows.
//!
//! # Example
//!
//! ```rust,no_run
//! # fn main() -> Result<(), ddc_winapi::Error> {
//! use ddc::Ddc;
//! use ddc_winapi::Monitor;
//!
//! for mut ddc in Monitor::enumerate()? {
//!     let mccs_version = ddc.get_vcp_feature(0xdf)?;
//!     println!("MCCS version: {:04x}", mccs_version.maximum());
//! }
//! # Ok(())
//! # }
//! ```

pub use self::{
    display::{DisplayDevice, DisplayDeviceFlags, MonitorDevice},
    monitor::Monitor,
    output::{Output, OutputInfo},
};
#[doc(no_inline)]
pub use {
    self::device::{
        Info as DeviceInfo, InfoSet as DeviceInfoSet, PropertyKey as DevicePropertyKey,
        PropertyType as DevicePropertyType, PropertyTypeMod as DevicePropertyTypeMod,
    },
    windows::core::Error,
};

pub mod device;
mod display;
mod guid;
mod monitor;
mod output;
#[doc(hidden)]
pub mod registry;

#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
pub mod win32 {
    //! [`windows`] API re-exports

    #[allow(missing_docs)]
    #[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
    #[cfg_attr(not(feature = "win32"), doc(hidden))]
    #[doc(no_inline)]
    pub use windows::{
        core::{GUID, HRESULT},
        Win32::{
            Devices::{
                DeviceAndDriverInstallation::{HDEVINFO, SP_DEVINFO_DATA},
                Display::{MC_TIMING_REPORT, MC_VCP_CODE_TYPE, PHYSICAL_MONITOR},
                Properties::DEVPROPKEY,
            },
            Foundation::{DECIMAL, FILETIME, NTSTATUS, RECT, SYSTEMTIME, WIN32_ERROR},
            Graphics::Gdi::{DISPLAY_DEVICEW, HMONITOR, MONITORINFO, MONITORINFOEXW},
            System::Registry::{HKEY, REG_VALUE_TYPE},
        },
    };
    #[allow(missing_docs)]
    #[cfg_attr(feature = "doc", doc(cfg(feature = "win32-extras")))]
    #[cfg(feature = "win32-extras")]
    #[doc(no_inline)]
    pub use windows::{
        Win32::Security::SECURITY_DESCRIPTOR,
        Win32::System::Com::{CY, CY_0},
    };
    pub use {
        super::guid::Guid,
        widestring::{self, WideCStr, WideCString, WideStr, WideString},
        windows::{self, core, Win32},
    };
    use {
        std::{
            borrow::Cow,
            fmt::{Display, Write},
            mem::{align_of, size_of, ManuallyDrop},
            ptr, slice,
        },
        windows::core::{Error, Result as WinResult, HSTRING},
    };

    pub(crate) fn wide_str_from_slice_truncated(sz: &[u16]) -> &WideStr {
        match WideCStr::from_slice_truncate(&sz) {
            Ok(str) => str.as_ref(),
            Err(_) => WideStr::from_slice(&sz),
        }
    }

    pub(crate) fn win32_enum_<R, F: Fn(u32) -> WinResult<R>>(index: u32, f: F) -> WinResult<Option<R>> {
        use windows::Win32::Foundation::ERROR_NO_MORE_ITEMS;

        match f(index) {
            Ok(v) => Ok(Some(v)),
            Err(e) => match e {
                e if e.code() == ERROR_NO_MORE_ITEMS.to_hresult() => Ok(None),
                err => Err(err),
            },
        }
    }

    pub(crate) fn win32_enum<'a, R, F: Fn(u32) -> WinResult<R> + 'a>(f: F) -> impl Iterator<Item = WinResult<R>> + 'a {
        (0..)
            .map(move |i| win32_enum_(i, &f))
            .take_while(|d| match d {
                Ok(Some(..)) | Err(..) => true,
                Ok(None) => false,
            })
            .filter_map(|d| d.transpose())
    }

    pub(crate) fn win32_error(code: WIN32_ERROR, f: &dyn Display) -> Error {
        let mut str = widestring::Utf16String::new();
        let _res = write!(str, "{f}");
        debug_assert!(_res.is_ok());
        Error::new(code.to_hresult(), HSTRING::from_wide(str.as_ref()))
    }

    pub(crate) fn is_aligned<T>(ptr: *const T) -> bool {
        // TODO: Replace this with std::mem::is_aligned() once it's stable
        (ptr as usize) & (align_of::<T>() - 1) == 0
    }

    pub(crate) unsafe fn transmute_slice<'a, O: Copy, T>(data: &'a [T]) -> Cow<'a, [O]>
    where
        [O]: ToOwned<Owned = Vec<O>>,
    {
        let mut ptr = data.as_ptr() as *const O;
        let len = data.len() * size_of::<T>() / size_of::<O>();
        match is_aligned(ptr) {
            true => Cow::Borrowed(unsafe { slice::from_raw_parts(ptr, len) }),
            false => Cow::Owned({
                let mut out = Vec::with_capacity(len);
                unsafe {
                    let end = ptr.add(len);
                    while ptr < end {
                        out.push(ptr::read_unaligned(ptr));
                        ptr = ptr.add(1);
                    }
                }
                out
            }),
        }
    }

    pub(crate) unsafe fn transmute_vec<'a, O: Copy, T, D: Into<Cow<'a, [T]>>>(data: D) -> Cow<'a, [O]>
    where
        [T]: ToOwned<Owned = Vec<T>> + 'a,
        [O]: ToOwned<Owned = Vec<O>>,
    {
        match data.into() {
            Cow::Borrowed(data) => transmute_slice(data),
            Cow::Owned(data) if size_of::<T>() != size_of::<O>() || align_of::<T>() != align_of::<O>() =>
                Cow::Owned(match transmute_slice(&data[..]) {
                    Cow::Owned(data) => data,
                    Cow::Borrowed(data) => {
                        let mut out = Vec::with_capacity(data.len());
                        out.copy_from_slice(data);
                        out
                    },
                }),
            Cow::Owned(data) => Cow::Owned(unsafe {
                let data = ManuallyDrop::new(data);
                Vec::from_raw_parts(data.as_ptr() as *mut O, data.len(), data.capacity())
            }),
        }
    }

    pub(crate) fn borrow_unaligned<'a, T: Clone, D: ?Sized>(_lifetime: &'a D, ptr: *const T) -> Cow<'a, T> {
        match is_aligned(ptr) {
            true => Cow::Borrowed(unsafe { &*ptr }),
            false => Cow::Owned(unsafe { ptr::read_unaligned(ptr) }),
        }
    }
}
