//! This is one half of an Udoo Key blinky program that uses the serial
//! connection between the esp32 and the rp2040 to communicate.
//!
//! This half waits for an rx on the serial connection via an interrupt. The
//! interrupt checks if a 0x1 was received and the green on-board led is
//! toggled. The rp2040 sends an alternating 0x1 (yellow) or 0x2 (blue) back
//! to the esp32.
//!
//! This program is meant to be paired with the esp32 program at
//! esp32/src/bin/uart.rs be sure to flash both programs
#![no_std]
#![no_main]

use panic_halt as _;

use rp2040_hal as hal;

use core::cell::RefCell;
use cortex_m::prelude::_embedded_hal_serial_Write;
use critical_section::Mutex;
use embedded_hal::digital::v2::ToggleableOutputPin;

use fugit::RateExtU32;
use rp2040_hal::clocks::Clock;

use hal::{
    gpio::{
        bank0::{Gpio0, Gpio1, Gpio25},
        FunctionUart, Pin, PushPullOutput,
    },
    pac::{self, interrupt, UART0},
    uart::{DataBits, Enabled, StopBits, UartConfig, UartPeripheral},
};

#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_GENERIC_03H;

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

type GreenLed = Pin<Gpio25, PushPullOutput>;
type SerialTxPin = Pin<Gpio0, FunctionUart>;
type SerialRxPin = Pin<Gpio1, FunctionUart>;
type Uart = UartPeripheral<Enabled, UART0, (SerialTxPin, SerialRxPin)>;
type GlobalSerial = Mutex<RefCell<Option<Uart>>>;

// Global serial connection to the esp32
static ESP_SERIAL: GlobalSerial = Mutex::new(RefCell::new(None));
// Global green led
static GREEN_LED: Mutex<RefCell<Option<GreenLed>>> = Mutex::new(RefCell::new(None));
// Global esp32 led state
static ESP_LED: Mutex<RefCell<Option<Esp32Led>>> = Mutex::new(RefCell::new(None));

#[repr(u8)]
#[derive(Clone, Copy, PartialEq)]
enum Esp32Led {
    Yellow = 0x1,
    Blue = 0x2,
}

#[rp2040_hal::entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let sio = hal::Sio::new(pac.SIO);

    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let uart_pins = (
        pins.gpio0.into_mode::<FunctionUart>(),
        pins.gpio1.into_mode::<FunctionUart>(),
    );

    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(9600.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // Enable enterrupt on rx
    uart.enable_rx_interrupt();

    // flash for life
    let mut led = pins.gpio25.into_push_pull_output();
    led.toggle().unwrap();
    delay.delay_ms(100);
    led.toggle().unwrap();

    // Store items in global variables
    critical_section::with(|cs| {
        ESP_SERIAL.borrow(cs).replace(Some(uart));
        GREEN_LED.borrow(cs).replace(Some(led));
        ESP_LED.borrow(cs).replace(Some(Esp32Led::Yellow));
    });

    // unmask interrupt
    unsafe {
        hal::pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }

    loop {}
}

#[interrupt]
fn UART0_IRQ() {
    critical_section::with(|cs| {
        let mut esp_serial = ESP_SERIAL.borrow_ref_mut(cs);
        let esp_serial = esp_serial.as_mut().unwrap();
        let mut green_led = GREEN_LED.borrow_ref_mut(cs);
        let green_led = green_led.as_mut().unwrap();
        let mut esp_led = ESP_LED.borrow_ref_mut(cs);
        let esp_led = esp_led.as_mut().unwrap();
        if esp_serial.uart_is_readable() {
            let mut buff = [0_u8; 1];
            if esp_serial.read_full_blocking(&mut buff).is_ok() {
                if buff[0] == 0x1 {
                    _ = green_led.toggle();
                    esp_serial.write_full_blocking(&[*esp_led as u8]);
                    *esp_led = match esp_led {
                        Esp32Led::Yellow => Esp32Led::Blue,
                        Esp32Led::Blue => Esp32Led::Yellow,
                    };
                    _ = esp_serial.flush();
                }
            }
        }
    });
}
