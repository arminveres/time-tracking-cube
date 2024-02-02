#include <cstdint>

#include <pico/stdlib.h>

static constexpr uint8_t LED_PIN = 28;

[[noreturn]] auto main() -> int {
    gpio_init(LED_PIN);
    gpio_set_dir(LED_PIN, GPIO_OUT);

    while (1) {
        gpio_put(LED_PIN, 1);
        sleep_ms(1000);
        gpio_put(LED_PIN, 0);
        sleep_ms(250);
    }
}
