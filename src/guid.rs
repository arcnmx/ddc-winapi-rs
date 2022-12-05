use {
    std::{
        cmp::Ordering,
        fmt::{self, Debug, Display, Formatter},
        mem,
        ops::{Deref, DerefMut},
    },
    windows::core::GUID,
};

/// A wrapper around [windows::core::GUID]
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
#[doc(alias = "GUID")]
pub struct Guid {
    guid: GUID,
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl Guid {
    pub const fn win32_guid(&self) -> &GUID {
        &self.guid
    }

    pub const fn from_win32_ref(guid: &GUID) -> &Self {
        unsafe { mem::transmute(guid) }
    }

    pub const fn from_win32(guid: GUID) -> Self {
        Self { guid }
    }
}

impl Display for Guid {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let [d0, d1, d2, d3, d4, d5, d6, d7] = self.guid.data4;
        write!(
            f,
            "{{{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}}}",
            self.guid.data1, self.guid.data2, self.guid.data3, d0, d1, d2, d3, d4, d5, d6, d7
        )
    }
}

impl Debug for Guid {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_tuple("Guid").field(&format_args!("{}", self)).finish()
    }
}

impl PartialOrd for Guid {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        self.to_u128().partial_cmp(&rhs.to_u128())
    }
}

impl Ord for Guid {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.to_u128().cmp(&rhs.to_u128())
    }
}

impl Deref for Guid {
    type Target = GUID;

    fn deref(&self) -> &Self::Target {
        &self.guid
    }
}

impl DerefMut for Guid {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guid
    }
}

impl AsRef<GUID> for Guid {
    fn as_ref(&self) -> &GUID {
        &self.guid
    }
}

impl From<Guid> for GUID {
    fn from(g: Guid) -> Self {
        g.guid
    }
}

impl From<GUID> for Guid {
    fn from(g: GUID) -> Self {
        Self::from_win32(g)
    }
}
