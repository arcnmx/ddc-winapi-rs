use {
    super::property::iter_string_list,
    crate::{
        device::{PropertyKey, PropertyType, PropertyTypeMod},
        win32::{borrow_unaligned, is_aligned, Guid},
    },
    std::{
        borrow::Cow,
        io, mem, ptr, slice,
        time::{Duration, SystemTime},
    },
    widestring::{WideCStr, WideCString, WideStr, WideString},
    windows::{
        core::{Error, GUID, HRESULT},
        Win32::{
            Devices::Properties::DEVPROPKEY,
            Foundation::{FILETIME, NTSTATUS, SYSTEMTIME, WIN32_ERROR},
            System::Time::{FileTimeToSystemTime, SystemTimeToFileTime},
        },
    },
};

/// Represents a value that can be extracted from a [`Property`](super::Property)
///
/// This trait is responsible for the supported generic return values of
/// [`Property::get`](super::Property::get) and [`Info::get`](crate::DeviceInfo::get).
/// It generally should not need to be implemented or referenced by users of this crate.
pub trait InfoPropertyValue<'a>: Sized {
    /// The canonical [property type](PropertyType) associated with this Rust type
    const TYPE: PropertyTypeMod;

    /// Extract `Self` from a byte slice, using `type_` as a hint for the
    /// underlying data representation
    ///
    /// Properly behaving implementations shall return `None` if `type_` is
    /// [unsupported](InfoPropertyValue::supports_type) - even if `data` is
    /// otherwise valid for the type.
    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self>;

    /// Support extracting `Self` from a [modified type](PropertyTypeMod)
    ///
    /// ## Default implementation
    ///
    /// This just proxies out to [`get_plain()`](Self::get_plain) if `type_`
    /// is [plain](PropertyTypeMod::Plain).
    fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyTypeMod::Plain(ty) => InfoPropertyValue::get_plain(ty, data),
            _ => None,
        }
    }

    /// Indicate whether this type can be extracted from `type_`
    ///
    /// This hints to whether [`InfoPropertyValue::get_plain`] will succeed, assuming that
    /// the passed `data` contains correctly formatted bytes.
    ///
    /// ## Default implementation
    ///
    /// This just compares `type_` to [`TYPE`](Self::TYPE), returning `true` if they match.
    fn supports_type(type_: PropertyTypeMod) -> bool {
        type_ == Self::TYPE
    }
}

