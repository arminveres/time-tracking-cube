pub mod accel;
pub mod display;

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

use crate::time_tracking;

#[derive(Clone, Copy)]
pub struct DisplayState {
    pub current: time_tracking::Entry,
    pub previous: Option<time_tracking::Entry>,
}

pub static DISPLAY_SIGNAL: Signal<CriticalSectionRawMutex, DisplayState> = Signal::new();
