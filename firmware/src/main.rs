#![no_std]
#![no_main]

use core::cell::RefCell;

use defmt::{error, info, unwrap, Debug2Format};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, Async, I2c, InterruptHandler},
    peripherals::I2C1,
    spi::{self, Spi},
};
use embassy_sync::{
    blocking_mutex::{raw::NoopRawMutex, Mutex},
};
use embassy_time::{Delay, Timer};
use embedded_hal::delay::DelayNs;
use embedded_sdmmc::{Error, Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use oled_async::{prelude::*, Builder};

struct DummyTimesource();

impl TimeSource for DummyTimesource {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

bind_interrupts!(struct Irqs {
    I2C1_IRQ => InterruptHandler<I2C1>;
});

const ADXL345_ADDR: u8 = 0x53;

type Accel = (i16, i16, i16);

/// Defines the six sides the cube has.
#[derive(PartialEq, Clone, Copy)]
enum Side {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
}

impl Side {
    pub fn get_side_from_accel(acc: Accel) -> Self {
        const THS: i16 = 127;

        if acc.2 > THS {
            Self::One
        } else if acc.2 < (-THS) {
            Self::Two
        } else if acc.1 > THS {
            Self::Three
        } else if acc.1 < (-THS) {
            Self::Four
        } else if acc.0 > THS {
            Self::Five
        } else if acc.0 < (-THS) {
            Self::Six
        } else {
            panic!("Unknown Side values")
        }
    }
}

// struct TTCConfig {
//     sides: u8,
// }

// impl TTCConfig {
//     pub fn gen_entry() {}
// }

struct TTCEntry {
    pub side: u8,
    pub duration: u64,
}

impl TTCEntry {
    fn new(side: Side, duration: u64) -> Self {
        Self {
            side: side as u8,
            duration,
        }
    }
}

#[embassy_executor::task]
async fn log_accel(mut accel: adxl345_eh_driver::Driver<I2c<'static, I2C1, Async>>) -> ! {
    const THS: u64 = 15; // Threshold in seconds on when to start a new timer.
    let mut time = embassy_time::Instant::now();
    let mut starting_side = Side::One;

    loop {
        let accel = aclm.get_accel_raw().unwrap();
        let current_side = Side::get_side_from_accel(accel);

        // info!("ADXL345: x: {}, y: {}, z: {}", accel.0, accel.1, accel.2);
        // info!("Side: {}", current_side as u8);
        // Timer::after_secs(1).await;

        if current_side == starting_side {
            continue;
        }
        if time.elapsed().as_secs() < THS {
            continue;
        }

        let entry = TTCEntry::new(starting_side, time.elapsed().as_secs());
        starting_side = current_side;
        time = embassy_time::Instant::now(); // reset time

        info!("New Entry: {}s on side {}", entry.duration, entry.side);
        // sender.send(entry);
        Timer::after_secs(1).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting main!");
    embassy_rp::pac::SIO.spinlock(31).write_value(1);
    let p = embassy_rp::init(Default::default());

    // SPI clock needs to be running at <= 400kHz during initialization
    let mut spi_config = spi::Config::default();
    spi_config.frequency = 400_000;

    // let spi = Spi::new_blocking(p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, spi_config);
    let spi = Spi::new(
        p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, p.DMA_CH0, p.DMA_CH1, spi_config,
    );
    // let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let sda = p.PIN_14;
    let scl = p.PIN_15;

    info!("Setting up i2c on pin 14 and 15");
    let i2c_conf = i2c::Config::default();
    let i2c = i2c::I2c::new_async(p.I2C1, scl, sda, Irqs, i2c_conf);

    let adxl = match adxl345_eh_driver::Driver::new(i2c, Some(ADXL345_ADDR)) {
        Ok(a) => a,
        Err(err) => loop {
            error!("Error: {}", Debug2Format(&err));
            Timer::after_secs(10).await;
        },
    };

    unwrap!(spawner.spawn(log_accel(adxl)));

    info!("Setting up OLED Display");
    let cs_disp = Output::new(p.PIN_7, Level::High);
    let dc = Output::new(p.PIN_6, Level::High);
    let mut reset = Output::new(p.PIN_8, Level::High);

    // let spi_dev = SpiDevice::new(&spi_bus, cs);
    let spi_dev = embedded_hal_bus::spi::ExclusiveDevice::new(spi, cs_disp, Delay);
    let di = display_interface_spi::SPIInterface::new(spi_dev, dc);

    let disp = oled_async::Builder::new(oled_async::displays::sh1107::Sh1107_64_128 {})
        .with_rotation(DisplayRotation::Rotate90)
        .connect(di);
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

    Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
        .draw(&mut disp)
        .unwrap();

    disp.flush().await.unwrap();

    loop {
        Timer::after_secs(1).await;
    }

    /*
        // Real cs pin
        let cs = Output::new(p.PIN_5, Level::High);

        // let spi_dev = ExclusiveDevice::new_no_delay(spi, cs);
        let spi_dev = SpiDevice::new(&spi_bus, cs);
        let sdcard = SdCard::new(spi_dev, embassy_time::Delay);
        info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

        let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource());

        let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
        info!("Volume 0: {:?}", Debug2Format(&volume0));

        let root_dir = RefCell::new(volume0.open_root_dir().unwrap());

        read_file(&root_dir).unwrap();

        write_file(&root_dir, "Hello from fresh!").unwrap();

        info!("All operations successfull");
        loop {
            Timer::after_secs(1).await;
        }
    */
}

fn read_file<D: embedded_sdmmc::BlockDevice, T: embedded_sdmmc::TimeSource>(
    dir: &RefCell<embedded_sdmmc::Directory<D, T, 4, 4, 1>>,
) -> Result<(), Error<SdCardError>> {
    let mut binding = dir.borrow_mut();
    let mut file = binding
        .open_file_in_dir("MY_FILE.TXT", Mode::ReadOnly)
        .unwrap();
    while !file.is_eof() {
        let mut buf = [0u8; 32];
        if let Ok(n) = file.read(&mut buf) {
            info!("{:a}", buf[..n]);
        }
    }

    Ok(())
}
fn write_file<D: embedded_sdmmc::BlockDevice, T: embedded_sdmmc::TimeSource>(
    dir: &RefCell<embedded_sdmmc::Directory<D, T, 4, 4, 1>>,
    text: &str,
) -> Result<(), Error<SdCardError>> {
    let mut binding = dir.borrow_mut();
    let mut file =
        match binding.open_file_in_dir("MY_FILE.TXT", embedded_sdmmc::Mode::ReadWriteAppend) {
            Ok(file) => file,
            Err(err) => loop {
                error!("We got an error: {:?}", defmt::Debug2Format(&err));
            },
        };

    if let Ok(()) = file.write(text.as_bytes()) {
        info!("Wrote to sd card");
    } else {
        error!("Failed to write");
    }
    Ok(())
}