macro_rules! impl_value {
    (@primitives $($(#[$attr:meta])* $ty:ty = $pty:path, ($pat:pat => $opt:expr),)*) => {
        $(
            impl_value! { @primitive $(#[$attr])* $ty = $pty{$pty}, ($pat => $opt) }
        )*
    };
    (@primitives $($(#[$attr:meta])* $ty:ty = $pty:path{$ptypat:pat}, ($pat:pat => $opt:expr),)*) => {
        $(
            impl_value! { @primitive $(#[$attr])* $ty = $pty{$ptypat}, ($pat => $opt) }
        )*
    };
    (@primitive $(#[$attr:meta])* $ty:ty = $pty:path{$ptypat:pat}, ($pat:pat => $opt:expr)) => {
        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for $ty {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Plain($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                match type_ {
                    PropertyTypeMod::Plain($ptypat) => true,
                    _ => false,
                }
            }

            fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
                match type_ {
                    $ptypat => match data {
                        $pat => $opt,
                        #[allow(unreachable_patterns)]
                        _ => None,
                    },
                    _ => None,
                }
            }
        }

        impl_value! { @ref $ty = $pty{$ptypat}, $pat }
        impl_value! { @cow $ty = $pty{$ptypat}, $pat }
    };
    (@pods $($(#[$attr:meta])* $ty:ty = $pty:path,)*) => {
        $(
            $(#[$attr])*
            impl<'a> InfoPropertyValue<'a> for $ty {
                const TYPE: PropertyTypeMod = PropertyTypeMod::Plain($pty);

                fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
                    match type_ {
                        $pty if data.len() == mem::size_of::<$ty>() =>
                            Some(unsafe { ptr::read_unaligned(data.as_ptr() as *const $ty) }),
                        _ => None,
                    }
                }
            }

            impl_value! { @ref $(#[$attr])* $ty = $pty{$pty}, _ }
            impl_value! { @cow $(#[$attr])* $ty = $pty{$pty}, _ }
        )*
    };
    (@ref $(#[$attr:meta])* $ty:ty = $pty:path{$ptypat:pat}, $data:pat) => {
        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for &'a $ty {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Plain($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                match type_ {
                    PropertyTypeMod::Plain($ptypat) => true,
                    _ => false,
                }
            }

            fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
                match type_ {
                    $ptypat if data.len() == mem::size_of::<$ty>() && is_aligned::<$ty>(data.as_ptr() as *const $ty) => match data {
                        #[allow(unused_variables)]
                        $data => Some(unsafe { &*(data.as_ptr() as *const $ty) }),
                        #[allow(unreachable_patterns)]
                        _ => None,
                    },
                    _ => None,
                }
            }
        }

        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for &'a [$ty] {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Array($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                match type_ {
                    PropertyTypeMod::Array($ptypat) => true,
                    _ => false,
                }
            }

            fn get_plain(_: PropertyType, _: &'a [u8]) -> Option<Self> {
                None
            }

            fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
                match type_ {
                    PropertyTypeMod::Array($ptypat) if data.len() % mem::size_of::<$ty>() == 0 && is_aligned::<$ty>(data.as_ptr() as *const $ty) &&
                        data.windows(mem::size_of::<$ty>()).all(|data| match data {
                            #[allow(unused_variables)]
                            $data => true,
                            #[allow(unreachable_patterns)]
                            _ => false,
                        }) => Some(unsafe { slice::from_raw_parts(data.as_ptr() as *const $ty, data.len() / mem::size_of::<$ty>()) }),
                    _ => None,
                }
            }
        }

        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for Cow<'a, [$ty]> {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Array($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                match type_ {
                    PropertyTypeMod::Array($ptypat) => true,
                    _ => false,
                }
            }

            fn get_plain(_: PropertyType, _: &'a [u8]) -> Option<Self> {
                None
            }

            fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
                let mut ty_data = data.as_ptr() as *const $ty;
                match type_ {
                    PropertyTypeMod::Array($ptypat) if data.len() % mem::size_of::<$ty>() == 0 &&
                        data.windows(mem::size_of::<$ty>()).all(|data| match data {
                            #[allow(unused_variables)]
                            $data => true,
                            #[allow(unreachable_patterns)]
                            _ => false,
                        })
                    => Some(match is_aligned::<$ty>(ty_data) {
                        true => Cow::Borrowed(unsafe { slice::from_raw_parts(ty_data, data.len() / mem::size_of::<$ty>()) }),
                        false => Cow::Owned({
                            let mut vec = Vec::new();
                            unsafe {
                                let end = ty_data.add((data.len() / mem::size_of::<u16>()));
                                while ty_data < end {
                                    vec.push(ptr::read_unaligned(ty_data));
                                    ty_data = ty_data.add(1);
                                }
                            }
                            vec
                        }),
                    }),
                    _ => None,
                }
            }
        }

        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for Vec<$ty> {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Array($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                Cow::<[$ty]>::supports_type(type_)
            }

            fn get_plain(_: PropertyType, _: &'a [u8]) -> Option<Self> {
                None
            }

            fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
                Cow::<[$ty]>::get(type_, data).map(|v| v.into_owned())
            }
        }
    };
    (@cow $(#[$attr:meta])* $ty:ty = $pty:path{$ptypat:pat}, $data:pat) => {
        $(#[$attr])*
        impl<'a> InfoPropertyValue<'a> for Cow<'a, $ty> {
            const TYPE: PropertyTypeMod = PropertyTypeMod::Plain($pty);

            fn supports_type(type_: PropertyTypeMod) -> bool {
                match type_ {
                    PropertyTypeMod::Plain($ptypat) => true,
                    _ => false,
                }
            }

            fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
                match type_ {
                    $ptypat if data.len() == mem::size_of::<$ty>() =>
                        Some(borrow_unaligned(data, data.as_ptr() as *const $ty)),
                    _ => None,
                }
            }
        }
    };
}

impl_value! {
    @primitives
        bool = PropertyType::Boolean, (&[b] => Some(match b {
            0 => false,
            _ => true,
        })),
        u8 = PropertyType::Byte, (&[b] => Some(b)),
        i8 = PropertyType::SByte, (&[b] => Some(i8::from_ne_bytes([b]))),
        u16 = PropertyType::UInt16, (&[b0, b1] => Some(u16::from_ne_bytes([b0, b1]))),
        i16 = PropertyType::Int16, (&[b0, b1] => Some(i16::from_ne_bytes([b0, b1]))),
        i32 = PropertyType::Int32, (&[b0, b1, b2, b3] => Some(i32::from_ne_bytes([b0, b1, b2, b3]))),
        u32 = PropertyType::UInt32, (&[b0, b1, b2, b3] => Some(u32::from_ne_bytes([b0, b1, b2, b3]))),
        f32 = PropertyType::Float, (&[b0, b1, b2, b3] => Some(f32::from_ne_bytes([b0, b1, b2, b3]))),
}
impl_value! {
    @primitives
        u64 = PropertyType::UInt64{PropertyType::UInt64 | PropertyType::FileTime}, (&[b0, b1, b2, b3, b4, b5, b6, b7] => Some(u64::from_ne_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))),
        i64 = PropertyType::Int64{PropertyType::Int64 | PropertyType::Decimal | PropertyType::Currency}, (&[b0, b1, b2, b3, b4, b5, b6, b7] => Some(i64::from_ne_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))),
        f64 = PropertyType::Double{PropertyType::Double | PropertyType::Date}, (&[b0, b1, b2, b3, b4, b5, b6, b7] => Some(f64::from_ne_bytes([b0, b1, b2, b3, b4, b5, b6, b7]))),
}
#[cfg(target_endian = "little")]
impl_value! {
    @primitives
        PropertyType = PropertyType::PropertyType, (&[b @ PropertyType::MIN_DEVPROP_TYPE_U8..=PropertyType::MAX_DEVPROP_TYPE_U8, 0, 0, 0] =>
            PropertyType::from_win32(b as u32)
        ),
}
#[cfg(target_endian = "big")]
impl_value! {
    @primitives
        PropertyType = PropertyType::PropertyType, (&[0, 0, 0, b @ PropertyType::MIN_DEVPROP_TYPE_U8..=PropertyType::MAX_DEVPROP_TYPE_U8] =>
            PropertyType::from_win32(b as u32)
        ),
}

