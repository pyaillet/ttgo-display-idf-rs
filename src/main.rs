use anyhow::Result;
use embedded_hal_0_2::blocking::delay::DelayUs;

use std::{sync::Arc, thread, time::Duration};

use esp_idf_svc::nvs::*;
use esp_idf_sys::EspError;

use esp_idf_hal::delay;

use ::log::*;

mod ble;

struct TestBleApp {}

impl ble::GattApplication for TestBleApp {
    fn get_application_id(&self) -> u16 {
        2
    }
}

async fn bluetooth(nvs: Arc<EspDefaultNvs>) -> Result<(), EspError> {
    let mut ble = ble::EspBle::new("Test device".into(), nvs).expect("init ble");
    ble.register_gatt_service_application(Box::new(TestBleApp {}))
        .await?;

    let advertise_config = ble::advertise::AdvertiseConfiguration {
        appearance: ble::advertise::AppearanceCategory::Watch,
        include_name: true,
        ..Default::default()
    };

    ble.configure_advertising(advertise_config).await?;

    ble.start_advertise().await
}

fn main() {
    init_esp().expect("Error initializing ESP");

    #[allow(unused)]
    let default_nvs = Arc::new(EspDefaultNvs::new().unwrap());

    let mut delay = delay::Ets {};

    delay.delay_us(100_u32);

    //let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    match futures::executor::block_on(async { bluetooth(default_nvs).await }) {
        Ok(()) => info!("Bluetooth initialized"),
        Err(e) => error!("Error: {:?}", e),
    }
    // ble::bluetooth().unwrap();

    loop {
        thread::sleep(Duration::from_millis(20));
    }
}

fn init_esp() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    Ok(())
}
