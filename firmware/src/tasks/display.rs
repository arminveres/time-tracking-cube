use core::fmt::Write;

use embassy_embedded_hal::shared_bus::asynch;
use embassy_rp::{
    gpio::Output,
    peripherals::SPI0,
    spi::{self, Spi},
};
use embassy_sync::blocking_mutex;
use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use heapless::String;
use oled_async::prelude::*;

pub type DisplaySpiDev = asynch::spi::SpiDevice<
    'static,
    blocking_mutex::raw::NoopRawMutex,
    Spi<'static, SPI0, spi::Async>,
    Output<'static>,
>;
pub type DisplayIface = display_interface_spi::SPIInterface<DisplaySpiDev, Output<'static>>;
pub type DisplayType = GraphicsMode<oled_async::displays::sh1107::Sh1107_64_128, DisplayIface>;

pub async fn display_task(mut disp: DisplayType) -> ! {
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    loop {
        let state = super::DISPLAY_SIGNAL.wait().await;
        disp.clear();

        let mut buf: String<32> = String::new();

        // Current side
        write!(&mut buf, "Side: {}", state.current.side).ok();
        Text::with_baseline(&buf, Point::zero(), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();

        buf.clear();
        let (h, m, s) = secs_to_hms(state.current.duration);
        write!(&mut buf, "{:02}:{:02}:{:02}", h, m, s).ok();
        Text::with_baseline(&buf, Point::new(0, 12), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();

        // Previous side (shown below once at least one transition has occurred)
        if let Some(prev) = state.previous {
            buf.clear();
            write!(&mut buf, "Last: {}", prev.side).ok();
            Text::with_baseline(&buf, Point::new(0, 28), text_style, Baseline::Top)
                .draw(&mut disp)
                .unwrap();

            buf.clear();
            let (h, m, s) = secs_to_hms(prev.duration);
            write!(&mut buf, "{:02}:{:02}:{:02}", h, m, s).ok();
            Text::with_baseline(&buf, Point::new(0, 40), text_style, Baseline::Top)
                .draw(&mut disp)
                .unwrap();
        }

        disp.flush().await.unwrap();
    }
}

fn secs_to_hms(secs: u64) -> (u64, u64, u64) {
    (secs / 3600, (secs % 3600) / 60, secs % 60)
}
