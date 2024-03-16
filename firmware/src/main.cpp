#include <pico/platform.h>
#include <pico/stdio.h>
#include <pico/stdlib.h>
#include <pico/time.h>

#include <cstring>
#include <vector>

#include "ael/boards/pi_pico/extras/lis3dh.hpp"
#include "ael/boards/pi_pico/spi.hpp"
#include "ael/boards/pi_pico/timer.hpp"
#include "ael/peripherals/lis3dh/registers.hpp"
#include "ael/types.hpp"

using namespace ael::types;
using namespace ael::boards::pi_pico::timer;
using namespace ael::boards::pi_pico::spi;
using namespace ael::boards::pi_pico::extras::lis3dh;
using namespace ael::peripherals::lis3dh;

enum class eSides {
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
    eSides side;
    TimeStamp ts;
};

/**
 * @brief Serializes a data struct to bytes.
 * @warn Each call instantiates its own buffer at compiletime, so the code gets larger. Keep in
 * mind.
 */
template <class T>
auto serialize(const T& entry) -> u8* {
    static u8 serbuf[sizeof(T)];
    std::memcpy(serbuf, reinterpret_cast<const u8*>(&entry), sizeof(T));
    return serbuf;
}

using TSVec = std::vector<TimeEntry>;

[[noreturn]] int main() {
    stdio_init_all();

    auto spi = SPI(eInstSPI::SPI_0, 17, 18, 19, 16, 1'000'000);
    // FIXME(aver): set sampling rate as a parameter
    auto accm = LIS3DH(spi);

    (void)accm.reg_read(reg_addr::WHO_AM_I);

    const auto id = accm.reg_read(reg_addr::WHO_AM_I);
    if (id == LIS3DH::LIS3DH_ID) {
        printf("SPI address 0x%x\n", id);
    } else {
        printf("ERROR: Expected Address 0x%x\n", reg_addr::WHO_AM_I);
        while (true) {
            sleep_us(10'000);
        }
    }

    // neat error handling
    if (const auto err = accm.init(); err) {
        printf("ERROR: Ecountered error\n");
        while (true) sleep_us(10000);
    }

    /// Threshold
    constexpr auto ths = 127;
    auto timer1 = Timer<eTimeType::eSec>().start();
    auto old_side = eSides::eSide1;
    auto new_side = eSides::eSide1;
    auto entries = TSVec();

    while (true) {
        // skip if we don't have new data
        if (const auto status = accm.reg_read(reg_status::ADDR); not(status & 0x0Fu)) {
            // printf("Status: 0b%08b\n", status);
            continue;
        }

        const auto accel = accm.read_accel();
        // printf("CON: x: %03d, y: %03d, z: %03d\n", accel.x, accel.y, accel.z);

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
        if (auto timestamp = timer1.get_now_triple(); std::get<2>(timestamp) < 15) {
            printf("%d\n", std::get<2>(timestamp));
            continue;
        }

        printf("\e[1;1H\e[2J");

        const auto entry = TimeEntry{
            .side = new_side,
            .ts = timer1.stop_with_triple(),
        };

        entries.emplace_back(entry);

        printf("Time spent on side %d: [ %02d:%02d:%02d ]\n",
               static_cast<int>(old_side),
               std::get<0>(entry.ts),
               std::get<1>(entry.ts),
               std::get<2>(entry.ts));
        printf("Changed side to: %d\n", static_cast<int>(new_side));

        old_side = new_side;
        timer1.start();

        // FIXME(aver): sleep should be adjusted to the sampling rate of lis3dh
        // sleep_ms(100);
    }
}
