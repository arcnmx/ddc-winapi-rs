#[cfg(doc)]
use windows::Win32;
use {
    crate::win32::win32_error,
    std::{
        fmt::{self, Display, Formatter},
        mem,
    },
    windows::{
        core::{Error, Result as WinResult},
        Win32::{Devices::Properties, Foundation::ERROR_INVALID_DATA},
    },
};

/// Wraps available [`DEVPROP_TYPE`][devprop_type] constants
///
/// [devprop_type]: https://learn.microsoft.com/en-us/previous-versions/ff537793(v=vs.85)
#[repr(u32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "DEVPROP_TYPE")]
pub enum PropertyType {
    /// [`DEVPROP_TYPE_BOOLEAN`][dpt] is a `bool` value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-boolean
    Boolean = Properties::DEVPROP_TYPE_BOOLEAN,
    /// [`DEVPROP_TYPE_BYTE`][dpt] is a `u8` integer
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-byte
    #[doc(alias = "DEVPROP_TYPE_BYTE")]
    Byte = Properties::DEVPROP_TYPE_BYTE,
    /// [`DEVPROP_TYPE_CURRENCY`][dpt] is a signed fixed-point `i32`.`u32` number
    #[cfg_attr(feature = "win32-extras", doc = "")]
    #[cfg_attr(feature = "win32-extras", doc = "See also: [`Win32::System::Com::CY`]")]
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-currency
    #[doc(alias = "DEVPROP_TYPE_CURRENCY")]
    Currency = Properties::DEVPROP_TYPE_CURRENCY,
    /// [`DEVPROP_TYPE_DATE`][dpt] represents the number of days since December 31, 1899 as a `f64`
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-date
    Date = Properties::DEVPROP_TYPE_DATE,
    /// [`DEVPROP_TYPE_DECIMAL`][dpt] is a [`DECIMAL`](Win32::Foundation::DECIMAL) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-decimal
    Decimal = Properties::DEVPROP_TYPE_DECIMAL,
    /// [`DEVPROP_TYPE_DEVPROPKEY`][dpt] represents a [property key](super::PropertyKey) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-devpropkey
    PropertyKey = Properties::DEVPROP_TYPE_DEVPROPKEY,
    /// [`DEVPROP_TYPE_DEVPROPTYPE`][dpt] represents a [property type](PropertyTypeMod) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-devproptype
    PropertyType = Properties::DEVPROP_TYPE_DEVPROPTYPE,
    /// [`DEVPROP_TYPE_DOUBLE`][dpt] is a `f64` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-double
    Double = Properties::DEVPROP_TYPE_DOUBLE,
    /// [`DEVPROP_TYPE_EMPTY`][dpt] indicates that a property does not exist
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-empty
    Empty = Properties::DEVPROP_TYPE_EMPTY,
    /// [`DEVPROP_TYPE_ERROR`][dpt] represents
    /// [error code values](Win32::Foundation::WIN32_ERROR)
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-error
    Error = Properties::DEVPROP_TYPE_ERROR,
    /// [`DEVPROP_TYPE_FILETIME`][dpt] contains a [`FILETIME`](Win32::Foundation::FILETIME) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-filetime
    FileTime = Properties::DEVPROP_TYPE_FILETIME,
    /// [`DEVPROP_TYPE_FLOAT`][dpt] is a `f32` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-float
    Float = Properties::DEVPROP_TYPE_FLOAT,
    /// [`DEVPROP_TYPE_GUID`][dpt] is a [`GUID`](windows::core::GUID) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-guid
    Guid = Properties::DEVPROP_TYPE_GUID,
    /// [`DEVPROP_TYPE_INT16`][dpt] is a `i16` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-int16
    Int16 = Properties::DEVPROP_TYPE_INT16,
    /// [`DEVPROP_TYPE_INT32`][dpt] is a `i32` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-int32
    Int32 = Properties::DEVPROP_TYPE_INT32,
    /// [`DEVPROP_TYPE_INT64`][dpt] is a `i64` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-int64
    Int64 = Properties::DEVPROP_TYPE_INT64,
    /// [`DEVPROP_TYPE_NTSTATUS`][dpt] is a [`NTSTATUS`](Win32::Foundation::NTSTATUS) value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-ntstatus
    NtStatus = Properties::DEVPROP_TYPE_NTSTATUS,
    /// [`DEVPROP_TYPE_NULL`][dpt] indicates that a device property exists,
    /// but has no associated value
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-null
    Null = Properties::DEVPROP_TYPE_NULL,
    /// [`DEVPROP_TYPE_SBYTE`][dpt] is a `i8` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-sbyte
    SByte = Properties::DEVPROP_TYPE_SBYTE,
    /// [`DEVPROP_TYPE_SECURITY_DESCRIPTOR`][dpt] is a [`SECURITY_DESCRIPTOR`][security_descriptor]
    /// value
    #[cfg_attr(feature = "win32-extras", doc = "")]
    #[cfg_attr(feature = "win32-extras", doc = "See also: [`Win32::Security::SECURITY_DESCRIPTOR`]")]
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-security-descriptor
    /// [security_descriptor]: https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-security_descriptor
    SecurityDescriptor = Properties::DEVPROP_TYPE_SECURITY_DESCRIPTOR,
    /// [`DEVPROP_TYPE_SECURITY_DESCRIPTOR_STRING`][dpt] is a
    /// [NULL-terminated Unicode string](widestring::WideCStr) that contains a security
    /// descriptor in the [Security Descriptor Definition Language][sddl] format
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-security-descriptor-string
    /// [sddl]: https://learn.microsoft.com/en-us/windows/win32/secauthz/security-descriptor-definition-language
    SecurityDescriptorString = Properties::DEVPROP_TYPE_SECURITY_DESCRIPTOR_STRING,
    /// [`DEVPROP_TYPE_STRING`][dpt] is a [NULL-terminated Unicode string](widestring::WideCStr)
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-string
    String = Properties::DEVPROP_TYPE_STRING,
    /// [`DEVPROP_TYPE_STRING_INDIRECT`][dpt] is a
    /// [NULL-terminated Unicode string](widestring::WideCStr) that contains
    /// an indirect string reference
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-string-indirect
    StringIndirect = Properties::DEVPROP_TYPE_STRING_INDIRECT,
    /// [`DEVPROP_TYPE_UINT16`][dpt] is a `u16` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-uint16
    UInt16 = Properties::DEVPROP_TYPE_UINT16,
    /// [`DEVPROP_TYPE_UINT32`][dpt] is a `u32` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-uint32
    UInt32 = Properties::DEVPROP_TYPE_UINT32,
    /// [`DEVPROP_TYPE_UINT64`][dpt] is a `u64` number
    ///
    /// [dpt]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-type-uint64
    UInt64 = Properties::DEVPROP_TYPE_UINT64,
}

