use {
    super::{InfoPropertyValue, InfoSet, Property, PropertyKey, PropertyTypeMod},
    crate::{
        registry,
        win32::{win32_error, Guid},
        DisplayDevice,
    },
    ddc::Edid,
    std::{
        collections::HashMap,
        fmt::{self, Debug, Formatter},
    },
    widestring::{widecstr, WideCString, WideString},
    windows::{
        core::{Error, Result as WinResult},
        Win32::{
            Devices::{
                DeviceAndDriverInstallation::{
                    SetupDiGetDevicePropertyKeys, SetupDiGetDevicePropertyW, SetupDiOpenDevRegKey, HDEVINFO,
                    SP_DEVINFO_DATA,
                },
                Properties::DEVPROPKEY,
            },
            Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_DATA, ERROR_INVALID_HANDLE, ERROR_NOT_FOUND},
            System::Registry::{HKEY, KEY_READ, REG_SAM_FLAGS},
        },
    },
};

/// [SetupAPI device info][sp_devinfo_data]
///
/// This is usually constructed via [`InfoSet::enumerate`] and related methods.
///
/// This is a wrapper around [`SP_DEVINFO_DATA`](SP_DEVINFO_DATA`).
///
/// [sp_devinfo_data]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/ns-setupapi-sp_devinfo_data
#[derive(Clone, PartialEq, Eq)]
#[doc(alias = "SP_DEVINFO_DATA")]
pub struct Info<'s> {
    handle: Option<&'s InfoSet>,
    info: SP_DEVINFO_DATA,
}

impl<'s> Info<'s> {
    /// The GUID of the device's setup class.
    pub fn class(&self) -> Guid {
        Guid::from_win32(self.info.ClassGuid)
    }

    /// An opaque handle to the device instance (also known as a handle to the [devnode][devnode]).
    ///
    /// [devnode]: https://learn.microsoft.com/en-us/windows-hardware/drivers/
    pub fn instance(&self) -> u32 {
        self.info.DevInst
    }

    /// Enumerate all stored properties and their data for this device
    #[doc(alias = "SetupDiGetDevicePropertyKeys")]
    pub fn all_properties(&self) -> WinResult<HashMap<PropertyKey, Property>> {
        self.property_keys().and_then(|keys| {
            keys.map(|key| self.win32_property(key.as_ref()).map(|v| (key, v)))
                .collect()
        })
    }

    /// Enumerate the names of all stored properties for this device
    ///
    /// This is a wrapper around [`SetupDiGetDevicePropertyKeys`][wraps]
    ///
    /// [wraps]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdevicepropertykeys
    #[doc(alias = "SetupDiGetDevicePropertyKeys")]
    pub fn property_keys(&self) -> WinResult<impl Iterator<Item = PropertyKey>> {
        self.win32_property_keys()
            .map(|keys| keys.into_iter().map(PropertyKey::from_win32))
    }

    /// Retrieve a particular device property, if it exists
    ///
    /// This is a wrapper around [`SetupDiGetDevicePropertyW`][wraps]
    ///
    /// [wraps]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdevicepropertyw
    #[doc(alias = "SetupDiGetDevicePropertyW")]
    pub fn property(&self, key: &PropertyKey) -> WinResult<Option<Property>> {
        match self.win32_property(key.as_ref()) {
            Err(e) if e.code() == ERROR_NOT_FOUND.to_hresult() => Ok(None),
            res => res.map(Some),
        }
    }

    /// [Retrieve a device property](Self::property), then [convert it to `T`](InfoPropertyValue)
    ///
    /// This is a wrapper around [`SetupDiGetDevicePropertyW`][wraps]
    ///
    /// [wraps]: https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdevicepropertyw
    #[doc(alias = "SetupDiGetDevicePropertyW")]
    pub fn get<T: for<'v> InfoPropertyValue<'v>>(&self, key: &PropertyKey) -> WinResult<Option<T>> {
        self.property(key).and_then(|v| match v {
            Some(v) => match v.get() {
                Some(v) => Ok(Some(v)),
                None => Err(win32_error(
                    ERROR_INVALID_DATA,
                    &format_args!("property {:?} data did not conform to requested type {}", key, T::TYPE),
                )),
            },
            None => Ok(None),
        })
    }

