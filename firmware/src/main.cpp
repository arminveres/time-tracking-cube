#include <hardware/flash.h>
#include <pico/platform.h>
#include <pico/stdio.h>
#include <pico/stdlib.h>
#include <pico/time.h>

#include <array>
#include <cstdio>
#include <cstring>
#include <vector>

#include "ael/boards/pi_pico/extras/adxl345.hpp"
#include "ael/boards/pi_pico/spi.hpp"
#include "ael/boards/pi_pico/timer.hpp"
#include "ael/peripherals/adxl345/registers.hpp"
#include "ael/types.hpp"
#include "pico-oled/display.hpp"
#include "pico-oled/fonts.hpp"
#include "pico-oled/paint.hpp"
#include "pico-oled/paint_enums.hpp"

using namespace ael::types;
using namespace ael::boards::pi_pico::timer;
using namespace ael::boards::pi_pico::spi;
using namespace ael::boards::pi_pico::extras::adxl345;
using namespace ael::peripherals::adxl345;

constexpr u32 RESERVED_FLASH_ADDRESS = 0x101FC000;
constexpr u32 RESERVED_FLASH_SIZE = 0x400;
[[gnu::section("flx_buf")]] u8* const g_flash_buffer =
    reinterpret_cast<u8*>(RESERVED_FLASH_ADDRESS);

/// @brief Clear terminal output
[[gnu::always_inline]] static inline auto clear_term() { std::printf("\x1B[1;1H\x1B[2J"); }

enum class eSides : u8 {
    eSide1 = 1,
    eSide2,
    eSide3,
    eSide4,
    eSide5,
    eSide6,
};

/**
 * @brief Struct that holds an entry value for the tasks.
 * @note Expected size is 7 bytes = 4 + (1 + 1 + 1)
 */
struct [[gnu::packed]] TimeEntry {
    static constexpr auto ser_size = sizeof(eSides) + sizeof(TimeStamp);
    using SerBuf = std::array<u8, ser_size>;
    eSides side;
    TimeStamp ts;

    auto serialize() const -> SerBuf {
        SerBuf ser_buf = {static_cast<u8>(side), ts.hours, ts.minutes, ts.seconds};
        return ser_buf;
    }
};

/**
 * @brief Serializes a data struct to bytes.
 * @warn Each call instantiates its own buffer at compiletime, so the code gets
 * larger. Keep in mind.
 */
template <class T>
auto serialize(const T& entry) -> u8* {
    static u8 serbuf[sizeof(T)];
    std::memcpy(serbuf, reinterpret_cast<const u8*>(&entry), sizeof(T));
    return serbuf;
}

using TSVec = std::vector<TimeEntry>;
// using TSVec = std::vector<TimeEntry::SerBuf>;