impl_value! {
    @pods
        GUID = PropertyType::Guid,
        Guid = PropertyType::Guid,
        DEVPROPKEY = PropertyType::PropertyKey,
        PropertyKey = PropertyType::PropertyKey,
        WIN32_ERROR = PropertyType::Error,
        NTSTATUS = PropertyType::NtStatus,
}
#[cfg(feature = "win32-extras")]
impl_value! {
    @pods
        #[cfg_attr(feature = "doc", doc(cfg(feature = "win32-extras")))]
        crate::win32::CY = PropertyType::Currency,
        #[cfg_attr(feature = "doc", doc(cfg(feature = "win32-extras")))]
        crate::win32::CY_0 = PropertyType::Currency,
        #[cfg_attr(feature = "doc", doc(cfg(feature = "win32-extras")))]
        crate::win32::SECURITY_DESCRIPTOR = PropertyType::SecurityDescriptor,
}

/// https://github.com/mdsteele/rust-msi/blob/3f4ebba42732263b5db194903e7e58d4eeb3cc87/src/internal/time.rs#L25
fn filetime_epoch() -> Option<SystemTime> {
    let res = SystemTime::UNIX_EPOCH.checked_sub(Duration::from_secs(11644473600));
    debug_assert!(res.is_some());
    res
}

#[cfg(feature = "win32-extras")]
fn variant_to_systemtime(v: f64) -> Option<SYSTEMTIME> {
    use windows::Win32::{Foundation::BOOL, System::Ole::VariantTimeToSystemTime};
    let mut out = SYSTEMTIME::default();
    match BOOL(unsafe { VariantTimeToSystemTime(v, &mut out) }).as_bool() {
        true => Some(out),
        false => None,
    }
}

