pub use self::{
    property::Property,
    ty::{PropertyType, PropertyTypeMod},
    value::InfoPropertyValue,
};
#[cfg(doc)]
use windows::Win32;
use {
    crate::win32::Guid,
    std::{
        cmp::Ordering,
        fmt::{self, Debug, Display, Formatter},
        hash::{Hash, Hasher},
        mem,
    },
    windows::Win32::Devices::Properties::{self, DEVPROPKEY},
};

pub(crate) mod property;
pub(crate) mod ty;
pub(crate) mod value;

/// Represents a device property key for a device property in the
/// [unified device property model][unified]
///
/// Wraps a [`DEVPROPKEY`][devpropkey].
///
/// See also: [`Win32::Devices::Properties::DEVPROPKEY`]
///
/// [unified]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/unified-device-property-model--windows-vista-and-later-
/// [devpropkey]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/devpropkey
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(transparent)]
#[doc(alias = "DEVPROPKEY")]
pub struct PropertyKey {
    info: DEVPROPKEY,
}

impl PropertyKey {
    /// Specifies a property category
    pub const fn format(&self) -> &Guid {
        Guid::from_win32_ref(&self.info.fmtid)
    }

    /// Uniquely identifies the property within the property category
    ///
    /// For internal system reasons, a property identifier must be greater than or equal to two.
    pub const fn id(&self) -> u32 {
        self.info.pid
    }
}

#[allow(missing_docs)]
#[cfg_attr(feature = "doc", doc(cfg(feature = "win32")))]
#[cfg_attr(not(feature = "win32"), doc(hidden))]
impl PropertyKey {
    pub const fn win32_info(&self) -> &DEVPROPKEY {
        &self.info
    }

    pub const fn into_win32(&self) -> DEVPROPKEY {
        self.info
    }

    pub const fn from_win32_ref(info: &DEVPROPKEY) -> &Self {
        unsafe { mem::transmute(info) }
    }

    pub const fn from_win32(info: DEVPROPKEY) -> Self {
        Self { info }
    }
}

impl Hash for PropertyKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.format(), self.id()).hash(state)
    }
}

impl PartialOrd for PropertyKey {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        (self.format(), self.id()).partial_cmp(&(rhs.format(), rhs.id()))
    }
}

impl Ord for PropertyKey {
    fn cmp(&self, rhs: &Self) -> Ordering {
        (self.format(), self.id()).cmp(&(rhs.format(), rhs.id()))
    }
}

impl Debug for PropertyKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self.name() {
            Some(name) => f.debug_tuple("PropertyKey").field(&name).finish(),
            None => f
                .debug_struct("PropertyKey")
                .field("name", &self.name())
                .field("format", self.format())
                .field("id", &self.id())
                .finish(),
        }
    }
}

impl Display for PropertyKey {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}\\{}", self.format(), self.id())
    }
}

impl AsRef<DEVPROPKEY> for PropertyKey {
    fn as_ref(&self) -> &DEVPROPKEY {
        &self.info
    }
}

impl From<PropertyKey> for DEVPROPKEY {
    fn from(info: PropertyKey) -> Self {
        info.into_win32()
    }
}

impl From<DEVPROPKEY> for PropertyKey {
    fn from(info: DEVPROPKEY) -> Self {
        Self::from_win32(info)
    }
}

macro_rules! devpkeys {
    ($(pub const $name:ident = $prop:ident;)*) => {
        impl PropertyKey {
            /// The symbol name of a [static `DEVPROPKEY`][system-devpropkeys]
            ///
            /// ```rust
            /// let name = PropertyKey::DEVICE_FRIENDLY_NAME.name();
            /// assert_eq!(name, Some("DEVPKEY_Device_FriendlyName"));
            /// ```
            ///
            /// [system-devpropkeys]: https://learn.microsoft.com/en-us/windows-hardware/drivers/install/system-defined-device-properties2
            pub fn name(&self) -> Option<&'static str> {
                mod devpkey_matches {
                    use super::Properties;
                    $(
                        pub(crate) const $name: (u128, u32) = (Properties::$prop.fmtid.to_u128(), Properties::$prop.pid);
                    )*
                }
                match (self.info.fmtid.to_u128(), self.info.pid) {
                    $(
                        devpkey_matches::$name => Some(stringify!($prop)),
                    )*
                    _ => None,
                }
            }
        }

        #[allow(missing_docs)]
        impl PropertyKey {
            $(
                pub const $name: &'static Self = Self::from_win32_ref(&Properties::$prop);
            )*
        }
    };
}

