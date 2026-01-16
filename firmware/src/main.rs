#![no_std]
#![no_main]

mod config;
mod sd_card;
mod time_tracking;

use core::cell::RefCell;
use core::fmt::Write;
use embassy_futures::join::join;

use defmt::{Debug2Format, debug, info, unwrap};
use embassy_embedded_hal::shared_bus::asynch;
use embassy_embedded_hal::shared_bus::blocking;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, Async, I2c, InterruptHandler},
    peripherals::{I2C1, SPI0, SPI1},
    spi::{self, Spi},
};
use embassy_sync::blocking_mutex;
use embassy_sync::mutex;
use embassy_time::{Delay, Timer};
use fugit::{HertzU32, RateExtU32};
use heapless::String;
use sd_card::SDCard;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use oled_async::prelude::*;

bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<I2C1>;
});

const ADXL345_ADDR: u8 = 0x53;

type Spi0BusAsync = mutex::Mutex<blocking_mutex::raw::NoopRawMutex, Spi<'static, SPI0, spi::Async>>;
static SPI_BUS_DISPLAY: StaticCell<Spi0BusAsync> = StaticCell::new();

type Spi1Bus = blocking_mutex::Mutex<
    blocking_mutex::raw::ThreadModeRawMutex,
    RefCell<Spi<'static, SPI1, spi::Blocking>>,
>;
static SPI_BUS_SDCARD: StaticCell<Spi1Bus> = StaticCell::new();

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting main!");
    embassy_rp::pac::SIO.spinlock(31).write_value(1);
    let p = embassy_rp::init(Default::default());

    let adxl = {
        debug!("Setting up i2c on pin 14 and 15");
        let i2c_conf = i2c::Config::default();

        let i2c = i2c::I2c::new_async(p.I2C1, p.PIN_15, p.PIN_14, Irqs, i2c_conf);

        match adxl345_eh_driver::Driver::new(i2c, Some(ADXL345_ADDR)) {
            Ok(a) => a,
            Err(err) => panic!("Error: {:?}", Debug2Format(&err)),
        }
    };

    let disp = {
        // SPI clock needs to be running at <= 400kHz during initialization
        let mut spi_config = spi::Config::default();
        let val: HertzU32 = 400.kHz();
        spi_config.frequency = val.to_Hz();

        // let spi = Spi::new_blocking(p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, spi_config);
        let spi = Spi::new(
            p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, p.DMA_CH0, p.DMA_CH1, spi_config,
        );
        // let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));
        let spi_bus = SPI_BUS_DISPLAY.init(mutex::Mutex::new(spi));

        info!("Setting up OLED Display");
        let cs_disp = Output::new(p.PIN_7, Level::High);
        let dc = Output::new(p.PIN_6, Level::High);

        let spi_dev = asynch::spi::SpiDevice::new(spi_bus, cs_disp);

        let disp_interface = display_interface_spi::SPIInterface::new(spi_dev, dc);

        oled_async::Builder::new(oled_async::displays::sh1107::Sh1107_64_128 {})
            .with_rotation(DisplayRotation::Rotate90)
            .connect(disp_interface)
    };

    let sdcard = {
        info!("SettinRateExtU32 SD Card");

        let mut spi_config = spi::Config::default();
        // TODO(aver): test max frequency
        spi_config.frequency = 400_000;

        let spi = RefCell::new(Spi::new_blocking(
            p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, spi_config,
        ));

        let spi_bus = SPI_BUS_SDCARD.init(blocking_mutex::Mutex::new(spi));

        // Real cs pin
        let cs = Output::new(p.PIN_13, Level::High);

        // let spi_dev = ExclusiveDevice::new_no_delay(spi_bus, cs);
        let spi_dev = blocking::spi::SpiDevice::new(spi_bus, cs);

        sd_card::SDCard::new(spi_dev)
    };

    debug!("Testing the display");
    // Do stuff with the display
    let mut reset = Output::new(p.PIN_8, Level::High);

    let mut disp: GraphicsMode<_, _> = disp.into();
    let mut delay = Delay {};

    disp.reset(&mut reset, &mut delay).unwrap();
    disp.init().await.unwrap();
    disp.clear();
    disp.flush().await.unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    Text::with_baseline("Hello there!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();

    disp.flush().await.unwrap();

    // const FILE_NAME: &str = "test.txt";
    debug!("Writing to SDCard");
    // sdcard.write_file(FILE_NAME, "Hello From Rust!").unwrap();
    // sdcard.write_file(FILE_NAME, "Hello From Rust 2!").unwrap();

    // unwrap!(spawner.spawn(log_accel(adxl)));

    let log_handle = log_accel(adxl, sdcard);
    let placeholder = placeholder();

    join(log_handle, placeholder).await;
}

async fn log_accel<SPI>(
    mut aclm: adxl345_eh_driver::Driver<I2c<'static, I2C1, Async>>,
    mut sd_card: SDCard<SPI>,
) where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    // TODO(aver): Create an entry on the filesystem, probably as a CSV file
    // - Will need to pass the sd card either fully to this task, or
    // - ping-pong via SPSC channel / Mutex
    const TRESHOLD: u64 = 15; // Threshold in seconds on when to start a new timer.
    const FILENAME: &str = "entries.csv";
    info!("Running Acceleration Task");

    let mut time = embassy_time::Instant::now();
    let mut starting_side = time_tracking::Side::One;
    let mut content: String<64> = String::new();

    debug!("Starting loop");
    loop {
        let raw_accel = aclm
            .get_accel_raw()
            .expect("Couldn't get acceleration data");

        let accel = time_tracking::Accel {
            x: raw_accel.0,
            y: raw_accel.1,
            z: raw_accel.2,
        };
        let current_side = accel.get_side();

        // info!("ADXL345: x: {}, y: {}, z: {}", accel.0, accel.1, accel.2);
        // info!("Side: {}", current_side as u8);
        // Timer::after_secs(1).await;

        if current_side == starting_side {
            continue;
        }
        if time.elapsed().as_secs() < TRESHOLD {
            continue;
        }

        let entry = time_tracking::Entry::new(starting_side, time.elapsed().as_secs());
        info!("logging new entry: side: {}, duration: {}", entry.side, entry.duration);
        starting_side = current_side;
        time = embassy_time::Instant::now(); // reset time

        unwrap!(
            write!(&mut content, "{},{}\n", entry.duration, entry.side),
            "Writing entry to buffer failed"
        );
        unwrap!(
            sd_card.write_file(FILENAME, content.as_str()),
            "Could not write to file"
        );

        info!("New Entry: {}s on side {}", entry.duration, entry.side);

        Timer::after_secs(1).await;
    }
}

// TODO(aver): remove this or make it more useful
async fn placeholder() -> ! {
    loop {
        Timer::after_secs(1000).await;
    }
}