    /// Whether this device info matches a [display](DisplayDevice)
    /// or [monitor device](crate::MonitorDevice)
    pub fn matches_device(&self, device: &DisplayDevice) -> WinResult<bool> {
        let (device_id, eq) = match (&self.class(), device.is_monitor()) {
            (super::DEVCLASS_MONITOR, false) => (
                self.get::<WideCString>(PropertyKey::DEVICE_PARENT)?
                    .map(|s| s.into_ustring()),
                false,
            ),
            (super::DEVCLASS_DISPLAY, false) | (super::DEVCLASS_MONITOR, true) => {
                let mut device_id = WideString::new();
                let driver = match self.get::<WideCString>(PropertyKey::DEVICE_DRIVER)? {
                    Some(d) => d,
                    None => return Ok(false),
                };
                let ids = match self.property(PropertyKey::DEVICE_HARDWARE_IDS)? {
                    Some(ids) => ids,
                    None => return Ok(false),
                };
                let ids = match ids.win32_string_list() {
                    Some(ids) => ids,
                    None => return Ok(false),
                };
                for id in ids {
                    device_id.push(id.as_ustr());
                    device_id.push_char('\\');
                }
                device_id.push(driver.as_ustr());
                (Some(device_id), true)
            },
            _ => return Ok(false),
        };

        match device_id {
            Some(id) if eq => Ok(id == device.win32_id()),
            Some(id) => Ok(id.as_slice().starts_with(device.win32_id().as_slice())),
            None => Ok(false),
        }
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl<'s> Info<'s> {
    pub fn new(handle: &'s InfoSet, info: SP_DEVINFO_DATA) -> Self {
        Self {
            handle: Some(handle),
            info,
        }
    }

    #[doc(alias = "SetupDiOpenDevRegKey")]
    pub fn open_registry_key(&self) -> WinResult<registry::Key> {
        self.win32_open_registry_key(true, 0, true, KEY_READ)
            .map(|reg| unsafe { registry::Key::from_win32(reg) })
    }

    pub fn from_win32(info: SP_DEVINFO_DATA) -> Self {
        Self { handle: None, info }
    }

    pub fn into_win32(self) -> SP_DEVINFO_DATA {
        self.info
    }

    pub fn win32_handle(&self) -> WinResult<HDEVINFO> {
        self.handle
            .map(|h| h.win32_handle())
            .ok_or_else(|| win32_error(ERROR_INVALID_HANDLE, &"attempted operation on a boneless device::Info"))
    }

    #[doc(alias = "SetupDiGetDevicePropertyW")]
    pub fn win32_property(&self, key: &DEVPROPKEY) -> WinResult<Property> {
        let handle = self.win32_handle()?;
        let mut len = 0;
        let mut prop_type = 0;
        let mut len = match unsafe {
            SetupDiGetDevicePropertyW(handle, &self.info, key, &mut prop_type, None, Some(&mut len), 0).ok()
        } {
            Ok(()) => len,
            Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => len,
            Err(e) => return Err(e),
        };
        let mut data = vec![0u8; len as usize];
        unsafe {
            SetupDiGetDevicePropertyW(
                handle,
                &self.info,
                key,
                &mut prop_type,
                Some(&mut data),
                Some(&mut len),
                0,
            )
            .ok()?;
        }
        PropertyTypeMod::try_from_win32(prop_type).map(|type_| Property::new(type_, data))
    }

    #[doc(alias = "SetupDiGetDevicePropertyKeys")]
    pub fn win32_property_keys(&self) -> WinResult<Vec<DEVPROPKEY>> {
        let handle = self.win32_handle()?;
        let mut prop_count = 0;
        let prop_count =
            match unsafe { SetupDiGetDevicePropertyKeys(handle, &self.info, None, Some(&mut prop_count), 0).ok() } {
                Ok(()) => prop_count,
                Err(e) if e.code() == ERROR_INSUFFICIENT_BUFFER.to_hresult() => prop_count,
                Err(e) => return Err(e),
            };
        let mut properties = vec![DEVPROPKEY::default(); prop_count as usize];
        unsafe {
            SetupDiGetDevicePropertyKeys(handle, &self.info, Some(&mut properties), None, 0).ok()?;
        }
        Ok(properties)
    }

    #[doc(alias = "SetupDiOpenDevRegKey")]
    pub fn win32_open_registry_key(
        &self,
        global_scope: bool,
        hardware_profile: u32,
        hardware_key: bool,
        access: REG_SAM_FLAGS,
    ) -> WinResult<HKEY> {
        use windows::Win32::Devices::DeviceAndDriverInstallation::{
            DICS_FLAG_CONFIGSPECIFIC, DICS_FLAG_GLOBAL, DIREG_DEV, DIREG_DRV,
        };
        let scope = match global_scope {
            true => DICS_FLAG_GLOBAL,
            false => DICS_FLAG_CONFIGSPECIFIC,
        };
        let key_type = match hardware_key {
            true => DIREG_DEV,
            false => DIREG_DRV,
        };
        unsafe {
            SetupDiOpenDevRegKey(
                self.win32_handle()?,
                &self.info,
                scope,
                hardware_profile,
                key_type,
                access.0,
            )
        }
    }
}

impl<'s> Edid for Info<'s> {
    type EdidError = Error;

    fn read_edid(&mut self, offset: u8, data: &mut [u8]) -> WinResult<usize> {
        let (_, edid) = self.open_registry_key()?.win32_query_value(widecstr!("EDID"))?;

        let edid = edid.get(offset as usize..).ok_or_else(|| {
            win32_error(
                ERROR_INVALID_DATA,
                &format_args!("read_edid offset={offset} out of range"),
            )
        })?;
        let len = data.len().min(edid.len());
        data[..len].copy_from_slice(&edid[..len]);
        Ok(len)
    }
}

impl<'s> Debug for Info<'s> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let mut debug = f.debug_struct("DeviceInfo");
        debug.field("class", &self.class());
        debug.field("instance", &self.instance());

        if let Ok(Some(v)) = self.get::<WideCString>(PropertyKey::DEVICE_INSTANCE_ID) {
            debug.field("instance_id", &v.to_ustring());
        }

        if let Ok(Some(v)) = self.get::<WideCString>(PropertyKey::DEVICE_FRIENDLY_NAME) {
            debug.field("friendly_name", &v.to_ustring());
        } else if let Ok(Some(v)) = self.get::<WideCString>(PropertyKey::DEVICE_DESC) {
            debug.field("device_desc", &v.to_ustring());
        }

        debug.finish()
    }
}

impl<'s> From<SP_DEVINFO_DATA> for Info<'s> {
    fn from(info: SP_DEVINFO_DATA) -> Self {
        Self::from_win32(info)
    }
}

impl<'s> From<Info<'s>> for SP_DEVINFO_DATA {
    fn from(info: Info<'s>) -> Self {
        info.into_win32()
    }
}

impl<'s> AsRef<SP_DEVINFO_DATA> for Info<'s> {
    fn as_ref(&self) -> &SP_DEVINFO_DATA {
        &self.info
    }
}
