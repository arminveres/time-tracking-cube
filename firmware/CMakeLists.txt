cmake_minimum_required(VERSION 3.12)

include(libs/pico-oled/cmake/pico_sdk_import.cmake)

# use pico sdk defined in ael
set(AEL_USE_PICO_SDK true)
set(AEL_PICO_IMPORT true)
set(AEL_BOARD pico_w)
# set the path through shell env
set(AEL_TOOLCHAIN_PATH $ENV{AEL_TOOLCHAIN_PATH})

# add AEL
add_subdirectory(libs/ael-cpp)

project(ttc-firmware
    LANGUAGES C CXX ASM
    VERSION 0.1.0
)

set(CMAKE_C_STANDARD 23)
set(CMAKE_CXX_STANDARD 23)
set(CMAKE_EXPORT_COMPILE_COMMANDS on)

# initialize the Raspberry Pi Pico SDK
pico_sdk_init()

# TODO(aver): add better inclusion of this lib
add_subdirectory(libs/pico-oled src)

add_executable(${PROJECT_NAME}
    src/main.cpp
)

target_compile_options(${PROJECT_NAME} PRIVATE
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
    -fdiagnostics-color
    -fuse-ld=mold
)

target_link_options(${PROJECT_NAME} PRIVATE
    -Wl,-gc-sections
    -Wl,-print-memory-usage
)

target_include_directories(${PROJECT_NAME} PRIVATE
    include
)

target_link_libraries(${PROJECT_NAME} PRIVATE
    ael_main
    ael_boards_pi_pico
    ael_peripherals_lis3dh

    pico_stdlib
)

pico_set_linker_script(${PROJECT_NAME} ${CMAKE_SOURCE_DIR}/memmap.ld)
pico_add_extra_outputs(${PROJECT_NAME})
pico_enable_stdio_usb(${PROJECT_NAME} 1)
