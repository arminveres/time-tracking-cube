use core::fmt::Write;

use defmt::{Debug2Format, debug, error, info, unwrap};
use embassy_rp::{
    i2c::{Async, I2c},
    peripherals::I2C1,
};
use embassy_time::Timer;
use heapless::String;

use crate::{sd_card::SDCard, time_tracking};

pub async fn log_accel<SPI>(
    mut aclm: adxl345_eh_driver::Driver<I2c<'static, I2C1, Async>>,
    mut sd_card: SDCard<SPI>,
) -> !
where
    SPI: embedded_hal::spi::SpiDevice<u8>,
{
    // A side switch is only committed after the new side has been stable for
    // CONFIRMATION_SECS, preventing accidental knocks from being logged.
    const THRESHOLD_SECS: u64 = 5;
    const CONFIRMATION_SECS: u64 = 3;
    const FILENAME: &str = "entries.csv";
    info!("Running Acceleration Task");

    #[derive(Clone, Copy)]
    enum State {
        Active {
            side: time_tracking::Side,
            start: embassy_time::Instant,
        },
        Transitioning {
            original_side: time_tracking::Side,
            original_start: embassy_time::Instant,
            candidate_side: time_tracking::Side,
            candidate_start: embassy_time::Instant,
        },
    }

    let mut state = State::Active {
        side: time_tracking::Side::One,
        start: embassy_time::Instant::now(),
    };
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

        state = match state {
            State::Active { side, start } => {
                if current_side != side && start.elapsed().as_secs() >= THRESHOLD_SECS {
                    State::Transitioning {
                        original_side: side,
                        original_start: start,
                        candidate_side: current_side,
                        candidate_start: embassy_time::Instant::now(),
                    }
                } else {
                    State::Active { side, start }
                }
            }
            State::Transitioning {
                original_side,
                original_start,
                candidate_side,
                candidate_start,
            } => {
                if current_side == original_side {
                    // Returned to original side — cancel transition, preserve original timer
                    info!("Transition cancelled, back to side {}", original_side as u8);
                    State::Active { side: original_side, start: original_start }
                } else if current_side == candidate_side
                    && candidate_start.elapsed().as_secs() >= CONFIRMATION_SECS
                {
                    // New side confirmed — log the completed entry and commit the switch
                    let entry = time_tracking::Entry::new(
                        original_side,
                        original_start.elapsed().as_secs(),
                    );
                    info!(
                        "logging new entry: side: {}, duration: {}",
                        entry.side, entry.duration
                    );
                    unwrap!(
                        write!(&mut content, "{},{}\n", entry.duration, entry.side),
                        "Writing entry to buffer failed"
                    );
                    match sd_card.write_file(FILENAME, content.as_str()) {
                        Ok(_) => content.clear(),
                        Err(e) => error!("Could not write to file: {:?}", Debug2Format(&e)),
                    }
                    State::Active { side: candidate_side, start: candidate_start }
                } else if current_side != candidate_side {
                    // Yet another side — reset the candidate timer
                    State::Transitioning {
                        original_side,
                        original_start,
                        candidate_side: current_side,
                        candidate_start: embassy_time::Instant::now(),
                    }
                } else {
                    // Still on candidate, waiting for confirmation window
                    State::Transitioning {
                        original_side,
                        original_start,
                        candidate_side,
                        candidate_start,
                    }
                }
            }
        };

        // During transition the display keeps showing the original side and its elapsed time
        let (display_side, display_elapsed) = match state {
            State::Active { side, start } => (side, start.elapsed().as_secs()),
            State::Transitioning { original_side, original_start, .. } => {
                (original_side, original_start.elapsed().as_secs())
            }
        };
        super::DISPLAY_SIGNAL.signal(time_tracking::Entry::new(display_side, display_elapsed));

        Timer::after_secs(1).await;
    }
}
