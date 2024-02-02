cmake_minimum_required(VERSION 3.12)
set(CMAKE_EXPORT_COMPILE_COMMANDS on)

set(PICO_TOOLCHAIN_PATH /opt/gcc-arm-none-eabi/bin)
# NOTE: this must happen before project()
include(libs/pico-sdk/pico_sdk_init.cmake)

project(ttc-firmware C CXX ASM)

set(CMAKE_C_STANDARD 23)
set(CMAKE_CXX_STANDARD 23)

set(CMAKE_CXX_COMPILER /opt/gcc-arm-none-eabi/bin/arm-none-eabi-g++)

set(PICO_BOARD pico_w)
set(PICO_EXAMPLES_PATH ${PROJECT_SOURCE_DIR})

# initialize the Raspberry Pi Pico SDK
pico_sdk_init()

# TODO(aver): add better inclusion of this lib
add_subdirectory(libs/pico-oled src)

include_directories(
    libs/ael/cpp/includes
)

add_compile_options(
    # -Werror
    -Wall
    -Wextra
    -Wconversion
    -Wpedantic
    # note covered by above
    -Wshadow
    -Wundef
    -Wdouble-promotion
    -Os
    -mtune=cortex-m0plus
    -fno-common
    -fstack-usage
    -ffunction-sections
    -fdata-sections
)

add_link_options(
    -Wl,-gc-sections
    -Wl,-print-memory-usage
)

add_executable(${PROJECT_NAME}
    src/main.cpp
)

target_include_directories(${PROJECT_NAME} PRIVATE
    include
)

target_link_libraries(${PROJECT_NAME}
    pico_stdlib
)

pico_add_extra_outputs(${PROJECT_NAME})
