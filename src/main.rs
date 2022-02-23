#![feature(int_abs_diff)]

use anyhow::Result;

use std::{thread, time::Duration};

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::image::*;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;

use esp_idf_hal::{delay, gpio, peripherals, prelude::*, spi};

use st7789;

fn main() {
    init_esp().expect("Error initializing ESP");

    let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    let mosi = peripherals.pins.gpio19.into_output().unwrap();
    let sclk = peripherals.pins.gpio18.into_output().unwrap();
    let cs = peripherals.pins.gpio5.into_output().unwrap();
    let dc = peripherals.pins.gpio16.into_output().unwrap();
    let rst = peripherals.pins.gpio23.into_output().unwrap();
    let bl = peripherals.pins.gpio4.into_output().unwrap();

    let config = <spi::config::Config as Default>::default()
        .baudrate(26.MHz().into())
        // .bit_order(embedded_hal::spi::BitOrder::MSBFirst)
        .data_mode(embedded_hal::spi::MODE_0);

    let spi = spi::Master::<spi::SPI2, _, _, _, _>::new(
        peripherals.spi2,
        spi::Pins {
            sclk,
            sdo: mosi,
            sdi: Option::<gpio::Gpio21<gpio::Unknown>>::None,
            cs: Some(cs),
        },
        config,
    )
    .unwrap();
    let di = SPIInterfaceNoCS::new(spi, dc);

    let mut display = st7789::ST7789::new(di, Some(rst), Some(bl), 135, 240);

    // initialize
    display.init(&mut delay::Ets).unwrap();
    // set default orientation
    display
        .set_orientation(st7789::Orientation::Portrait)
        .unwrap();

    let raw_image_data = ImageRawLE::new(include_bytes!("../assets/ferris.raw"), 86);
    let ferris = Image::new(&raw_image_data, Point::new(34, 8));

    // draw image on black background
    display.clear(Rgb565::BLACK).unwrap();
    ferris.draw(&mut display).unwrap();

    loop {
        thread::sleep(Duration::from_millis(500));
        log::info!("Looping...");
    }
}

fn init_esp() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    /*
        use esp_idf_svc::{netif::EspNetifStack, nvs::EspDefaultNvs, sysloop::EspSysLoopStack};
        use std::sync::Arc;

        #[allow(unused)]
        let netif_stack = Arc::new(EspNetifStack::new()?);
        #[allow(unused)]
        let sys_loop_stack = Arc::new(EspSysLoopStack::new()?);
        #[allow(unused)]
        let default_nvs = Arc::new(EspDefaultNvs::new()?);
    */

    /*
    unsafe { esp_idf_sys::gpio_install_isr_service(0) };
    */

    Ok(())
}
