use {
    crate::win32::{win32_enum, win32_error},
    std::{
        fmt::{self, Debug, Formatter},
        mem,
    },
    widestring::{WideCStr, WideCString},
    windows::{
        core::{Result as WinResult, PCWSTR, PWSTR},
        Win32::{
            Foundation::{ERROR_INVALID_DATA, FILETIME},
            System::Registry::{
                self, RegCloseKey, RegCreateKeyW, RegEnumKeyExW, RegEnumValueW, RegOpenKeyExW, RegQueryValueExW, HKEY,
                REG_OPEN_CREATE_OPTIONS, REG_SAM_FLAGS, REG_VALUE_TYPE,
            },
        },
    },
};

#[doc(alias = "HKEY")]
#[repr(transparent)]
#[derive(PartialEq, Eq)]
pub struct Key {
    handle: HKEY,
}

impl Key {
    pub const HKEY_CLASSES_ROOT: &'static Self = Self::from_win32_ref(&Registry::HKEY_CLASSES_ROOT);
    pub const HKEY_CURRENT_CONFIG: &'static Self = Self::from_win32_ref(&Registry::HKEY_CURRENT_CONFIG);
    pub const HKEY_CURRENT_USER: &'static Self = Self::from_win32_ref(&Registry::HKEY_CURRENT_USER);
    pub const HKEY_LOCAL_MACHINE: &'static Self = Self::from_win32_ref(&Registry::HKEY_LOCAL_MACHINE);
    pub const HKEY_USERS: &'static Self = Self::from_win32_ref(&Registry::HKEY_USERS);

    pub const fn win32_handle(&self) -> HKEY {
        self.handle
    }

    pub const fn from_win32_ref(handle: &HKEY) -> &Self {
        unsafe { mem::transmute(handle) }
    }

    pub unsafe fn from_win32(handle: HKEY) -> Self {
        Self { handle }
    }

    #[doc(alias = "RegCreateKeyW")]
    pub fn win32_create(&self, key: &WideCStr) -> WinResult<Self> {
        let mut handle = HKEY::default();
        unsafe { RegCreateKeyW(self.handle, PCWSTR(key.as_ptr()), &mut handle).ok() }
            .map(|()| unsafe { Key::from_win32(handle) })
    }

    #[doc(alias = "RegOpenKeyExW")]
    pub fn win32_open(
        &self,
        key: &WideCStr,
        options: REG_OPEN_CREATE_OPTIONS,
        access: REG_SAM_FLAGS,
    ) -> WinResult<Self> {
        let mut handle = HKEY::default();
        unsafe { RegOpenKeyExW(self.handle, PCWSTR(key.as_ptr()), options.0, access, &mut handle).ok() }
            .map(|()| unsafe { Key::from_win32(handle) })
    }

    #[doc(alias = "RegQueryValueExW")]
    pub fn win32_query_value(&self, key: &WideCStr) -> WinResult<(REG_VALUE_TYPE, Vec<u8>)> {
        let mut len = 0;
        let mut ty = REG_VALUE_TYPE::default();
        unsafe {
            RegQueryValueExW(
                self.win32_handle(),
                PCWSTR(key.as_ptr()),
                None,
                Some(&mut ty),
                None,
                Some(&mut len),
            )
            .ok()
        }
        .map(|()| len)
        .and_then(|mut len| unsafe {
            let mut data = vec![0u8; len as usize];
            RegQueryValueExW(
                self.win32_handle(),
                PCWSTR(key.as_ptr()),
                None,
                Some(&mut ty),
                Some(data.as_mut_ptr()),
                Some(&mut len),
            )
            .ok()
            .map(|()| (ty, data))
        })
    }