fn filetime_to_systemtime(v: &FILETIME) -> Option<SYSTEMTIME> {
    let mut out = SYSTEMTIME::default();
    match unsafe { FileTimeToSystemTime(v, &mut out) }.as_bool() {
        true => Some(out),
        false => None,
    }
}

fn systemtime_to_filetime(v: &SYSTEMTIME) -> Option<FILETIME> {
    let mut out = FILETIME::default();
    match unsafe { SystemTimeToFileTime(v, &mut out) }.as_bool() {
        true => Some(out),
        false => None,
    }
}

fn filetime_to_std(v: &FILETIME) -> Option<SystemTime> {
    let _100ns = (v.dwHighDateTime as u64) << 32 | v.dwLowDateTime as u64;
    let delta = Duration::new(_100ns / 10_000_000, (_100ns % 10_000_000) as u32 * 100);
    filetime_epoch().and_then(|jan1601| jan1601.checked_add(delta))
}

impl<'a> InfoPropertyValue<'a> for SYSTEMTIME {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Date);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        match type_ {
            PropertyTypeMod::Plain(PropertyType::Date | PropertyType::FileTime) => true,
            _ => false,
        }
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match (type_, data) {
            #[cfg(feature = "win32-extras")]
            (PropertyType::Date, data) => f64::get_plain(type_, data).and_then(variant_to_systemtime),
            (PropertyType::FileTime, data) =>
                Cow::<FILETIME>::get_plain(type_, data).and_then(|v| filetime_to_systemtime(&v)),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for FILETIME {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::FileTime);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        match type_ {
            PropertyTypeMod::Plain(PropertyType::Date | PropertyType::FileTime) => true,
            _ => false,
        }
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::FileTime if data.len() == mem::size_of::<FILETIME>() =>
                Some(unsafe { ptr::read_unaligned(data.as_ptr() as *const FILETIME) }),
            PropertyType::Date => SYSTEMTIME::get_plain(type_, data).and_then(|v| systemtime_to_filetime(&v)),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for Cow<'a, FILETIME> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::FileTime);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        FILETIME::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::FileTime if data.len() == mem::size_of::<FILETIME>() =>
                Some(borrow_unaligned(data, data.as_ptr() as *const FILETIME)),
            PropertyType::Date => SYSTEMTIME::get_plain(type_, data)
                .and_then(|v| systemtime_to_filetime(&v))
                .map(Cow::Owned),
            _ => None,
        }
    }
}

impl_value! { @ref FILETIME = PropertyType::FileTime{PropertyType::FileTime}, _ }

impl<'a> InfoPropertyValue<'a> for SystemTime {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::FileTime);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        FILETIME::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::FileTime | PropertyType::Date =>
                FILETIME::get_plain(type_, data).and_then(|v| filetime_to_std(&v)),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for HRESULT {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Error);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        match type_ {
            PropertyTypeMod::Plain(PropertyType::Error | PropertyType::NtStatus) => true,
            _ => false,
        }
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match (type_, data) {
            (PropertyType::Error, data) => WIN32_ERROR::get_plain(type_, data).map(|res| res.to_hresult()),
            (PropertyType::NtStatus, data) => NTSTATUS::get_plain(type_, data).map(|res| res.to_hresult()),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for Error {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Error);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        HRESULT::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        HRESULT::get_plain(type_, data).map(Into::into)
    }
}

impl<'a> InfoPropertyValue<'a> for io::Error {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Error);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Error::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        Error::get_plain(type_, data).map(Into::into)
    }
}

