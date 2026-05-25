# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Target

RP2040 (Raspberry Pi Pico), Cortex-M0+ — `thumbv6m-none-eabi`, `no_std`, no heap.

## Commands

```bash
# Build
cargo build

# Flash and run (requires probe-rs + connected debugger)
cargo run

# Flash and run with RTT log streaming + GDB server
cargo embed

# Check without linking
cargo check

# Format
cargo fmt

# Lint
cargo clippy
```

`DEFMT_LOG` controls log verbosity (default: `debug`). Override per-run:
```bash
DEFMT_LOG=info cargo run
```

## Architecture

Two async futures run concurrently via `embassy_futures::join` inside the single `#[embassy_executor::main]` task:

- **`log_accel`** — polls the ADXL345 accelerometer every second, detects which of the 6 sides the cube rests on, and logs an `Entry` to the SD card when a side change persists beyond `TRESHOLD_IN_SECONDS`. Signals the display every second with the current side and elapsed time.
- **`display_task`** — waits on `DISPLAY_SIGNAL` and renders side + elapsed time (`hh:mm:ss`) to the OLED.

Inter-task communication uses a single `Signal<CriticalSectionRawMutex, Entry>` (embassy-sync). `Signal` overwrites rather than queues — only the latest value matters.

### Modules

| File | Purpose |
|---|---|
| `src/main.rs` | Hardware init (I2C, SPI0 async, SPI1 blocking), task wiring |
| `src/time_tracking.rs` | `Accel → Side` mapping, `Entry` (side + duration in seconds) |
| `src/sd_card.rs` | `SDCard<SPI>` wrapping `embedded_sdmmc::VolumeManager`; `write_file` appends CSV rows |
| `src/config.rs` | `Wifi` struct for future WiFi config (unused) |
| `src/display.rs` | Stub (empty) |
| `src/accelerometer.rs` | Stub (empty) |

### Hardware pin map

| Peripheral | Bus | Pins |
|---|---|---|
| ADXL345 accelerometer | I2C1 async | SDA=14, SCL=15 |
| SH1107 OLED (64×128) | SPI0 async | SCK=2, MOSI=3, MISO=4, CS=7, DC=6, RST=8 |
| SD card | SPI1 blocking | SCK=10, MOSI=11, MISO=12, CS=13 |

### Key constraints

- **Task arena**: `task-arena-size-16384` in `embassy-executor` features. Increase if the executor panics with "task arena is full".
- **SD card SPI**: initialised at 400 kHz; `embedded_sdmmc` handles card init on first `open_volume` call.
- **Buffers**: `heapless::String<N>` throughout — no alloc. `content` in `log_accel` must be `clear()`ed after each successful SD write to avoid re-appending stale data.
- **Logging**: defmt over RTT (`defmt-rtt`). Types that don't implement `defmt::Format` must be wrapped with `Debug2Format(&val)` at the log call site.
