# esp32

## Programs

##### [`src/bin/blinky.rs`](src/bin/blinky.rs)

Simple program to blink the yellow and blue on board leds.

To build and flash:
```shell
cargo run --release --bin blinky
```

##### [`src/bin/uart.rs`](src/bin/uart.rs)

This program is meant to be paired with the matching uart program in the rp2040
directory. Each chip sends a byte to the opposite chip that blinks their leds.

To build and flash:
```shell
cargo run --release --bin uart
```

##### [`src/bin/chip8.rs`](src/bin/chip8.rs)

This program creates a socket client on the ESP32 that pulls a Chip8 rom from
a socket server ([`src/rom_server.py`](src/rom_server.py)). It then passes it
on to the RP2040 over the serial connection for the Chip8 interpreter to load.
It expects an environment variable of `ADDRESS`.

To build and flash:
```shell
# replace ipaddress with the ip address of the rom server
ADDRESS=ipaddress:5000 cargo run --release --bin chip8
```

The rom server is located at [`src/rom_server.py`](src/rom_server.py) and can
be started with either of the following commands:
```shell
# :5000 means accept any connection on port 5000
python src/rom_server.py :5000 roms
# or you can use localhost:5000 to only accept local connections
python src/rom_server.py localhost:5000 roms
```

