//! This is one half of an Udoo Key blinky program that uses the serial
//! connection between the esp32 and the rp2040 to communicate.
//!
//! The esp32 sends the first byte (0x1) to the rp2040 triggering the interrupt
//! and toggling the LED connected to the rp2040. The rp2040 responds by with
//! either a 0x1 (yellow) or a 0x2 (blue) and the esp32 responds back. This
//! is continued by the interrupts.
//!
//! This program is meant to be paired with the rp2040 program at
//! rp2040/src/bin/uart.rs be sure to flash both programs
#![no_std]
#![no_main]

use core::cell::RefCell;
use core::fmt::Write;
use critical_section::Mutex;

use esp32_hal::{
    clock::ClockControl,
    gpio::{Gpio32, Gpio33, Output, PushPull, IO},
    interrupt,
    peripherals::{self, Peripherals, TIMG0, UART1, UART2},
    prelude::*,
    timer::{Timer0, TimerGroup},
    uart::{
        config::{Config, DataBits, Parity, StopBits},
        TxRxPins,
    },
    Delay, Rtc, Timer, Uart,
};
use esp_backtrace as _;
use nb::block;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
/// These are the Led options available for the esp32
enum Led {
    /// Yellow led is 1
    Yellow = 0x1,
    /// Blue led is 2
    Blue = 0x2,
    /// Catch all for bad values
    Invalid,
}

impl From<u8> for Led {
    /// Convert u8 to Led type
    fn from(value: u8) -> Self {
        match value {
            x if x == Self::Yellow as u8 => Self::Yellow,
            x if x == Self::Blue as u8 => Self::Blue,
            _ => Self::Invalid,
        }
    }
}

type GlobalSerial<UART> = Mutex<RefCell<Option<Uart<'static, UART>>>>;
type GlobalLed<LED> = Mutex<RefCell<Option<LED>>>;
type Blue = Gpio32<Output<PushPull>>;
type Yellow = Gpio33<Output<PushPull>>;

// Serial connection to the rp2040
static RP_SERIAL: GlobalSerial<UART1> = Mutex::new(RefCell::new(None));
// Serial connection to the external ESP32 uart
static UEXT_UART: GlobalSerial<UART2> = Mutex::new(RefCell::new(None));
// Yellow led on board
static YELLOW_LED: GlobalLed<Yellow> = Mutex::new(RefCell::new(None));
// Blue led on board
static BLUE_LED: GlobalLed<Blue> = Mutex::new(RefCell::new(None));
// Timer to slow blink
static TIMER0: Mutex<RefCell<Option<Timer<Timer0<TIMG0>>>>> = Mutex::new(RefCell::new(None));

/// Configuration for the serial connection
/// between the ESP32 and the RP2040
const RP_SERIAL_CONFIG: Config = Config {
    baudrate: 9600,
    data_bits: DataBits::DataBits8,
    parity: Parity::ParityNone,
    stop_bits: StopBits::STOP1,
};

