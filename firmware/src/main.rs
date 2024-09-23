//! This example shows how to use `embedded-sdmmc` with the RP2040 chip, over SPI.
//!
//! The example will attempt to read a file `MY_FILE.TXT` from the root directory
//! of the SD card and print its contents.

#![no_std]
#![no_main]

use core::fmt::Write;
use defmt::unwrap;
use embassy_embedded_hal::SetConfig;
use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::spi::Spi;
use embassy_rp::usb::{Driver, Instance, InterruptHandler};
use embassy_rp::{gpio, spi};
use embassy_time::{Duration, Timer};
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::UsbDevice;
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_sdmmc::sdcard::{DummyCsPin, SdCard};
use gpio::{Level, Output};
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

struct DummyTimesource();

impl embedded_sdmmc::TimeSource for DummyTimesource {
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

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    embassy_rp::pac::SIO.spinlock(31).write_value(1);
    let p = embassy_rp::init(Default::default());

    // Create the driver, from the HAL.
    let driver = Driver::new(p.USB, Irqs);

    // Create embassy-usb Config
    let usb_config = {
        let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
        config.manufacturer = Some("Embassy");
        config.product = Some("USB-serial example");
        config.serial_number = Some("12345678");
        config.max_power = 100;
        config.max_packet_size_0 = 64;

        // Required for windows compatibility.
        // https://developer.nordicsemi.com/nRF_Connect_SDK/doc/1.9.1/kconfig/CONFIG_CDC_ACM_IAD.html#help
        config.device_class = 0xEF;
        config.device_sub_class = 0x02;
        config.device_protocol = 0x01;
        config.composite_with_iads = true;
        config
    };

    // Create embassy-usb DeviceBuilder using the driver and config.
    // It needs some buffers for building the descriptors.
    let mut builder = {
        static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
        static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

        let builder = embassy_usb::Builder::new(
            driver,
            usb_config,
            CONFIG_DESCRIPTOR.init([0; 256]),
            BOS_DESCRIPTOR.init([0; 256]),
            &mut [], // no msos descriptors
            CONTROL_BUF.init([0; 64]),
        );
        builder
    };

    // Create classes on the builder.
    let mut class = {
        static STATE: StaticCell<State> = StaticCell::new();
        let state = STATE.init(State::new());
        CdcAcmClass::new(&mut builder, state, 64)
    };

    // Build the builder.
    let usb = builder.build();

    // Run the USB device.
    unwrap!(spawner.spawn(usb_task(usb)));

    class.wait_connection().await;

    let _ = class.write_packet(b"we did it!\r\n").await;
    Timer::after_secs(1).await;
    let _ = class.write_packet(b"we did it!\r\n").await;
    Timer::after_secs(1).await;

    // SPI clock needs to be running at <= 400kHz during initialization
    let mut spi_config = spi::Config::default();
    spi_config.frequency = 400_000;

    let spi = Spi::new_blocking(p.SPI0, p.PIN_2, p.PIN_3, p.PIN_4, spi_config);
    // Use a dummy cs pin here, for embedded-hal SpiDevice compatibility reasons
    let spi_dev = ExclusiveDevice::new_no_delay(spi, DummyCsPin);

    // Real cs pin
    let cs = Output::new(p.PIN_5, Level::High);

    let sdcard = SdCard::new(spi_dev, cs, embassy_time::Delay);
    // info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    let card_size = match sdcard.num_bytes() {
        Ok(num) => num.to_le_bytes(),
        Err(_) => 0_u64.to_le_bytes(),
    };
    class.write_packet(b"Opened card with size ").await.unwrap();
    class.write_packet(card_size.as_ref()).await.unwrap();
    class.write_packet(b"\r\n").await.unwrap();

    // BUG(aver): cannot use faster speeds, otherwise we get read errors
    // Now that the card is initialized, the SPI clock can go faster
    //
    // let mut spi_config = spi::Config::default();
    // spi_config.frequency = 16_000_000;
    // sdcard.spi(|dev| dev.bus_mut().set_config(&spi_config)).ok();

    // Now let's look for volumes (also known as partitions) on our block device.
    // To do this we need a Volume Manager. It will take ownership of the block device.
    let mut volume_mgr = embedded_sdmmc::VolumeManager::new(sdcard, DummyTimesource());

    // let raw_file = {
    // Try and access Volume 0 (i.e. the first partition).
    // The volume object holds information about the filesystem on that volume.
    // let mut volume0 = volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)).unwrap();
    // info!("Volume 0: {:?}", defmt::Debug2Format(&volume0));

    let mut volume0 = match volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)) {
        Ok(vol) => vol,
        Err(err) => {
            let mut text: heapless::String<64> = heapless::String::new();
            write!(&mut text, "We got an error: {:?}\r\n", err).unwrap();

            loop {
                let _ = class.write_packet(text.as_bytes()).await;
                Timer::after_secs(1).await;
            }
        }
    };
    let _ = class.write_packet(b"Opened Volume 0\r\n").await;

    // Open the root directory (mutably borrows from the volume).
    let mut root_dir = match volume0.open_root_dir() {
        Ok(root) => root,
        Err(err) => {
            let mut text: heapless::String<64> = heapless::String::new();
            write!(&mut text, "We got an error: {:?}\r\n", err).unwrap();
            loop {
                let _ = class.write_packet(text.as_bytes()).await;
                Timer::after_secs(1).await;
            }
        }
    };
    let _ = class.write_packet(b"Opened root dir\r\n").await;

    {
        // Open a file called "MY_FILE.TXT" in the root directory
        // This mutably borrows the directory.
        let mut my_file =
            match root_dir.open_file_in_dir("MY_FILE.TXT", embedded_sdmmc::Mode::ReadOnly) {
                Ok(file) => file,
                Err(err) => {
                    let mut text: heapless::String<64> = heapless::String::new();
                    write!(&mut text, "We got an error: {:?}\r\n", err).unwrap();

                    loop {
                        let _ = class.write_packet(text.as_bytes()).await;
                        Timer::after_secs(1).await;
                    }
                }
            };

        // Print the contents of the file
        while !my_file.is_eof() {
            let mut buf = [0u8; 32];
            if let Ok(n) = my_file.read(&mut buf) {
                // info!("{:a}", buf[..n]);
                let _ = class.write_packet(&buf[..n]).await;
            }
        }
    }
    {
        // Open a file called "MY_FILE.TXT" in the root directory
        // This mutably borrows the directory.
        let mut my_file =
            match root_dir.open_file_in_dir("MY_FILE.TXT", embedded_sdmmc::Mode::ReadWriteAppend) {
                Ok(file) => file,
                Err(err) => {
                    let mut text: heapless::String<64> = heapless::String::new();
                    write!(&mut text, "We got an error: {:?}\r\n", err).unwrap();

                    loop {
                        let _ = class.write_packet(text.as_bytes()).await;
                        Timer::after_secs(1).await;
                    }
                }
            };

        if let Ok(()) = my_file.write(b"Hello from pico!\n") {
            let _ = class.write_packet(b"Wrote to sd card").await;
        } else {
            let _ = class.write_packet(b"Failed to write").await;
        }
    }

    loop {
        let _ = class.write_packet(b"All operations successfull\r\n").await;
        Timer::after_secs(1).await;
    }
}

type MyUsbDriver = Driver<'static, USB>;
type MyUsbDevice = UsbDevice<'static, MyUsbDriver>;

#[embassy_executor::task]
async fn usb_task(mut usb: MyUsbDevice) -> ! {
    usb.run().await
}
