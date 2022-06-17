use anyhow::Result;
use embedded_hal_0_2::blocking::delay::DelayUs;

use std::{thread, time::Duration};

use esp_idf_svc::nvs::*;

use esp_idf_hal::delay;

mod ble;

fn main() {
    init_esp().expect("Error initializing ESP");

    #[allow(unused)]
    let default_nvs = EspDefaultNvs::new().unwrap();

    let mut delay = delay::Ets {};

    delay.delay_us(100_u32);

    //let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    ble::bluetooth().unwrap();

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
