use anyhow::Result;
use embedded_hal_0_2::blocking::delay::DelayUs;

use futures::channel::oneshot;
use std::{thread, time::Duration};

use esp_idf_hal::delay;

fn main() {
    init_esp().expect("Error initializing ESP");

    let mut delay = delay::Ets {};

    delay.delay_us(100_u32);

    let (sender, receiver) = oneshot::channel::<i32>();

    thread::spawn(|| {
        println!("THREAD: sleeping zzz...");
        thread::sleep(Duration::from_millis(1000));
        println!("THREAD: i'm awake! sending.");
        sender.send(3).unwrap();
    });

    println!("MAIN: doing some useful stuff");

    futures::executor::block_on(async {
        println!("MAIN: waiting for msg...");
        println!("MAIN: got: {:?}", receiver.await)
    });

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