impl PropertyType {
    /// Whether this type represents a null-terminated [`WideCStr`](widestring::WideCStr)
    pub const fn is_string(&self) -> bool {
        match self {
            Self::String | Self::StringIndirect | Self::SecurityDescriptorString => true,
            _ => false,
        }
    }

    /// Whether this type represents a numeric integer value.
    ///
    /// This indicates that [`Property::to_i128`](super::Property::to_i128)
    /// can return a value for this type.
    ///
    /// Aside from the obvious types, [`Boolean`](Self::Boolean) is included,
    /// alongside the fixed-point number types [`Decimal`](Self::Decimal) and
    /// [`Currency`](Self::Currency).
    pub const fn is_int(&self) -> bool {
        match self {
            Self::Byte
            | Self::SByte
            | Self::Int16
            | Self::UInt16
            | Self::Int32
            | Self::UInt32
            | Self::Int64
            | Self::UInt64
            | Self::Decimal
            | Self::Currency
            | Self::Boolean => true,
            _ => false,
        }
    }

    /// Whether this type represents a floating-point value.
    ///
    /// This indicates that [`Property::to_f64`](super::Property::to_f64)
    /// can return a value for this type.
    ///
    /// Aside from the obvious types, [`Date`](Self::Date) is included.
    pub const fn is_float(&self) -> bool {
        match self {
            Self::Float | Self::Double | Self::Date => true,
            _ => false,
        }
    }

    /// Constructs a [`PropertyTypeMod::Plain(self)`](PropertyTypeMod::Plain)
    pub const fn as_plain(self) -> PropertyTypeMod {
        PropertyTypeMod::Plain(self)
    }