/// Configuration for the ESP32 external
/// uart to output messages to the console
const UEXT_UART_CONFIG: Config = Config {
    baudrate: 115200,
    data_bits: DataBits::DataBits8,
    parity: Parity::ParityNone,
    stop_bits: StopBits::STOP1,
};

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the TIMG watchdog timer.
    let timer_group0 = TimerGroup::new(
        peripherals.TIMG0,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let timer_group1 = TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut system.peripheral_clock_control,
    );
    let mut wdt0 = timer_group0.wdt;
    let mut wdt1 = timer_group1.wdt;
    // Disable MWDT and RWDT (Watchdog) flash boot protection
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    wdt0.disable();
    wdt1.disable();
    rtc.rwdt.disable();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let mut delay = Delay::new(&clocks);
    let mut blue_led = io.pins.gpio32.into_push_pull_output();
    let mut yellow_led = io.pins.gpio33.into_push_pull_output();
    _ = blue_led.toggle();
    _ = yellow_led.toggle();
    delay.delay_ms(100_u32);
    _ = blue_led.toggle();
    _ = yellow_led.toggle();

    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio19.into_push_pull_output(),
        io.pins.gpio22.into_floating_input(),
    );

    let mut rp_serial = Uart::new_with_config(
        peripherals.UART1,
        Some(RP_SERIAL_CONFIG),
        Some(pins),
        &clocks,
        &mut system.peripheral_clock_control,
    );

    // Set up the uart rx fifo to only hold one byte
    rp_serial.set_rx_fifo_full_threshold(1);

    // Enable interrupt to fire when rx fifo is full (1 byte)
    rp_serial.listen_rx_fifo_full();

    // Send a 0x1 to the rp2040 on startup to kick things off
    // The rp2040 interrupt is wating for rx and will toggle the
    // led when it receives a 0x1
    rp_serial.write(0x1).ok();

    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio13.into_push_pull_output(),
        io.pins.gpio26.into_floating_input(),
    );

    let mut uext_uart = Uart::new_with_config(
        peripherals.UART2,
        Some(UEXT_UART_CONFIG),
        Some(pins),
        &clocks,
        &mut system.peripheral_clock_control,
    );

    _ = writeln!(uext_uart, "UEXT UART Enabled\n\r");

    // Enable interrupt on UART1
    if let Err(e) = interrupt::enable(
        peripherals::Interrupt::UART1,
        interrupt::Priority::Priority2,
    ) {
        _ = writeln!(uext_uart, "Error enabling interrupt: {e:?}\n\r");
    }

    // Move peripherals into their global variables
    let timer0 = timer_group0.timer0;
    critical_section::with(|cs| {
        RP_SERIAL.borrow_ref_mut(cs).replace(rp_serial);
        UEXT_UART.borrow_ref_mut(cs).replace(uext_uart);
        YELLOW_LED.borrow_ref_mut(cs).replace(yellow_led);
        BLUE_LED.borrow_ref_mut(cs).replace(blue_led);
        TIMER0.borrow_ref_mut(cs).replace(timer0);
    });

    loop {}
}

// Interrupt should be triggered when the rx fifo has
// received one byte from the rp2040
#[interrupt]
fn UART1() {
    critical_section::with(|cs| {
        let mut rp_serial = RP_SERIAL.borrow_ref_mut(cs);
        let rp_serial = rp_serial.as_mut();
        let mut uext_uart = UEXT_UART.borrow_ref_mut(cs);
        let uext_uart = uext_uart.as_mut();
        let mut yellow_led = YELLOW_LED.borrow_ref_mut(cs);
        let yellow_led = yellow_led.as_mut();
        let mut blue_led = BLUE_LED.borrow_ref_mut(cs);
        let blue_led = blue_led.as_mut();
        let mut timer0 = TIMER0.borrow_ref_mut(cs);
        let timer0 = timer0.as_mut();

        // Make sure peripherals are available before working on them
        if let (Some(serial), Some(ext_uart), Some(yellow), Some(blue), Some(timer)) =
            (rp_serial, uext_uart, yellow_led, blue_led, timer0)
        {
            timer.start(500_u64.millis());
            if let Ok(value) = serial.read() {
                match Led::from(value) {
                    Led::Yellow => {
                        _ = yellow.toggle();
                        _ = writeln!(ext_uart, "UART1 triggered: {:?}\n\r", Led::Yellow);
                        _ = block!(timer.wait());
                        serial.write(0x1).ok();
                    }
                    Led::Blue => {
                        _ = blue.toggle();
                        _ = writeln!(ext_uart, "UART1 triggered: {:?}\n\r", Led::Blue);
                        _ = block!(timer.wait());
                        serial.write(0x1).ok();
                    }
                    Led::Invalid => { /* Not a valid led */ }
                }
            }
            _ = serial.flush();
            serial.reset_rx_fifo_full_interrupt();
        }
    });
}
