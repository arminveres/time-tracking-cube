use core::cell::RefCell;

use defmt::{error, info, Debug2Format};
use embedded_sdmmc::{Error, Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};

// Define a dummy time source for the sd card filesytem
pub struct DummyTimesource();

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

// TODO(aver): create SD card struct with methods

pub fn setup_sd_card<SPI>(spi_dev: SPI)
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    let sdcard = SdCard::new(spi_dev, embassy_time::Delay);
    info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

    let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource());

    let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
    info!("Volume 0: {:?}", Debug2Format(&volume0));

    let root_dir = RefCell::new(volume0.open_root_dir().unwrap());

    read_file(&root_dir).unwrap();

    write_file(&root_dir, "Hello from fresh!").unwrap();

    info!("All operations successfull");
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
