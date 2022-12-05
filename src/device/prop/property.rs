use {
    super::{InfoPropertyValue, PropertyKey, PropertyType, PropertyTypeMod},
    crate::win32::{transmute_slice, transmute_vec, Guid},
    std::{
        any::{Any, TypeId},
        borrow::Cow,
        fmt::{self, Debug, Display, Formatter},
        iter, mem,
        time::SystemTime,
    },
    widestring::{WideCStr, WideCString},
    windows::{
        core::HRESULT,
        Win32::Foundation::{FILETIME, NTSTATUS, SYSTEMTIME, WIN32_ERROR},
    },
};

/// [Type-tagged](PropertyTypeMod) property data
///
/// This is usually constucted via [`Info::property`](crate::DeviceInfo::property)
/// or [`Info::all_properties`](crate::DeviceInfo::all_properties).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Property<'a> {
    /// The type of the contained [`data`](Self::data)
    pub type_: PropertyTypeMod,
    /// Raw property data
    pub data: Cow<'a, [u8]>,
}

impl<'a> Property<'a> {
    /// Convert this property data to [`T`](InfoPropertyValue#foreign-impls)
    pub fn get<'s, T: InfoPropertyValue<'s>>(&'s self) -> Option<T> {
        self.assert_result(T::get(self.type_, &self.data))
    }

