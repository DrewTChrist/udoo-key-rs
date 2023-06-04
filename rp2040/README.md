# rp2040

## Programs

* `src/bin/blinky.rs`
Simple program to blink the green on board led.

* `src/bin/uart.rs`
This program is meant to be paired with the matching uart program in the esp32
directory. Each chip sends a byte to the opposite chip that blinks their leds.
