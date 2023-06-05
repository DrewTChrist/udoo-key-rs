# esp32

## Programs

* [`src/bin/blinky.rs`](src/bin/blinky.rs)
Simple program to blink the yellow and blue on board leds.

* [`src/bin/uart.rs`](src/bin/uart.rs)
This program is meant to be paired with the matching uart program in the rp2040
directory. Each chip sends a byte to the opposite chip that blinks their leds.
