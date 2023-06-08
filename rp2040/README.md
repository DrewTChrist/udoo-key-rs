# rp2040

## Programs

##### [`src/bin/blinky.rs`](src/bin/blinky.rs)

Simple program to blink the green on board led.

To build and flash:
```shell
cargo run --release --bin blinky
```

##### [`src/bin/uart.rs`](src/bin/uart.rs)

This program is meant to be paired with the matching uart program in the esp32
directory. Each chip sends a byte to the opposite chip that blinks their leds.

To build and flash:
```shell
cargo run --release --bin uart
```

##### [`src/bin/chip8.rs`](src/bin/chip8.rs)

This program starts by waiting for a serial transfer of a Chip8 rom from the
ESP32. The ESP32 will retrieve the rom from a socket server and then send it to
the RP2040 to execute with the Chip8 interpreter.

To build and flash:
```shell
cargo run --release --bin chip8
```