    /// Borrow this property data as [`T`](InfoPropertyValue#foreign-impls),
    /// beyond the lifetime of `self`
    ///
    /// Note that this will fail if [`self.data`](Self::data) is [owned](Cow::Owned)
    /// by `self`, because it will not live as long as the lifetime `'a`.
    ///
    /// It is recommended to use [`self.borrow()`](Self::borrow) instead, or
    /// [`self.get::<&T>()`](Self::get) if the lifetime of `'a` is unimportant.
    pub fn get_ref<T: InfoPropertyValue<'a>>(&self) -> Option<T> {
        match self.data {
            Cow::Borrowed(data) => self.assert_result(T::get(self.type_, data)),
            Cow::Owned(..) => None,
        }
    }

    /// Try to [borrow](Self::get_ref) property data as [`T`](InfoPropertyValue#foreign-impls),
    /// returning it as [an owned value](Cow::Owned) if that would fail
    pub fn borrow<T: ?Sized + ToOwned>(&self) -> Option<Cow<'a, T>>
    where
        for<'v> &'v T: InfoPropertyValue<'v>,
    {
        match &self.data {
            Cow::Borrowed(data) => self
                .assert_result(InfoPropertyValue::get(self.type_, &data))
                .map(Cow::Borrowed),
            Cow::Owned(data) => self
                .assert_result(<&T>::get(self.type_, data))
                .map(|v| Cow::Owned(v.to_owned())),
        }
    }

    /// Iterate over all contained values of a
    /// [sequence-typed](PropertyTypeMod::is_sequence) property
    ///
    /// This will return `None` if the data is a [plain value](PropertyTypeMod::Plain).
    pub fn values<'s>(&'s self) -> Option<impl Iterator<Item = Property<'s>> + 's> {
        let (ty, aiter, liter) = match self.type_ {
            PropertyTypeMod::Array(ty) => (ty, iter_array(ty, &self.data), None),
            PropertyTypeMod::List(ty) => (ty, None, iter_string_list(&self.data)),
            _ => return None,
        };

        Some(
            liter
                .into_iter()
                .flatten()
                .map(move |string| {
                    Property::new(ty.as_plain(), match string {
                        Ok(Cow::Borrowed(data)) => unsafe { transmute_slice(data.as_slice_with_nul()) },
                        Ok(Cow::Owned(data)) => unsafe { transmute_vec(data.into_vec_with_nul()) },
                        Err(..) => Cow::Borrowed(&[][..]),
                    })
                })
                .chain(aiter.into_iter().flatten()),
        )
    }

    /// Iterate over all contained [`self.values()`](Self::values),
    /// and convert each of them to [`T`](InfoPropertyValue#foreign-impls)
    pub fn iter<'s, T: for<'v> InfoPropertyValue<'v>>(
        &'s self,
    ) -> Option<impl Iterator<Item = Result<T, Property<'s>>> + 's> {
        if !T::supports_type(self.type_) {
            return None
        }
        self.values().map(|values| {
            values.map(|v| match v.get::<T>() {
                Some(v) => Ok(v),
                None => Err(v),
            })
        })
    }

    /// Iterate over a [list](PropertyTypeMod::List) of [string values](PropertyType::is_string)
    pub fn strings<'s: 'a>(&'s self) -> Option<impl Iterator<Item = String> + 's> {
        self.win32_string_list()
            .map(|strings| strings.map(|s| s.to_string_lossy()))
    }

    /// Get the value of a [numeric](PropertyType::is_int) property
    ///
    /// The range of this value is [i64::MIN]..=[u64::MAX].
    /// [Booleans](PropertyType::Boolean) are converted to either `0` or `1`.
    /// Floating point values are not supported, use [`self.to_f64()`] for those instead.
    pub fn to_i128(&self) -> Option<i128> {
        match self.type_.base_type() {
            PropertyType::Boolean => self.get::<bool>().map(Into::into),
            PropertyType::Byte => self.get::<u8>().map(Into::into),
            PropertyType::SByte => self.get::<i8>().map(Into::into),
            PropertyType::UInt16 => self.get::<u16>().map(Into::into),
            PropertyType::Int16 => self.get::<i16>().map(Into::into),
            PropertyType::UInt32 => self.get::<u32>().map(Into::into),
            PropertyType::Int32 => self.get::<i32>().map(Into::into),
            PropertyType::UInt64 => self.get::<u64>().map(Into::into),
            PropertyType::Int64 | PropertyType::Decimal | PropertyType::Currency => self.get::<i64>().map(Into::into),
            _ => None,
        }
    }

    /// Get the value of a [floating-point](PropertyType::is_float) property
    pub fn to_f64(&self) -> Option<f64> {
        match self.type_.base_type() {
            PropertyType::Float => self.get::<f32>().map(Into::into),
            PropertyType::Double | PropertyType::Date => self.get::<f64>(),
            _ => None,
        }
    }

    /// An accessor to [the underlying data](self.data)
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Manually construct a property value from a type and bytes
    ///
    /// `data` is usually a `&[u8]` or [`Vec<u8>`](Vec).
    pub fn new<D: Into<Cow<'a, [u8]>>>(type_: PropertyTypeMod, data: D) -> Self {
        Self {
            type_,
            data: data.into(),
        }
    }

    fn with_value<R, F: FnOnce(&dyn Debug, Result<&dyn Display, &dyn Any>) -> R>(&self, f: F) -> Option<R> {
        match self.type_ {
            PropertyTypeMod::Plain(ty) => match ty {
                PropertyType::Boolean => self.get::<bool>().map(|v| f(&v, Err(&v))),
                PropertyType::Float | PropertyType::Double => self.to_f64().map(|v| f(&v, Ok(&v))),
                PropertyType::FileTime | PropertyType::Date => Some({
                    if let Some(date) = self.get::<SYSTEMTIME>() {
                        f(&date, Err(&date))
                    } else if let Some(date) = self.get::<FILETIME>() {
                        f(&date, Err(&date))
                    } else if let Some(date) = self.get::<SystemTime>() {
                        f(&date, Err(&date))
                    } else if let Some(date) = self.get::<f64>() {
                        f(&date, Ok(&date))
                    } else {
                        return self.get::<i64>().map(|ts| f(&ts, Ok(&ts)))
                    }
                }),
                #[cfg(feature = "win32-extras")]
                PropertyType::Currency => self.get::<crate::win32::CY_0>().map(|v| f(&v, Err(&v))),
                PropertyType::Guid => self.get::<Guid>().map(|v| f(&v, Ok(&v))),
                PropertyType::PropertyKey => self.get::<PropertyKey>().map(|v| f(&v, Ok(&v))),
                PropertyType::Error | PropertyType::NtStatus => self.get::<HRESULT>().map(|v| f(&v, Ok(&v))),
                PropertyType::String | PropertyType::StringIndirect | PropertyType::SecurityDescriptorString =>
                    self.borrow::<WideCStr>().map(|v| f(&v, Ok(&v.display()))),
                _ => self.to_i128().map(|v| f(&v, Ok(&v))),
            },
            _ => None,
        }
    }

    #[inline(always)]
    fn assert_result<'v, T: InfoPropertyValue<'v>>(&self, res: Option<T>) -> Option<T> {
        debug_assert!(res.is_none() || T::supports_type(self.type_));
        res
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl<'a> Property<'a> {
    pub fn win32_string_list<'s: 'a>(&'s self) -> Option<impl Iterator<Item = Cow<'a, WideCStr>> + 's> {
        self.get::<Vec<Cow<'s, WideCStr>>>().map(|v| v.into_iter())
    }
}

impl<'a> Debug for Property<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("Property");
        let dbg = debug.field("type_", &self.type_);
        let res = self
            .with_value(move |value, _| drop(dbg.field("data", value)))
            .or_else(|| match self.type_ {
                PropertyTypeMod::Array(_) | PropertyTypeMod::List(_) => self
                    .values()
                    .map(|values| drop(debug.field("data", &values.collect::<Vec<_>>()))),
                _ => None,
            });
        match res {
            Some(()) => &mut debug,
            None => debug.field("data", &self.data),
        }
        .finish()
    }
}