    /// Constructs a [`PropertyTypeMod::Array(self)`](PropertyTypeMod::Array)
    pub const fn as_array(self) -> Option<PropertyTypeMod> {
        match self.static_size() {
            Some(..) => Some(PropertyTypeMod::Array(self)),
            None => None,
        }
    }

    /// Constructs a [`PropertyTypeMod::List(self)`](PropertyTypeMod::List)
    pub const fn as_list(self) -> Option<PropertyTypeMod> {
        match self.is_string() {
            true => Some(PropertyTypeMod::Array(self)),
            false => None,
        }
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl PropertyType {
    pub(crate) const MAX_DEVPROP_TYPE: u32 = Properties::MAX_DEVPROP_TYPE;
    pub(crate) const MAX_DEVPROP_TYPE_U8: u8 = Self::MAX_DEVPROP_TYPE as u8;
    pub(crate) const MIN_DEVPROP_TYPE: u32 = 0;
    pub(crate) const MIN_DEVPROP_TYPE_U8: u8 = Self::MIN_DEVPROP_TYPE as u8;

    pub fn try_from_win32(ty: u32) -> WinResult<Self> {
        Self::from_win32(ty)
            .ok_or_else(|| win32_error(ERROR_INVALID_DATA, &format_args!("DEVPROP_TYPE out of range: {ty}")))
    }

    pub const fn from_win32(ty: u32) -> Option<Self> {
        debug_assert!(PropertyType::Empty as u32 == Self::MIN_DEVPROP_TYPE);
        debug_assert!(PropertyType::StringIndirect as u32 == Self::MAX_DEVPROP_TYPE);

        match ty {
            0..=Properties::MAX_DEVPROP_TYPE => Some(unsafe { mem::transmute(ty) }),
            _ => None,
        }
    }

    pub fn win32_devprop_type(&self) -> u32 {
        *self as u32
    }
}

impl Display for PropertyType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str(match *self {
            Self::Boolean => "DEVPROP_TYPE_BOOLEAN",
            Self::Byte => "DEVPROP_TYPE_BYTE",
            Self::Currency => "DEVPROP_TYPE_CURRENCY",
            Self::Date => "DEVPROP_TYPE_DATE",
            Self::Decimal => "DEVPROP_TYPE_DECIMAL",
            Self::PropertyKey => "DEVPROP_TYPE_DEVPROPKEY",
            Self::PropertyType => "DEVPROP_TYPE_DEVPROPTYPE",
            Self::Double => "DEVPROP_TYPE_DOUBLE",
            Self::Empty => "DEVPROP_TYPE_EMPTY",
            Self::Error => "DEVPROP_TYPE_ERROR",
            Self::FileTime => "DEVPROP_TYPE_FILETIME",
            Self::Float => "DEVPROP_TYPE_FLOAT",
            Self::Guid => "DEVPROP_TYPE_GUID",
            Self::Int16 => "DEVPROP_TYPE_INT16",
            Self::Int32 => "DEVPROP_TYPE_INT32",
            Self::Int64 => "DEVPROP_TYPE_INT64",
            Self::NtStatus => "DEVPROP_TYPE_NTSTATUS",
            Self::Null => "DEVPROP_TYPE_NULL",
            Self::SByte => "DEVPROP_TYPE_SBYTE",
            Self::SecurityDescriptor => "DEVPROP_TYPE_SECURITY_DESCRIPTOR",
            Self::SecurityDescriptorString => "DEVPROP_TYPE_SECURITY_DESCRIPTOR_STRING",
            Self::String => "DEVPROP_TYPE_STRING",
            Self::StringIndirect => "DEVPROP_TYPE_STRING_INDIRECT",
            Self::UInt16 => "DEVPROP_TYPE_UINT16",
            Self::UInt32 => "DEVPROP_TYPE_UINT32",
            Self::UInt64 => "DEVPROP_TYPE_UINT64",
        })
    }
}

impl From<PropertyType> for u32 {
    fn from(ty: PropertyType) -> u32 {
        ty.win32_devprop_type()
    }
}

impl TryFrom<u32> for PropertyType {
    type Error = Error;