impl<'a> InfoPropertyValue<'a> for PropertyTypeMod {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::PropertyType);

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match (type_, data) {
            (PropertyType::PropertyType, &[b0, b1, b2, b3]) =>
                PropertyTypeMod::from_win32(u32::from_ne_bytes([b0, b1, b2, b3])),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for () {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Null);

    fn get_plain(type_: PropertyType, _: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::Null => Some(()),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for &'a () {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Null);

    fn get_plain(type_: PropertyType, _: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::Null => Some(&()),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for Cow<'a, ()> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::Null);

    fn get_plain(type_: PropertyType, _: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::Null => Some(Cow::Owned(())),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for WideCString {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Cow::<WideCStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <Cow<WideCStr> as InfoPropertyValue>::get_plain(type_, data).map(|s| s.into_owned())
    }
}

impl<'a> InfoPropertyValue<'a> for &'a WideCStr {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        <&WideStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <&WideStr as InfoPropertyValue>::get_plain(type_, data).and_then(|s| WideCStr::from_slice(s.as_slice()).ok())
    }
}

impl<'a> InfoPropertyValue<'a> for Cow<'a, WideCStr> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Cow::<WideStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <Cow<WideStr> as InfoPropertyValue>::get_plain(type_, data).and_then(|s| match s {
            Cow::Borrowed(s) => WideCStr::from_slice(s.as_slice()).ok().map(Cow::Borrowed),
            Cow::Owned(s) => WideCString::from_vec(s.into_vec()).ok().map(Cow::Owned),
        })
    }
}

impl<'a> InfoPropertyValue<'a> for &'a WideStr {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        match type_ {
            PropertyTypeMod::Plain(ty) if ty.is_string() => true,
            _ => false,
        }
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::String | PropertyType::StringIndirect | PropertyType::SecurityDescriptorString =>
                <&[u16] as InfoPropertyValue>::get(PropertyTypeMod::Array(PropertyType::UInt16), data)
                    .map(|data| WideStr::from_slice(data)),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for Cow<'a, WideStr> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        <&WideStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyType::String | PropertyType::StringIndirect | PropertyType::SecurityDescriptorString =>
                <Cow<[u16]> as InfoPropertyValue>::get(PropertyTypeMod::Array(PropertyType::UInt16), data).map(|data| {
                    match data {
                        Cow::Borrowed(data) => Cow::Borrowed(WideStr::from_slice(data)),
                        Cow::Owned(data) => Cow::Owned(WideString::from_vec(data)),
                    }
                }),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for WideString {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Cow::<WideStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <Cow<WideStr> as InfoPropertyValue>::get_plain(type_, data).map(|s| s.into_owned())
    }
}

impl<'a> InfoPropertyValue<'a> for Vec<Cow<'a, WideCStr>> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::List(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        match type_ {
            PropertyTypeMod::List(ty) if ty.is_string() => true,
            _ => false,
        }
    }

    fn get_plain(_: PropertyType, _: &'a [u8]) -> Option<Self> {
        None
    }

    fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
        match type_ {
            PropertyTypeMod::List(ty) if ty.is_string() =>
                iter_string_list(data).and_then(|strings| strings.collect::<Result<_, _>>().ok()),
            _ => None,
        }
    }
}

impl<'a> InfoPropertyValue<'a> for Vec<WideCString> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::List(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Vec::<Cow<WideCStr>>::supports_type(type_)
    }

    fn get_plain(_: PropertyType, _: &'a [u8]) -> Option<Self> {
        None
    }

    fn get(type_: PropertyTypeMod, data: &'a [u8]) -> Option<Self> {
        <Vec<Cow<WideCStr>> as InfoPropertyValue>::get(type_, data)
            .map(|strings| strings.into_iter().map(|s| s.into_owned()).collect())
    }
}

impl<'a> InfoPropertyValue<'a> for String {
    const TYPE: PropertyTypeMod = PropertyTypeMod::Plain(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Cow::<WideCStr>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <Cow<WideCStr> as InfoPropertyValue>::get_plain(type_, data).map(|s| s.to_string_lossy())
    }
}

impl<'a> InfoPropertyValue<'a> for Vec<String> {
    const TYPE: PropertyTypeMod = PropertyTypeMod::List(PropertyType::String);

    fn supports_type(type_: PropertyTypeMod) -> bool {
        Vec::<Cow<WideCStr>>::supports_type(type_)
    }

    fn get_plain(type_: PropertyType, data: &'a [u8]) -> Option<Self> {
        <Vec<Cow<WideCStr>> as InfoPropertyValue>::get_plain(type_, data)
            .map(|strings| strings.into_iter().map(|s| s.to_string_lossy()).collect())
    }
}
