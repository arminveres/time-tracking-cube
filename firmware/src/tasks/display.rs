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
        let entry = super::DISPLAY_SIGNAL.wait().await;
        disp.clear();

        let mut buf: String<32> = String::new();
        write!(&mut buf, "Side: {}", entry.side).ok();
        Text::with_baseline(&buf, Point::zero(), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();

        buf.clear();
        let (h, m, s) = (
            entry.duration / 3600,
            (entry.duration % 3600) / 60,
            entry.duration % 60,
        );
        write!(&mut buf, "Time: {:02}:{:02}:{:02}", h, m, s).ok();
        Text::with_baseline(&buf, Point::new(0, 12), text_style, Baseline::Top)
            .draw(&mut disp)
            .unwrap();

        disp.flush().await.unwrap();
    }
}