[[noreturn]] int main() {
    /// ===========================================================================================
    /// Main Initializations
    /// ===========================================================================================
    stdio_init_all();

    auto spi = SPI(eInstSPI::SPI_0, 17, 18, 19, 16, 1'000'000);

#if 1
    // TODO(aver): refactor, meaning that the oled module lands in the ael-cpp repo
    /// General GPIO config
    gpio_init(EPD_RST_PIN);
    gpio_set_dir(EPD_RST_PIN, GPIO_OUT);

    gpio_init(EPD_DC_PIN);
    gpio_set_dir(EPD_DC_PIN, GPIO_OUT);

    gpio_init(EPD_CS_PIN);
    gpio_set_dir(EPD_CS_PIN, GPIO_OUT);

    gpio_set_dir(EPD_CS_PIN, GPIO_OUT);

    gpio_put(EPD_CS_PIN, 1);
    gpio_put(EPD_DC_PIN, 0);

    auto& display = pico_oled::Display<pico_oled::eConType::SPI>().clear();
    // auto display = pico_oled::Display<pico_oled::eConType::SPI>();
    // display.clear();

    pico_oled::paint::Paint image;

    image.create_image(pico_oled::k_width, pico_oled::k_height,
                       pico_oled::paint::eRotation::eROTATE_0,
                       pico_oled::paint::eImageColors::WHITE);

    image.clear_color(pico_oled::paint::eImageColors::BLACK);
#endif
    /// ===========================================================================================
    /// Main Initializations END
    /// ===========================================================================================

    // FIXME(aver): set sampling rate as a parameter
    // auto accm = LIS3DH(spi,
    // ael::peripherals::lis3dh::reg_ctrl1::RATE_100_HZ);

    auto accm = ADXL345(spi);
    auto _ = accm.reg_read(ADXL345_REG_DEVID);


    auto id = accm.reg_read(ADXL345_REG_DEVID);
    while (id != ADXL345::ADXL345_ID) {
        std::printf("ERROR: Expected SPI Address: 0x%x, got: 0x%x\n", ADXL345::ADXL345_ID, id);
        sleep_ms(1000);
    }

    // Neat error handling
    if (const auto result = accm.init(); not result) {
        std::printf("ERROR: Encountered Configuration error\n");
        while (true) sleep_ms(1'000);
    }

    /// Threshold
    constexpr auto ths = 127;
    auto timer1 = Timer<eTimeType::eSec>().start();
    auto old_side = eSides::eSide1;
    auto new_side = eSides::eSide1;
    auto entries = TSVec();

    for (;;) {
        const auto accel = accm.read_accel();
        // clear_term();
        printf("CON: x: %03d, y: %03d, z: %03d\n", accel.x, accel.y, accel.z);

#if 1
        {
            image.clear_color(pico_oled::paint::eImageColors::BLACK);
            sleep_ms(100);
            image.draw_en_string(0, 10, "X: ", pico_oled::font::Font8,
                                 pico_oled::paint::eImageColors::WHITE,
                                 pico_oled::paint::eImageColors::WHITE);
            image.draw_number(10, 10, accel.x, pico_oled::font::Font8, 0,
                              pico_oled::paint::eImageColors::WHITE,
                              pico_oled::paint::eImageColors::WHITE);

            image.draw_en_string(0, 30, "Y: ", pico_oled::font::Font8,
                                 pico_oled::paint::eImageColors::WHITE,
                                 pico_oled::paint::eImageColors::WHITE);
            image.draw_number(10, 30, accel.y, pico_oled::font::Font8, 0,
                              pico_oled::paint::eImageColors::WHITE,
                              pico_oled::paint::eImageColors::WHITE);

            image.draw_en_string(0, 50, "Z: ", pico_oled::font::Font8,
                                 pico_oled::paint::eImageColors::WHITE,
                                 pico_oled::paint::eImageColors::WHITE);
            image.draw_number(10, 50, accel.z, pico_oled::font::Font8, 0,
                              pico_oled::paint::eImageColors::WHITE,
                              pico_oled::paint::eImageColors::WHITE);

            // 3.Show image on page1
            display.show(image.get_image());
        }

#endif  // 0

        if (accel.z > ths) {
            new_side = eSides::eSide1;
            // printf("flat, side 1\n");
        }
        if (accel.z < -ths) {
            new_side = eSides::eSide2;
            // printf("flat, side 2\n");
        }
        if (accel.x > ths) {
            new_side = eSides::eSide3;
            // printf("on short edge, side 3\n");
        }
        if (accel.x < -ths) {
            new_side = eSides::eSide4;
            // printf("on short edge, side 4\n");
        }
        if (accel.y > ths) {
            new_side = eSides::eSide5;
            // printf("on long edge, side 5\n");
        }
        if (accel.y < -ths) {
            new_side = eSides::eSide6;
            // printf("on long edge, side 6\n");
        }

        // Put entry
        if (old_side == new_side) {
            continue;
        }
        // 15 seconds is the least we expect on a task
        if (auto timestamp = timer1.get_now_triple(); timestamp.seconds < 15) {
            printf("%d\n", timestamp.seconds);
            continue;
        }

        // clear output for new one
        clear_term();

        const auto entry = TimeEntry{
            .side = new_side,
            .ts = timer1.stop_with_triple(),
        };

        entries.emplace_back(entry);

        printf("Time spent on side %d: [ %02d:%02d:%02d ]\n", static_cast<int>(old_side),
               entry.ts.hours, entry.ts.minutes, entry.ts.seconds);
        printf("Changed side to: %d\n", static_cast<int>(new_side));

        old_side = new_side;
        timer1.start();

        // FIXME(aver): sleep should be adjusted to the sampling rate of lis3dh
        sleep_ms(100);
    }
}
