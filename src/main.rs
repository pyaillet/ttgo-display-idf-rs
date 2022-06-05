use anyhow::Result;
use embedded_hal_0_2::{blocking::delay::DelayUs,digital::v2::OutputPin};

use std::{f64::consts, thread, time::Duration};

use display_interface_spi::SPIInterfaceNoCS;

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_graphics_framebuf::{AsWords, FrameBuf};

use esp_idf_hal::{delay, gpio, peripherals, prelude::*, spi};

use mipidsi::{Display, Orientation};

const X: usize = 240;
const Y: usize = 135;

fn main() {
    init_esp().expect("Error initializing ESP");

    let mut delay = delay::Ets {};

    delay.delay_us(100_u32);

    let peripherals = peripherals::Peripherals::take().expect("Failed to take esp peripherals");

    let mosi = peripherals.pins.gpio19.into_output().unwrap();
    let sclk = peripherals.pins.gpio18.into_output().unwrap();
    let cs = peripherals.pins.gpio5.into_output().unwrap();
    let dc = peripherals.pins.gpio16.into_output().unwrap();
    let rst = peripherals.pins.gpio23.into_output().unwrap();
    let mut bl = peripherals.pins.gpio4.into_output().unwrap();

    let config = <spi::config::Config as Default>::default()
        .baudrate(80.MHz().into())
        .write_only(true)
        .dma(spi::Dma::Channel2(4096))
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

    let mut display = Display::st7789(di, rst);

    // initialize
    display.init(&mut delay, Default::default()).unwrap();
    // set default orientation
    display
        .set_orientation(Orientation::Landscape(true))
        .unwrap();
    display.set_scroll_offset(0).unwrap();

    display.clear(Rgb565::BLACK).unwrap();
    bl.set_high().unwrap();

    log::info!("ST7789 initialized");

    static mut FBUFF: FrameBuf<Rgb565, X, Y, { X * Y }> =
        FrameBuf([Rgb565::BLACK; { X * Y } ]);
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
    let max_x = (X - 1) as i32; //279; // YELLOW
    let min_y = 0; //53; // GREEN
    let max_y = (Y - 1) as i32; //187; // BLUE

    log::info!("Border drawn on FB");

    display.set_scroll_region(40, X as u16, 39).unwrap();

    let mut scroll_offset = 0_u16;

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

        // triangle to be shown "in the scroll zone"
        let triangle = Triangle::new(Point::new(x0, y0), Point::new(x1, y1), Point::new(x2, y2))
            .into_styled(PrimitiveStyle::with_fill(Rgb565::GREEN));
        triangle.draw(fbuff).unwrap();

        /*
        display
            .set_pixels(40, 53, 240 - 1 + 40, 53 + 135, fbuff.into_iter())
            .unwrap();
        */

        display.set_scroll_offset(scroll_offset).unwrap();
        display
            .write_raw(40, 53, 240 - 1 + 40, 53 + 135, fbuff.as_words())
            .unwrap();

        thread::sleep(Duration::from_millis(20));
        fbuff.clear_black();

        if y > max_y as f64 - radius {
            dy = -1.0;
        } else if y < 0.0 as f64 + radius {
            dy = 1.0;
        }
        if x > max_x as f64 - radius {
            dx = -1.0;
        } else if x < 0.0 as f64 + radius {
            dx = 1.0;
        }
        x = x + dx;
        y = y + dy;
        i = (i + 1) % 360;
        scroll_offset = (scroll_offset + 1) % X as u16;
    }
}

fn init_esp() -> Result<()> {
    esp_idf_sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    Ok(())
}