    #[doc(alias = "RegEnumKeyW")]
    pub fn win32_enumerate_key(&self, index: u32) -> WinResult<(WideCString, Option<WideCString>, FILETIME)> {
        let mut name_len = 0;
        let mut class_len = 0;
        let mut filetime = FILETIME::default();
        unsafe {
            RegEnumKeyExW(
                self.handle,
                index,
                PWSTR::null(),
                &mut name_len,
                None,
                PWSTR::null(),
                Some(&mut class_len),
                Some(&mut filetime),
            )
            .ok()?;
        }
        let mut name = vec![0u16; name_len as usize + 1];
        let mut class = match class_len {
            0 => None,
            class_len => Some(vec![0u16; class_len as usize + 1]),
        };
        unsafe {
            RegEnumKeyExW(
                self.handle,
                index,
                PWSTR(name.as_mut_ptr()),
                &mut name_len,
                None,
                class.as_mut().map(|c| PWSTR(c.as_mut_ptr())).unwrap_or(PWSTR::null()),
                Some(&mut class_len),
                None,
            )
            .ok()
        }
        .and_then(|()| {
            WideCString::from_vec(name)
                .and_then(|name| {
                    class
                        .map(|class| WideCString::from_vec(class))
                        .transpose()
                        .map(|class| (name, class))
                })
                .map_err(|e| win32_error(ERROR_INVALID_DATA, &format_args!("{e:?}")))
        })
        .map(|(name, class)| (name, class, filetime))
    }

    #[doc(alias = "RegEnumKeyW")]
    pub fn win32_enumerate_keys<'a>(
        &'a self,
    ) -> impl Iterator<Item = WinResult<(WideCString, Option<WideCString>, FILETIME)>> + 'a {
        win32_enum(move |i| self.win32_enumerate_key(i))
    }

    #[doc(alias = "RegEnumValueW")]
    pub fn win32_enumerate_value(&self, index: u32) -> WinResult<(WideCString, REG_VALUE_TYPE, usize)> {
        let mut name_len = 0;
        let mut type_ = 0;
        let mut len = 0;
        unsafe {
            RegEnumValueW(
                self.handle,
                index,
                PWSTR::null(),
                &mut name_len,
                None,
                Some(&mut type_),
                None,
                Some(&mut len),
            )
            .ok()?;
        }
        let mut name = vec![0u16; name_len as usize + 1];
        unsafe {
            RegEnumValueW(
                self.handle,
                index,
                PWSTR(name.as_mut_ptr()),
                &mut name_len,
                None,
                None,
                None,
                None,
            )
            .ok()
        }
        .and_then(|()| WideCString::from_vec(name).map_err(|e| win32_error(ERROR_INVALID_DATA, &format_args!("{e:?}"))))
        .map(|name| (name, REG_VALUE_TYPE(type_), len as usize))
    }

    #[doc(alias = "RegEnumValueW")]
    pub fn win32_enumerate_values<'a>(
        &'a self,
    ) -> impl Iterator<Item = WinResult<(WideCString, REG_VALUE_TYPE, usize)>> + 'a {
        win32_enum(move |i| self.win32_enumerate_value(i))
    }
}

impl Drop for Key {
    #[doc(alias = "RegCloseKey")]
    fn drop(&mut self) {
        let _ = unsafe { RegCloseKey(self.win32_handle()) };
    }
}

impl Debug for Key {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut debug = f.debug_tuple("Key");
        match self.handle {
            Registry::HKEY_CLASSES_ROOT => debug.field(&"HKEY_CLASSES_ROOT"),
            Registry::HKEY_CURRENT_CONFIG => debug.field(&"HKEY_CURRENT_CONFIG"),
            Registry::HKEY_CURRENT_USER => debug.field(&"HKEY_CURRENT_USER"),
            Registry::HKEY_LOCAL_MACHINE => debug.field(&"HKEY_LOCAL_MACHINE"),
            Registry::HKEY_USERS => debug.field(&"HKEY_USERS"),
            _ => debug.field(&self.handle),
        }
        .finish()
    }
}

impl AsRef<HKEY> for Key {
    fn as_ref(&self) -> &HKEY {
        &self.handle
    }
}
