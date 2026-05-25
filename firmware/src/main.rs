#![no_std]
#![no_main]

mod config;
mod sd_card;
mod tasks;
mod time_tracking;

use core::cell::RefCell;

use defmt::{Debug2Format, debug, info};
use embassy_embedded_hal::shared_bus::{asynch, blocking};
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_rp::{
    bind_interrupts,
    gpio::{Level, Output},
    i2c::{self, InterruptHandler},
    peripherals::{I2C1, SPI0, SPI1},
    spi::{self, Spi},
};
use embassy_sync::{blocking_mutex, mutex};
use embassy_time::Delay;
use fugit::{HertzU32, RateExtU32};
use sd_card::SDCard;
use static_cell::StaticCell;
use tasks::display::DisplayType;
use {defmt_rtt as _, panic_probe as _};

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
        let mut spi_config = spi::Config::default();
        let val: HertzU32 = 400.kHz();
        spi_config.frequency = val.to_Hz();

        let spi = Spi::new(
            p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, p.DMA_CH0, p.DMA_CH1, spi_config,
        );
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
        info!("Setting up SD Card");
        let mut spi_config = spi::Config::default();
        spi_config.frequency = 400_000;

        let spi = RefCell::new(Spi::new_blocking(
            p.SPI1, p.PIN_10, p.PIN_11, p.PIN_12, spi_config,
        ));
        let spi_bus = SPI_BUS_SDCARD.init(blocking_mutex::Mutex::new(spi));
        let cs = Output::new(p.PIN_13, Level::High);
        let spi_dev = blocking::spi::SpiDevice::new(spi_bus, cs);

        SDCard::new(spi_dev)
    };

    let mut reset = Output::new(p.PIN_8, Level::High);
    let mut disp: DisplayType = disp.into();
    let mut delay = Delay {};

    disp.reset(&mut reset, &mut delay).unwrap();
    disp.init().await.unwrap();
    disp.clear();
    disp.flush().await.unwrap();

    join(
        tasks::display::display_task(disp),
        tasks::accel::log_accel(adxl, sdcard),
    )
    .await;
}
