pub mod advertise;
pub mod gap;
pub mod gatt_client;
pub mod gatt_server;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex}, ffi::CString,
};

use futures::channel::oneshot;
use lazy_static::lazy_static;

use ::log::*;

use esp_idf_svc::nvs::EspDefaultNvs;

use esp_idf_sys::*;

pub use gap::*;
pub use gatt_client::*;
pub use gatt_server::*;

unsafe extern "C" fn gap_event_handler(
    event: esp_gap_ble_cb_event_t,
    param: *mut esp_ble_gap_cb_param_t,
) {
    let event = GapEvent::build(event, param);
    debug!("Called gap event handler with event {{ {:#?} }}", &event);

    match event {
        GapEvent::AdvertisingDatasetComplete(adv) => {
            Arc::clone(&ADV_CFG)
                .lock()
                .ok()
                .and_then(|mut m| m.take())
                .and_then(|sender| sender.send(esp!(adv.status)).ok())
                .unwrap_or_else(|| warn!("Unable to handle AdvertisingDatasetComplete event"));
        }
        GapEvent::ScanResponseDatasetComplete(rsp) => {
            Arc::clone(&ADV_SCAN_RSP_CFG)
                .lock()
                .ok()
                .and_then(|mut m| m.take())
                .and_then(|sender| sender.send(esp!(rsp.status)).ok())
                .unwrap_or_else(|| warn!("Unable to handle ScanResponseDatasetComplete event"));
        }
        GapEvent::AdvertisingStartComplete(start) => {
            Arc::clone(&ADV_START)
                .lock()
                .ok()
                .and_then(|mut m| m.take())
                .and_then(|sender| sender.send(esp!(start.status)).ok())
                .unwrap_or_else(|| warn!("Unable to handle AdvertisingStartComplete event"));
        }
        _ => warn!("Unhandled event"),
    }
}

unsafe extern "C" fn gatts_event_handler(
    event: esp_gatts_cb_event_t,
    gatts_if: esp_gatt_if_t,
    param: *mut esp_ble_gatts_cb_param_t,
) {
    let event = GattServiceEvent::build(event, param);
    debug!(
        "Called gatt service event handler with gatts_if: {}, event {{ {:#?} }}",
        gatts_if, &event
    );

    match event {
        GattServiceEvent::Register(register) => {
            let sender = Arc::clone(&REG_APP_RECV)
                .lock()
                .ok()
                .and_then(|mut m| m.remove(&register.app_id));
            if sender.map(|s| s.send(gatts_if).ok()).is_none() {
                error!("Error sending app registered event");
            }
        }
        _ => warn!("Unhandled event"),
    }
}

lazy_static! {
    static ref REG_APP_RECV: Arc<Mutex<HashMap<u16, oneshot::Sender<esp_gatt_if_t>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    static ref ADV_CFG: Arc<Mutex<Option<oneshot::Sender<Result<(), EspError>>>>> =
        Arc::new(Mutex::new(None));
    static ref ADV_SCAN_RSP_CFG: Arc<Mutex<Option<oneshot::Sender<Result<(), EspError>>>>> =
        Arc::new(Mutex::new(None));
    static ref ADV_START: Arc<Mutex<Option<oneshot::Sender<Result<(), EspError>>>>> =
        Arc::new(Mutex::new(None));
}

#[allow(dead_code)]
pub struct EspBle {
    device_name: String,
    nvs: Arc<EspDefaultNvs>,
    gatt_if_to_app: HashMap<esp_gatt_if_t, Box<dyn GattApplication>>,
}

impl EspBle {
    pub fn new(device_name: String, nvs: Arc<EspDefaultNvs>) -> Result<EspBle, EspError> {
        let ble = EspBle::init(device_name, nvs)?;

        Ok(ble)
    }

