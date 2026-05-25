pub mod accel;
pub mod display;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::time_tracking;

pub static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, time_tracking::Entry> = Signal::new();
