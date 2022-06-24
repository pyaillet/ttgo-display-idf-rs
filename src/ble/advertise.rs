use esp_idf_sys::*;

#[allow(clippy::upper_case_acronyms)]
#[repr(u16)]
pub enum AppearanceCategory {
    Unknown = 0x00,
    Phone,
    Computer,
    Watch,
    Clock,
    Display,
    RemoteControl,
    EyeGlass,
    Tag,
    Keyring,
    MediaPlayer,
    BarcodeScanner,
    Thermometer,
    HeartRateSensor,
    BloodPressure,
    HumanInterfaceDevice,
    GlucoseMeter,
    RunningWalkingSensor,
    Cycling,
    ControlDevice,
    NetworkDevice,
    Sensor,
    LightFixtures,
    Fan,
    HVAC,
    AirConditionning,
    Humidifier,
    Heating,
    AccessControl,
    MotorizedDevice,
    PowerDevice,
    LightSource,
    WindowCovering,
    AudioSink,
    AudioSource,
    MotorizedVehicle,
    DomesticAppliance,
    WearableAudioDevice,
    Aircraft,
    AVEquipment,
    DisplayEquipment,
    HearingAid,
    Gaming,
    Signage,
    PulseOximeter = 0x31,
    WeightScale,
    PersonalMobilityDevice,
    ContinuousGlucoseMonitor,
    InsulinPump,
    MedicationDelivery,
    OutdoorSportsActivity = 0x51,
}

impl From<AppearanceCategory> for i32 {
    fn from(cat: AppearanceCategory) -> Self {
        ((cat as u16) << 6) as _
    }
}

pub struct AdvertiseConfiguration {
    pub set_scan_rsp: bool,
    pub include_name: bool,
    pub include_txpower: bool,
    pub min_interval: i32,
    pub max_interval: i32,
    pub manufacturer: Option<String>,
    pub service: Option<String>,
    pub service_uuid: Option<String>,
    pub appearance: AppearanceCategory,
    pub flag: u8,
}

impl Default for AdvertiseConfiguration {
    fn default() -> Self {
        Self {
            set_scan_rsp: false,
            include_name: false,
            include_txpower: false,
            min_interval: 0,
            max_interval: 0,
            manufacturer: None,
            service: None,
            service_uuid: None,
            appearance: AppearanceCategory::Unknown,
            flag: ESP_BLE_ADV_FLAG_NON_LIMIT_DISC as _,
        }
    }
}

impl From<AdvertiseConfiguration> for esp_ble_adv_data_t {
    fn from(config: AdvertiseConfiguration) -> Self {
        esp_ble_adv_data_t {
            set_scan_rsp: config.set_scan_rsp,
            include_name: config.include_name,
            include_txpower: config.include_txpower,
            min_interval: config.min_interval,
            max_interval: config.max_interval,
            manufacturer_len: 0,
            p_manufacturer_data: std::ptr::null_mut(),
            service_data_len: 0,
            p_service_data: std::ptr::null_mut(),
            service_uuid_len: 0,
            p_service_uuid: std::ptr::null_mut(),
            appearance: config.appearance.into(),
            flag: config.flag,
        }
    }
}
