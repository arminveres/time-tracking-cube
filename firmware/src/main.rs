#![no_std]
#![no_main]

use core::cell::RefCell;

use defmt::{error, info, unwrap, Debug2Format};
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, Async, Config, I2c, InterruptHandler},
    peripherals::I2C1,
    spi::{self, Spi},
};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Timer;
use embedded_sdmmc::{Error, Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};
use {defmt_rtt as _, panic_probe as _};

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

#[embassy_executor::task]
async fn log_accel(mut accel: adxl345_eh_driver::Driver<I2c<'static, I2C1, Async>>) -> ! {
    loop {
        let (x, y, z) = accel.get_accel_raw().unwrap();
        info!("ADXL345: x: {}, y: {}, z: {}", x, y, z);
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

    let spi = Spi::new_blocking(p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, spi_config);
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let sda = p.PIN_14;
    let scl = p.PIN_15;

    info!("Set up i2c on pin 14 and 15");
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
