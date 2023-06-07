# udoo-key-rs

## Description

The Udoo Key is a development board with an ESP32 and an RP2040.

This repository has some Rust code for the Udoo Key that should run as is. Feel 
free to take a look. There is also an Udoo Key Pro that comes with a microphone
and an IMU. Unless someone with an Udoo Key Pro feels inclined to contribute an
example, there won't be any in this repository.

Read more about the Udoo Key:
* [Kickstarter](https://www.kickstarter.com/projects/udoo/udoo-key-the-4-ai-platform)
* [Udoo Key Docs](http://www.udoo.org/docs-key/Introduction/Introduction.html)

Here are some relevant links:
* [esp-rs Github Organization](https://github.com/esp-rs)
* [rp-rs Github Organization](https://github.com/rp-rs)
* [mpu-6500 rust driver](https://github.com/justdimaa/embedded-sensors)
* [icm-20948 rust driver](https://github.com/Zolkin1/icm20948_driver)

I've extracted some pinouts and key points from the Udoo Key documentation and 
put them in a [Quick Reference](REFERENCE.md) to save time.

## [esp32](esp32/README.md)

Programs that are meant to be run on the esp32 are in the [esp32](esp32/) directory.

## [rp2040](rp2040/README.md)

Programs that are meant to be run on the rp2040 are in the [rp2040](rp2040/) directory.