impl<'a> Display for Property<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let res = self
            .with_value(|debug, disp| match disp {
                Ok(disp) => Some(write!(f, "{disp}")),
                Err(any) =>
                    if any.type_id() == TypeId::of::<bool>() {
                        Some(write!(f, "{debug:?}"))
                    } else if let Some(time) = any.downcast_ref::<SYSTEMTIME>() {
                        Some(write!(
                            f,
                            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:02}",
                            time.wYear,
                            time.wMonth,
                            time.wDay,
                            time.wHour,
                            time.wMinute,
                            time.wSecond,
                            time.wMilliseconds
                        ))
                    } else if let Some(time) = any.downcast_ref::<FILETIME>() {
                        // TODO: format this literally any other way
                        Some(write!(f, "{:08x}{:08x}", time.dwHighDateTime, time.dwLowDateTime))
                    } else {
                        #[cfg(feature = "win32-extras")]
                        if let Some(cy) = any.downcast_ref::<crate::win32::CY_0>() {
                            return Some(write!(f, "{}.{}", cy.Hi, cy.Lo))
                        }
                        None
                    }, // TODO: if let Some(time) = any.downcast_ref::<SystemTime>()
            })
            .flatten();

        let res = match (res, self.type_) {
            (Some(res), _) => Some(res),
            (None, PropertyTypeMod::Array(_)) => self.values().map(|values| {
                f.write_str("[")?;
                for (i, v) in values.enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{v}")?;
                }
                f.write_str("]")
            }),
            (None, PropertyTypeMod::List(_)) => self.win32_string_list().map(|strings| {
                f.write_str("[")?;
                for (i, s) in strings.enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{}", s.display())?;
                }
                f.write_str("]")
            }),
            _ => None,
        };

        res.unwrap_or_else(|| {
            write!(f, "<{}:", self.type_)?;
            self.data.iter().try_for_each(|v| write!(f, "{:02x}", v))?;
            f.write_str(">")
        })
    }
}

impl PropertyType {
    /// The byte size of types that represent plain old data such as a struct or primitive
    ///
    /// Variable-length string types will naturally return `None`.
    pub const fn static_size(&self) -> Option<usize> {
        Some(match self {
            Self::Boolean | Self::Byte | Self::SByte => mem::size_of::<u8>(),
            Self::Int16 | Self::UInt16 => mem::size_of::<u16>(),
            Self::Int32 | Self::UInt32 => mem::size_of::<u32>(),
            Self::Int64 | Self::UInt64 | Self::FileTime | Self::Decimal | Self::Currency => mem::size_of::<u64>(),
            Self::Double | Self::Date => mem::size_of::<f64>(),
            Self::Float => mem::size_of::<f32>(),
            Self::PropertyKey => mem::size_of::<PropertyKey>(),
            Self::PropertyType => mem::size_of::<PropertyType>(),
            Self::Error => mem::size_of::<WIN32_ERROR>(),
            Self::NtStatus => mem::size_of::<NTSTATUS>(),
            Self::Guid => mem::size_of::<Guid>(),
            Self::SecurityDescriptor => {
                const SECURITY_DESCRIPTOR_SIZE: usize = {
                    let (align4, ptr) = match () {
                        #[cfg(target_pointer_width = "32")]
                        () => (0, 4),
                        #[cfg(target_pointer_width = "64")]
                        () => (4, 8),
                        #[cfg(not(any(target_pointer_width = "32", target_pointer_width = "64")))]
                        _ => compile_error!("unsupported platform"),
                    };
                    let sz = 2 + 2 + align4 + ptr * 4;
                    #[cfg(feature = "win32-extras")]
                    if sz != mem::size_of::<crate::win32::SECURITY_DESCRIPTOR>() {
                        panic!("SECURITY_DESCRIPTOR size is wrong")
                    }
                    sz
                };
                SECURITY_DESCRIPTOR_SIZE
            },
            _ => return None,
        })
    }
}

pub(crate) fn iter_array<'s>(ty: PropertyType, data: &'s [u8]) -> Option<impl Iterator<Item = Property<'s>> + 's> {
    let element_len = ty.static_size().and_then(|element_len| match data.len() % element_len {
        0 => Some(element_len),
        _ => None,
    });
    element_len.map(move |element_len| {
        data.windows(element_len)
            .map(move |data| Property::new(ty.as_plain(), Cow::Borrowed(data)))
    })
}

pub(crate) fn iter_string_list<'a>(
    data: &'a [u8],
) -> Option<impl Iterator<Item = Result<Cow<'a, WideCStr>, widestring::error::MissingNulTerminator>> + 'a> {
    <Cow<[u16]> as InfoPropertyValue>::get(PropertyTypeMod::Array(u16::TYPE.base_type()), data).map(|mut data| {
        iter::from_fn(move || {
            match WideCStr::from_slice_truncate(&data).map(|s| s.as_slice_with_nul().len()) {
                // an empty string signifies the end of the list
                Ok(1) => None,
                Ok(len) => {
                    let next = match &mut data {
                        Cow::Borrowed(ref mut data) => {
                            let (s, res) = data.split_at(len);
                            *data = res;
                            Cow::Borrowed(unsafe { WideCStr::from_slice_unchecked(s) })
                        },
                        Cow::Owned(ref mut data) => Cow::Owned(
                            WideCString::from_vec(data.drain(..len).collect::<Vec<u16>>())
                                .expect("already checked for nul above"),
                        ),
                    };
                    debug_assert_ne!(len, 0);
                    Some(Ok(next))
                },
                Err(e) => Some(Err(e)),
            }
        })
    })
}
