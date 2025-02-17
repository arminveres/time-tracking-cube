use heapless::Vec;

use defmt::{debug, error, info, Debug2Format};
use embedded_sdmmc::{Mode, SdCard, SdCardError, TimeSource, VolumeIdx, VolumeManager};

/// Max file size in bytes
const RET_BUF_SIZE: usize = 256;

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

pub struct SDCard<SPI>
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    // sd_card: SdCard<SPI, embassy_time::Delay>,
    volume_mgr: VolumeManager<SdCard<SPI, embassy_time::Delay>, DummyTimesource>,
    // root_dir: Directory<'a, SdCard<SPI, embassy_time::Delay>, DummyTimesource, 4, 4, 1>,
}

impl<SPI> SDCard<SPI>
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    pub fn new(spi_device: SPI) -> Self {
        let sd_card = SdCard::new(spi_device, embassy_time::Delay);
        // let mut volume_mgr = VolumeManager::new(sd_card, DummyTimesource());
        // we open volume 0, as we currently don't support complex file handling yet
        // let mut volume0 = match volume_mgr.open_volume(VolumeIdx(0)) {
        //     Ok(vol) => vol,
        //     Err(err) => panic!("{:?}", err),
        // };
        // debug!("Volume 0: {:?}", Debug2Format(&volume0));

        // let root_dir = match volume0.open_root_dir() {
        //     Ok(root_dir) => root_dir,
        //     Err(err) => panic!("{:?}", err),
        // };
        Self {
            // sd_card: SdCard::new(spi_device, embassy_time::Delay),
            volume_mgr: VolumeManager::new(sd_card, DummyTimesource()),
            // root_dir,
        }
    }

    pub fn write_file(
        &mut self,
        file_name: &str,
        content: &str,
    ) -> Result<(), embedded_sdmmc::Error<embedded_sdmmc::SdCardError>> {
        // we open volume 0, as we currently don't support complex file handling yet
        let mut volume0 = match self.volume_mgr.open_volume(VolumeIdx(0)) {
            Ok(vol) => vol,
            Err(err) => panic!("{:?}", err),
        };
        debug!("Volume 0: {:?}", Debug2Format(&volume0));

        let mut root_dir = match volume0.open_root_dir() {
            // Ok(root_dir) => RefCell::new(root_dir),
            Ok(root_dir) => root_dir,
            Err(err) => panic!("{:?}", err),
        };

        let mut file = match root_dir.open_file_in_dir(file_name, Mode::ReadWriteCreateOrAppend) {
            Ok(file) => file,
            Err(err) => panic!("{:?}", err),
        };

        match file.write(content.as_bytes()) {
            Ok(_) => {
                info!("Writing to file {} was successfull!", file_name);
                Ok(())
            }
            Err(err) => {
                error!(
                    "Caught error while writing to file: {:?}",
                    Debug2Format(&err)
                );
                Err(err)
            }
        }
    }
    pub fn read_file(
        &mut self,
        file_name: &str,
    ) -> Result<Vec<u8, RET_BUF_SIZE>, embedded_sdmmc::Error<SdCardError>> {
        // we open volume 0, as we currently don't support complex file handling yet
        let mut volume0 = match self.volume_mgr.open_volume(VolumeIdx(0)) {
            Ok(vol) => vol,
            Err(err) => panic!("{:?}", err),
        };
        debug!("Volume 0: {:?}", Debug2Format(&volume0));

        let mut root_dir = match volume0.open_root_dir() {
            // Ok(root_dir) => RefCell::new(root_dir),
            Ok(root_dir) => root_dir,
            Err(err) => panic!("{:?}", err),
        };

        let mut file = match root_dir.open_file_in_dir(file_name, Mode::ReadOnly) {
            Ok(file) => file,
            Err(err) => panic!("{:?}", err),
        };

        // TODO(aver): consider making this a static/member variable
        let mut ret_buf = Vec::<u8, RET_BUF_SIZE>::new();

        while !file.is_eof() {
            const LOCAL_BUF_SIZE: usize = 32;
            let mut buf = [0u8; LOCAL_BUF_SIZE];
            match file.read(&mut buf) {
                Ok(no_bytes_read) => {
                    if no_bytes_read >= LOCAL_BUF_SIZE {
                        error!("Bytes read larget than buffer size {}! Consider increasing local buffer!", no_bytes_read);
                        return Err(embedded_sdmmc::Error::Unsupported);
                    }
                    if let Err(err) = ret_buf.extend_from_slice(&buf[..no_bytes_read]) {
                        // TODO(aver): find the correct error for this
                        return Err(todo!());
                    }
                }
                Err(err) => {
                    error!(
                        "File could not be read with error: {:?}",
                        Debug2Format(&err)
                    );
                    return Err(err);
                }
            }
        }
        Ok(ret_buf)
    }
}

// TODO(aver): create SD card struct with methods

// pub fn setup_sd_card<SPI>(spi_dev: SPI)
// where
//     SPI: embedded_hal::spi::SpiDevice<u8>,
// {
//     let sdcard = SdCard::new(spi_dev, embassy_time::Delay);
//     info!("Card size is {} bytes", sdcard.num_bytes().unwrap());

//     let mut volume_mgr = VolumeManager::new(sdcard, DummyTimesource());

//     let mut volume0 = volume_mgr.open_volume(VolumeIdx(0)).unwrap();
//     info!("Volume 0: {:?}", Debug2Format(&volume0));

//     let root_dir = RefCell::new(volume0.open_root_dir().unwrap());

//     read_file(&root_dir).unwrap();

//     write_file(&root_dir, "Hello from fresh!").unwrap();

//     info!("All operations successfull");
// }

// fn read_file<D: embedded_sdmmc::BlockDevice, T: embedded_sdmmc::TimeSource>(
//     dir: &RefCell<embedded_sdmmc::Directory<D, T, 4, 4, 1>>,
// ) -> Result<(), Error<SdCardError>> {
//     let mut binding = dir.borrow_mut();
//     let mut file = binding
//         .open_file_in_dir("MY_FILE.TXT", Mode::ReadOnly)
//         .unwrap();
//     while !file.is_eof() {
//         let mut buf = [0u8; 32];
//         if let Ok(n) = file.read(&mut buf) {
//             info!("{:a}", buf[..n]);
//         }
//     }

//     Ok(())
// }
// fn write_file<D: embedded_sdmmc::BlockDevice, T: embedded_sdmmc::TimeSource>(
//     dir: &RefCell<embedded_sdmmc::Directory<D, T, 4, 4, 1>>,
//     text: &str,
// ) -> Result<(), Error<SdCardError>> {
//     let mut binding = dir.borrow_mut();
//     let mut file =
//         match binding.open_file_in_dir("MY_FILE.TXT", embedded_sdmmc::Mode::ReadWriteAppend) {
//             Ok(file) => file,
//             Err(err) => loop {
//                 error!("We got an error: {:?}", defmt::Debug2Format(&err));
//             },
//         };

//     if let Ok(()) = file.write(text.as_bytes()) {
//         info!("Wrote to sd card");
//     } else {
//         error!("Failed to write");
//     }
//     Ok(())
// }
