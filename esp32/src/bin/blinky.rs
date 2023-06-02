#![no_std]
#![no_main]

use hal::{
    clock::ClockControl,
    gpio::IO,
    peripherals::Peripherals,
    prelude::*,
    timer::TimerGroup,
    Delay,
    Rtc,
};
use esp_backtrace as _;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt = timer_group0.wdt;
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);

    // Disable MWDT and RWDT (Watchdog) flash boot protection
    wdt.disable();
    rtc.rwdt.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut yellow_led = io.pins.gpio33.into_push_pull_output();
    let mut blue_led = io.pins.gpio32.into_push_pull_output();

    yellow_led.set_high().unwrap();
    blue_led.set_high().unwrap();

    let mut delay = Delay::new(&clocks);

    loop {
        yellow_led.toggle().unwrap();
        blue_led.toggle().unwrap();
        delay.delay_ms(500u32);
    }
}