    fn init(device_name: String, nvs: Arc<EspDefaultNvs>) -> Result<EspBle, EspError> {
        let mut bt_cfg = esp_bt_controller_config_t {
            controller_task_stack_size: ESP_TASK_BT_CONTROLLER_STACK as _,
            controller_task_prio: ESP_TASK_BT_CONTROLLER_PRIO as _,
            hci_uart_no: BT_HCI_UART_NO_DEFAULT as _,
            hci_uart_baudrate: BT_HCI_UART_BAUDRATE_DEFAULT,
            scan_duplicate_mode: SCAN_DUPLICATE_MODE as _,
            scan_duplicate_type: SCAN_DUPLICATE_TYPE_VALUE as _,
            normal_adv_size: NORMAL_SCAN_DUPLICATE_CACHE_SIZE as _,
            mesh_adv_size: MESH_DUPLICATE_SCAN_CACHE_SIZE as _,
            send_adv_reserved_size: SCAN_SEND_ADV_RESERVED_SIZE as _,
            controller_debug_flag: CONTROLLER_ADV_LOST_DEBUG_BIT,
            mode: esp_bt_mode_t_ESP_BT_MODE_BLE as _,
            ble_max_conn: CONFIG_BTDM_CTRL_BLE_MAX_CONN_EFF as _,
            bt_max_acl_conn: CONFIG_BTDM_CTRL_BR_EDR_MAX_ACL_CONN_EFF as _,
            bt_sco_datapath: CONFIG_BTDM_CTRL_BR_EDR_SCO_DATA_PATH_EFF as _,
            auto_latency: BTDM_CTRL_AUTO_LATENCY_EFF != 0,
            bt_legacy_auth_vs_evt: BTDM_CTRL_LEGACY_AUTH_VENDOR_EVT_EFF != 0,
            bt_max_sync_conn: CONFIG_BTDM_CTRL_BR_EDR_MAX_SYNC_CONN_EFF as _,
            ble_sca: CONFIG_BTDM_BLE_SLEEP_CLOCK_ACCURACY_INDEX_EFF as _,
            pcm_role: CONFIG_BTDM_CTRL_PCM_ROLE_EFF as _,
            pcm_polar: CONFIG_BTDM_CTRL_PCM_POLAR_EFF as _,
            hli: BTDM_CTRL_HLI != 0,
            magic: ESP_BT_CONTROLLER_CONFIG_MAGIC_VAL,
        };

        esp!(unsafe { esp_bt_controller_init(&mut bt_cfg) })?;

        esp!(unsafe { esp_bt_controller_enable(esp_bt_mode_t_ESP_BT_MODE_BLE) })?;

        info!("init bluetooth");
        esp!(unsafe { esp_bluedroid_init() })?;

        esp!(unsafe { esp_bluedroid_enable() })?;

        esp!(unsafe { esp_ble_gatts_register_callback(Some(gatts_event_handler)) })?;

        esp!(unsafe { esp_ble_gap_register_callback(Some(gap_event_handler)) })?;

        esp!(unsafe { esp_ble_gatt_set_local_mtu(500) })?;

        let device_name_cstr = CString::new(device_name.clone()).unwrap();
        esp!(unsafe { esp_ble_gap_set_device_name(device_name_cstr.as_ptr() as _) })?;

        Ok(EspBle {
            device_name,
            nvs,
            gatt_if_to_app: HashMap::new(),
        })
    }

    pub async fn register_gatt_service_application(
        &mut self,
        application: Box<dyn gatt_server::GattApplication>,
    ) -> Result<(), EspError> {
        let (sender, receiver) = oneshot::channel::<esp_gatt_if_t>();

        if let Ok(mut m) = Arc::clone(&REG_APP_RECV).lock() {
            m.insert(application.get_application_id(), sender);
        } else {
            return Err(EspError::from(1).unwrap());
        }

        esp!(unsafe { esp_ble_gatts_app_register(application.get_application_id()) })?;

        let gatt_if = receiver
            .await
            .map_err(|_| EspError::from(ESP_ERR_INVALID_RESPONSE).unwrap())?;

        self.gatt_if_to_app.insert(gatt_if, application);

        Ok(())
    }

    pub async fn configure_advertising(
        &mut self,
        config: advertise::AdvertiseConfiguration,
    ) -> Result<(), EspError> {
        if config.set_scan_rsp {
            if let Some(receiver) = Arc::clone(&ADV_SCAN_RSP_CFG).lock().ok().map(|mut m| {
                let (sender, receiver) = oneshot::channel::<Result<(), EspError>>();
                *m = Some(sender);
                receiver
            }) {
                let mut adv_data: esp_ble_adv_data_t = config.into();

                esp!(unsafe { esp_ble_gap_config_adv_data(&mut adv_data) })?;

                receiver.await.unwrap()
            } else {
                Err(EspError::from(ESP_ERR_INVALID_RESPONSE).unwrap())
            }
        } else if let Some(receiver) = Arc::clone(&ADV_CFG).lock().ok().map(|mut m| {
            let (sender, receiver) = oneshot::channel::<Result<(), EspError>>();
            *m = Some(sender);
            receiver
        }) {
            let mut adv_data: esp_ble_adv_data_t = config.into();

            esp!(unsafe { esp_ble_gap_config_adv_data(&mut adv_data) })?;

            receiver.await.unwrap()
        } else {
            Err(EspError::from(ESP_ERR_INVALID_RESPONSE).unwrap())
        }
    }

    pub async fn start_advertise(&mut self) -> Result<(), EspError> {
        if let Some(receiver) = Arc::clone(&ADV_START).lock().ok().map(|mut m| {
            let (sender, receiver) = oneshot::channel::<Result<(), EspError>>();
            *m = Some(sender);
            receiver
        }) {
            let mut adv_param: esp_ble_adv_params_t = esp_ble_adv_params_t {
                adv_int_min: 0x20,
                adv_int_max: 0x40,
                adv_type: 0x00,
                own_addr_type: 0x00,
                peer_addr: [0; 6],
                peer_addr_type: 0,
                channel_map: 0x07,
                adv_filter_policy: 0x00,
            };

            esp!(unsafe { esp_ble_gap_start_advertising(&mut adv_param) })?;

            receiver.await.unwrap()
        } else {
            Err(EspError::from(ESP_ERR_INVALID_RESPONSE).unwrap())
        }
    }
}