devpkeys! {
    pub const DEVICE_ADDITIONAL_SOFTWARE_REQUESTED = DEVPKEY_Device_AdditionalSoftwareRequested;
    pub const DEVICE_ADDRESS = DEVPKEY_Device_Address;
    pub const DEVICE_ASSIGNED_TO_GUEST = DEVPKEY_Device_AssignedToGuest;
    pub const DEVICE_BASE_CONTAINER_ID = DEVPKEY_Device_BaseContainerId;
    pub const DEVICE_BIOS_DEVICE_NAME = DEVPKEY_Device_BiosDeviceName;
    pub const DEVICE_BUS_NUMBER = DEVPKEY_Device_BusNumber;
    pub const DEVICE_BUS_RELATIONS = DEVPKEY_Device_BusRelations;
    pub const DEVICE_BUS_REPORTED_DEVICE_DESC = DEVPKEY_Device_BusReportedDeviceDesc;
    pub const DEVICE_BUS_TYPE_GUID = DEVPKEY_Device_BusTypeGuid;
    pub const DEVICE_CAPABILITIES = DEVPKEY_Device_Capabilities;
    pub const DEVICE_CHARACTERISTICS = DEVPKEY_Device_Characteristics;
    pub const DEVICE_CHILDREN = DEVPKEY_Device_Children;
    pub const DEVICE_CLASS = DEVPKEY_Device_Class;
    pub const DEVICE_CLASS_GUID = DEVPKEY_Device_ClassGuid;
    pub const DEVICE_COMPATIBLE_IDS = DEVPKEY_Device_CompatibleIds;
    pub const DEVICE_CONFIG_FLAGS = DEVPKEY_Device_ConfigFlags;
    pub const DEVICE_CONFIGURATION_ID = DEVPKEY_Device_ConfigurationId;
    pub const DEVICE_CONTAINER_ID = DEVPKEY_Device_ContainerId;
    pub const DEVICE_CREATOR_PROCESS_ID = DEVPKEY_Device_CreatorProcessId;
    pub const DEVICE_DHP_REBALANCE_POLICY = DEVPKEY_Device_DHP_Rebalance_Policy;
    pub const DEVICE_DEBUGGER_SAFE = DEVPKEY_Device_DebuggerSafe;
    pub const DEVICE_DEPENDENCY_DEPENDENTS = DEVPKEY_Device_DependencyDependents;
    pub const DEVICE_DEPENDENCY_PROVIDERS = DEVPKEY_Device_DependencyProviders;
    pub const DEVICE_DEV_NODE_STATUS = DEVPKEY_Device_DevNodeStatus;
    pub const DEVICE_DEV_TYPE = DEVPKEY_Device_DevType;
    pub const DEVICE_DESC = DEVPKEY_Device_DeviceDesc;
    pub const DEVICE_DRIVER = DEVPKEY_Device_Driver;
    pub const DEVICE_DRIVER_CO_INSTALLERS = DEVPKEY_Device_DriverCoInstallers;
    pub const DEVICE_DRIVER_DATE = DEVPKEY_Device_DriverDate;
    pub const DEVICE_DRIVER_DESC = DEVPKEY_Device_DriverDesc;
    pub const DEVICE_DRIVER_INF_PATH = DEVPKEY_Device_DriverInfPath;
    pub const DEVICE_DRIVER_INF_SECTION = DEVPKEY_Device_DriverInfSection;
    pub const DEVICE_DRIVER_INF_SECTION_EXT = DEVPKEY_Device_DriverInfSectionExt;
    pub const DEVICE_DRIVER_LOGO_LEVEL = DEVPKEY_Device_DriverLogoLevel;
    pub const DEVICE_DRIVER_PROBLEM_DESC = DEVPKEY_Device_DriverProblemDesc;
    pub const DEVICE_DRIVER_PROP_PAGE_PROVIDER = DEVPKEY_Device_DriverPropPageProvider;
    pub const DEVICE_DRIVER_PROVIDER = DEVPKEY_Device_DriverProvider;
    pub const DEVICE_DRIVER_RANK = DEVPKEY_Device_DriverRank;
    pub const DEVICE_DRIVER_VERSION = DEVPKEY_Device_DriverVersion;
    pub const DEVICE_EJECTION_RELATIONS = DEVPKEY_Device_EjectionRelations;
    pub const DEVICE_ENUMERATOR_NAME = DEVPKEY_Device_EnumeratorName;
    pub const DEVICE_EXCLUSIVE = DEVPKEY_Device_Exclusive;
    pub const DEVICE_EXTENDED_ADDRESS = DEVPKEY_Device_ExtendedAddress;
    pub const DEVICE_EXTENDED_CONFIGURATION_IDS = DEVPKEY_Device_ExtendedConfigurationIds;
    pub const DEVICE_FIRMWARE_DATE = DEVPKEY_Device_FirmwareDate;
    pub const DEVICE_FIRMWARE_REVISION = DEVPKEY_Device_FirmwareRevision;
    pub const DEVICE_FIRMWARE_VERSION = DEVPKEY_Device_FirmwareVersion;
    pub const DEVICE_FIRST_INSTALL_DATE = DEVPKEY_Device_FirstInstallDate;
    pub const DEVICE_FRIENDLY_NAME = DEVPKEY_Device_FriendlyName;
    pub const DEVICE_FRIENDLY_NAME_ATTRIBUTES = DEVPKEY_Device_FriendlyNameAttributes;
    pub const DEVICE_GENERIC_DRIVER_INSTALLED = DEVPKEY_Device_GenericDriverInstalled;
    pub const DEVICE_HARDWARE_IDS = DEVPKEY_Device_HardwareIds;
    pub const DEVICE_HAS_PROBLEM = DEVPKEY_Device_HasProblem;
    pub const DEVICE_IN_LOCAL_MACHINE_CONTAINER = DEVPKEY_Device_InLocalMachineContainer;
    pub const DEVICE_INSTALL_DATE = DEVPKEY_Device_InstallDate;
    pub const DEVICE_INSTALL_STATE = DEVPKEY_Device_InstallState;
    pub const DEVICE_INSTANCE_ID = DEVPKEY_Device_InstanceId;
    pub const DEVICE_IS_ASSOCIATEABLE_BY_USER_ACTION = DEVPKEY_Device_IsAssociateableByUserAction;
    pub const DEVICE_IS_PRESENT = DEVPKEY_Device_IsPresent;
    pub const DEVICE_IS_REBOOT_REQUIRED = DEVPKEY_Device_IsRebootRequired;
    pub const DEVICE_LAST_ARRIVAL_DATE = DEVPKEY_Device_LastArrivalDate;
    pub const DEVICE_LAST_REMOVAL_DATE = DEVPKEY_Device_LastRemovalDate;
    pub const DEVICE_LEGACY = DEVPKEY_Device_Legacy;
    pub const DEVICE_LEGACY_BUS_TYPE = DEVPKEY_Device_LegacyBusType;
    pub const DEVICE_LOCATION_INFO = DEVPKEY_Device_LocationInfo;
    pub const DEVICE_LOCATION_PATHS = DEVPKEY_Device_LocationPaths;
    pub const DEVICE_LOWER_FILTERS = DEVPKEY_Device_LowerFilters;
    pub const DEVICE_MANUFACTURER = DEVPKEY_Device_Manufacturer;
    pub const DEVICE_MANUFACTURER_ATTRIBUTES = DEVPKEY_Device_ManufacturerAttributes;
    pub const DEVICE_MATCHING_DEVICE_ID = DEVPKEY_Device_MatchingDeviceId;
    pub const DEVICE_MODEL = DEVPKEY_Device_Model;
    pub const DEVICE_MODEL_ID = DEVPKEY_Device_ModelId;
    pub const DEVICE_NO_CONNECT_SOUND = DEVPKEY_Device_NoConnectSound;
    pub const DEVICE_NUMA_NODE = DEVPKEY_Device_Numa_Node;
    pub const DEVICE_NUMA_PROXIMITY_DOMAIN = DEVPKEY_Device_Numa_Proximity_Domain;
    pub const DEVICE_PDONAME = DEVPKEY_Device_PDOName;
    pub const DEVICE_PARENT = DEVPKEY_Device_Parent;
    pub const DEVICE_PHYSICAL_DEVICE_LOCATION = DEVPKEY_Device_PhysicalDeviceLocation;
    pub const DEVICE_POST_INSTALL_IN_PROGRESS = DEVPKEY_Device_PostInstallInProgress;
    pub const DEVICE_POWER_DATA = DEVPKEY_Device_PowerData;
    pub const DEVICE_POWER_RELATIONS = DEVPKEY_Device_PowerRelations;
    pub const DEVICE_PRESENCE_NOT_FOR_DEVICE = DEVPKEY_Device_PresenceNotForDevice;
    pub const DEVICE_PROBLEM_CODE = DEVPKEY_Device_ProblemCode;
    pub const DEVICE_PROBLEM_STATUS = DEVPKEY_Device_ProblemStatus;
    pub const DEVICE_REMOVAL_POLICY = DEVPKEY_Device_RemovalPolicy;
    pub const DEVICE_REMOVAL_POLICY_DEFAULT = DEVPKEY_Device_RemovalPolicyDefault;
    pub const DEVICE_REMOVAL_POLICY_OVERRIDE = DEVPKEY_Device_RemovalPolicyOverride;
    pub const DEVICE_REMOVAL_RELATIONS = DEVPKEY_Device_RemovalRelations;
    pub const DEVICE_REPORTED = DEVPKEY_Device_Reported;
    pub const DEVICE_REPORTED_DEVICE_IDS_HASH = DEVPKEY_Device_ReportedDeviceIdsHash;
    pub const DEVICE_RESOURCE_PICKER_EXCEPTIONS = DEVPKEY_Device_ResourcePickerExceptions;
    pub const DEVICE_RESOURCE_PICKER_TAGS = DEVPKEY_Device_ResourcePickerTags;
    pub const DEVICE_SAFE_REMOVAL_REQUIRED = DEVPKEY_Device_SafeRemovalRequired;
    pub const DEVICE_SAFE_REMOVAL_REQUIRED_OVERRIDE = DEVPKEY_Device_SafeRemovalRequiredOverride;
    pub const DEVICE_SECURITY = DEVPKEY_Device_Security;
    pub const DEVICE_SECURITY_SDS = DEVPKEY_Device_SecuritySDS;
    pub const DEVICE_SERVICE = DEVPKEY_Device_Service;
    pub const DEVICE_SESSION_ID = DEVPKEY_Device_SessionId;
    pub const DEVICE_SHOW_IN_UNINSTALL_UI = DEVPKEY_Device_ShowInUninstallUI;
    pub const DEVICE_SIBLINGS = DEVPKEY_Device_Siblings;
    pub const DEVICE_SIGNAL_STRENGTH = DEVPKEY_Device_SignalStrength;
    pub const DEVICE_SOFT_RESTART_SUPPORTED = DEVPKEY_Device_SoftRestartSupported;
    pub const DEVICE_STACK = DEVPKEY_Device_Stack;
    pub const DEVICE_TRANSPORT_RELATIONS = DEVPKEY_Device_TransportRelations;
    pub const DEVICE_UI_NUMBER = DEVPKEY_Device_UINumber;
    pub const DEVICE_UI_NUMBER_DESC_FORMAT = DEVPKEY_Device_UINumberDescFormat;
    pub const DEVICE_UPPER_FILTERS = DEVPKEY_Device_UpperFilters;
}
