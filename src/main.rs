use anyhow::Result;
use embedded_graphics_framebuf::AsBytes;
use embedded_hal_0_2::digital::v2::OutputPin;
use embedded_svc::sys_time::SystemTime;
use mipidsi::ColorOrder;
use mipidsi::DisplayOptions;

use std::{f64::consts, thread, time::Duration};

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics_framebuf::FrameBuf;

use esp_idf_hal::{
    delay, gpio, peripherals,
    prelude::*,
    spi::{self, config::DeviceFlag},
};

use mipidsi::{Display, Orientation};

const TRANSFER_SIZE: usize = 32400;

fn main() {
    init_esp().expect("Error initializing ESP");

    let timer = esp_idf_svc::timer::EspTimerService::new().unwrap();

    let mut delay = delay::Ets;

    let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    let mosi = peripherals.pins.gpio19.into_output().unwrap();
    let sclk = peripherals.pins.gpio18.into_output().unwrap();
    let cs = peripherals.pins.gpio5.into_output().unwrap();
    let dc = peripherals.pins.gpio16.into_output().unwrap();
    let rst = peripherals.pins.gpio23.into_output().unwrap();
    let mut bl = peripherals.pins.gpio4.into_output().unwrap();

    let config = <spi::config::Config as Default>::default()
        //.baudrate(53.MHz().into())
        .baudrate(80.MHz().into())
        .device_flags(DeviceFlag::NoDummy as u32)
        .dma_channel(2)
        .max_transfer_size(TRANSFER_SIZE as i32)
        // .bit_order(embedded_hal::spi::BitOrder::MSBFirst)
        .data_mode(embedded_hal::spi::MODE_0);

    let spi = spi::Master::<spi::SPI3, _, _, _, _>::new(
        peripherals.spi3,
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

    let mut display = Display::st7789(di, rst);

    let mut options = DisplayOptions::default();
    options.color_order = ColorOrder::Bgr;

    // initialize
    display.init(&mut delay, options).unwrap();
    // set default orientation
    display
        .set_orientation(Orientation::Landscape(false))
        .unwrap();
    // display.set_inversion(false).unwrap();
    display.set_scroll_offset(0).unwrap();

    display.clear(Rgb565::BLACK).unwrap();
    bl.set_high().unwrap();

    log::info!("ST7789 initialized");

    static mut FBUFF: FrameBuf<Rgb565, 240_usize, 135_usize, 32_400_usize> =
        FrameBuf([Rgb565::BLACK; 32_400_usize]);
    let fbuff = unsafe { &mut FBUFF };

    fbuff.clear_black();
    log::info!("FB initialized");

    let mut i: i32 = 0;
    let radius: f64 = 35.0;

    let mut y = 100.0;
    let mut dy = 1.0;
    let mut x = 100.0;
    let mut dx = 1.0;

    let min_x = 0; //40; // RED
    let max_x = 239; //279; // YELLOW
    let min_y = 0; //53; // GREEN
    let max_y = 134; //187; // BLUE

    log::info!("Border drawn on FB");

    let mut increment: u16 = 1u16;
    let mut color: u16 = 0u16;

    log::info!("Ready to go !");

    loop {
        let angle0 = (i as f64).to_radians();
        let angle1 = angle0 + (2.0 * consts::PI / 3.0);
        let angle2 = angle1 + (2.0 * consts::PI / 3.0);

        let x0 = (angle0.cos() * radius + x) as i32;
        let y0 = (angle0.sin() * radius + y) as i32;
        let x1 = (angle1.cos() * radius + x) as i32;
        let y1 = (angle1.sin() * radius + y) as i32;
        let x2 = (angle2.cos() * radius + x) as i32;
        let y2 = (angle2.sin() * radius + y) as i32;

        Line::new(Point::new(min_x, min_y), Point::new(min_x, max_y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::RED, 1))
            .draw(fbuff)
            .unwrap();
        Line::new(Point::new(min_x, max_y), Point::new(max_x, max_y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::BLUE, 1))
            .draw(fbuff)
            .unwrap();
        Line::new(Point::new(max_x, max_y), Point::new(max_x, min_y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::YELLOW, 1))
            .draw(fbuff)
            .unwrap();
        Line::new(Point::new(max_x, min_y), Point::new(min_x, min_y))
            .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1))
            .draw(fbuff)
            .unwrap();

        let (r, g, b): (u8, u8, u8) = (
            (((color >> 11) & 0b11111) as u8),
            (((color >> 6) & 0b111111) as u8),
            ((color & 0b11111) as u8),
        );

        // triangle to be shown "in the scroll zone"
        let triangle = Triangle::new(Point::new(x0, y0), Point::new(x1, y1), Point::new(x2, y2))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::new(r, g, b)));
        triangle.draw(fbuff).unwrap();
        // fbuff.clear(Rgb565::BLUE).unwrap();
        // log::info!("Triangle drawn on FB");

        let start = timer.now();
        /*
        display
            .set_pixels(40, 53, 240 - 1 + 40, 53 + 135, fbuff.into_iter())
            .unwrap();
            */

        display
            .write_raw(
                40,
                53,
                240 - 1 + 40,
                53 - 1 + 135,
                TRANSFER_SIZE,
                fbuff.as_bytes(),
            )
            .unwrap();
        let end = timer.now();

        log::info!(
            "Time drawing - start: {:?} - end: {:?} - total: {:?}",
            start,
            end,
            end - start
        );

        thread::sleep(Duration::from_millis(10));
        // log::info!("FB sent to display");
        fbuff.clear_black();
        // display.write_raw(40, 53, 240 - 1 + 40, 53 + 135, fbuff.as_bytes()).unwrap();
        // log::info!("FB cleared");

        if y > 134.0 - radius {
            dy = -1.0;
        } else if y < 0.0 as f64 + radius {
            dy = 1.0;
        }
        if x > 239.0 as f64 - radius {
            dx = -1.0;
        } else if x < 0.0 as f64 + radius {
            dx = 1.0;
        }
        x = x + dx;
        y = y + dy;
        i = (i + 1) % 360;
        color = (color << 1) | increment;
        if color == u16::max_value() {
            increment = 0;
        } else if color == 0 {
            increment = 1;
        }
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