    fn try_from(ty: u32) -> WinResult<Self> {
        Self::try_from_win32(ty)
    }
}

/// Modifiers to a [PropertyType].
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[doc(alias = "DEVPROP_TYPEMOD")]
pub enum PropertyTypeMod {
    /// Represents a singular value
    Plain(PropertyType),
    /// An array of [statically-sized](PropertyType::static_size) value types
    ///
    /// See also: [`DEVPROP_TYPEMOD_ARRAY`](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-typemod-array)
    Array(PropertyType),
    /// A list of null-terminated strings, terminated by an empty string
    ///
    /// See also: [`DEVPROP_TYPEMOD_LIST`](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devprop-typemod-list)
    List(PropertyType),
}

impl PropertyTypeMod {
    /// The unmodified underlying type
    pub const fn base_type(&self) -> PropertyType {
        match *self {
            Self::Plain(ty) => ty,
            Self::Array(ty) => ty,
            Self::List(ty) => ty,
        }
    }

    /// Whether this type represents an [array](Self::Array) or [list](Self::List)
    pub const fn is_sequence(&self) -> bool {
        match self {
            Self::Array(..) | Self::List(..) => true,
            Self::Plain(..) => false,
        }
    }

    /// Whether this typemod is a valid match for its [base type](Self::base_type)
    pub const fn is_valid(&self) -> bool {
        match self {
            Self::Plain(..) => true,
            Self::Array(ty) => ty.static_size().is_some(),
            Self::List(ty) => ty.is_string(),
        }
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl PropertyTypeMod {
    pub fn win32_devprop_type(&self) -> u32 {
        self.base_type().win32_devprop_type()
            | match self {
                Self::Plain(_) => 0,
                Self::Array(_) => Properties::DEVPROP_TYPEMOD_ARRAY,
                Self::List(_) => Properties::DEVPROP_TYPEMOD_LIST,
            }
    }

    pub fn try_from_win32(ty: u32) -> WinResult<Self> {
        let (mod_, base) = Self::parse_win32(ty).ok_or_else(|| {
            win32_error(
                ERROR_INVALID_DATA,
                &format_args!("DEVPROP_TYPEMOD out of range: {ty:08x}"),
            )
        })?;
        let base = PropertyType::try_from_win32(base as u32)?;
        Ok(match mod_ {
            Properties::DEVPROP_TYPEMOD_LIST => Self::List(base),
            Properties::DEVPROP_TYPEMOD_ARRAY => Self::Array(base),
            _ => {
                debug_assert_eq!(mod_, 0);
                Self::Plain(base)
            },
        })
    }

    pub const fn from_win32(ty: u32) -> Option<Self> {
        let (mod_, base) = match Self::parse_win32(ty) {
            Some(v) => v,
            None => return None,
        };
        Some(match (mod_, PropertyType::from_win32(base as u32)) {
            (Properties::DEVPROP_TYPEMOD_LIST, Some(base)) => Self::List(base),
            (Properties::DEVPROP_TYPEMOD_ARRAY, Some(base)) => Self::Array(base),
            (0, Some(base)) => Self::Plain(base),
            _ => return None,
        })
    }

    const fn parse_win32(ty: u32) -> Option<(u32, u8)> {
        let base = ty as u8 & 0x1f;
        match ty & 0xffffffe0 {
            mod_ @ (Properties::DEVPROP_TYPEMOD_LIST | Properties::DEVPROP_TYPEMOD_ARRAY | 0) => Some((mod_, base)),
            _ => None,
        }
    }
}

impl Display for PropertyTypeMod {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::Plain(ty) => Display::fmt(ty, f),
            Self::Array(ty) | Self::List(ty) => {
                f.write_str("[")?;
                Display::fmt(ty, f)?;
                f.write_str("]")
            },
        }
    }
}

impl From<PropertyType> for PropertyTypeMod {
    fn from(ty: PropertyType) -> Self {
        ty.as_plain()
    }
}

impl From<PropertyTypeMod> for u32 {
    fn from(ty: PropertyTypeMod) -> Self {
        ty.win32_devprop_type()
    }
}

impl TryFrom<u32> for PropertyTypeMod {
    type Error = Error;

    fn try_from(ty: u32) -> WinResult<Self> {
        Self::try_from_win32(ty)
    }
}
