#![no_std]
#![no_main]

use core::cell::RefCell;
use cortex_m_rt::entry;
use critical_section::Mutex;
use defmt_rtt as _;
use embedded_graphics::{pixelcolor::Rgb565, prelude::*};
use embedded_hal::timer::{Cancel, CountDown};
use fugit::ExtU32;
use fugit::RateExtU32;
use panic_probe as _;

use rp2040_hal as hal;

use hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::{dynpin::DynPin, FunctionUart, Pins},
    pac::{self, interrupt},
    rosc::RingOscillator,
    sio::Sio,
    timer::Timer,
    uart::{DataBits, Enabled, StopBits, UartConfig, UartPeripheral},
    watchdog::Watchdog,
};

use chip8::fonts;
use chip8::keypad::KeyPad;
use chip8::Chip8;

enum Rom<'a> {
    Bytes(&'a [u8]),
}

impl<'a> Rom<'a> {
    fn bytes(&self) -> &'a [u8] {
        let Rom::Bytes(b) = self;
        b
    }
}

/// Mock display because this example
/// is more about using the bytes sent
/// from the esp32
struct Chip8MockDisplay;
/// Mock Error
struct DisplayError;

impl embedded_graphics::geometry::OriginDimensions for Chip8MockDisplay {
    fn size(&self) -> Size {
        todo!()
    }
}

impl DrawTarget for Chip8MockDisplay {
    type Color = Rgb565;
    type Error = DisplayError;
    fn draw_iter<I>(
        &mut self,
        _: I,
    ) -> Result<(), <Self as embedded_graphics::draw_target::DrawTarget>::Error> {
        todo!()
    }
}

type SerialTxPin = Pin<Gpio0, FunctionUart>;
type SerialRxPin = Pin<Gpio1, FunctionUart>;
type Uart = UartPeripheral<Enabled, UART0, (SerialTxPin, SerialRxPin)>;
type GlobalSerial = Mutex<RefCell<Option<Uart>>>;

// Global serial connection to the esp32
static ESP_SERIAL: GlobalSerial = Mutex::new(RefCell::new(None));
static ROM_BUFFER: Mutex<RefCell<Option<[u8; 1024]>>> = Mutex::new(RefCell::new(None));
static ROM_SIZE: Mutex<RefCell<Option<usize>>> = Mutex::new(RefCell::new(None));
// Some(()) for new rom available, None for no new rom available
static ROM_LOAD_STATE: Mutex<RefCell<Option<()>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut countdown = timer.count_down();

    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = Pins::new(
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

    let display = Chip8MockDisplay;

    let keypad = KeyPad::<DynPin, DynPin>::new(
        [
            pins.gpio19.into_push_pull_output().into(),
            pins.gpio18.into_push_pull_output().into(),
            pins.gpio17.into_push_pull_output().into(),
            pins.gpio16.into_push_pull_output().into(),
        ],
        [
            pins.gpio26.into_pull_up_input().into(),
            pins.gpio22.into_pull_up_input().into(),
            pins.gpio21.into_pull_up_input().into(),
            pins.gpio20.into_pull_up_input().into(),
        ],
    );

    let rosc = RingOscillator::new(pac.ROSC);
    let rng = rosc.initialize();
    let mut chip8 = Chip8::new(display, keypad, rng, delay);
    let rom = Rom::Bytes(&[]);
    chip8.load_program(rom.bytes());
    chip8.load_font(fonts::DEFAULT);

    // Store items in global variables
    critical_section::with(|cs| {
        ROM_BUFFER.borrow(cs).replace(Some([u8; 1024]));
        ROM_SIZE.borrow(cs).replace(None);
        ROM_LOAD_STATE.borrow(cs).replace(None);
        ESP_SERIAL.borrow(cs).replace(Some(uart));
    });

    // unmask interrupt
    unsafe {
        hal::pac::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }

    loop {
        chip8.tick();
        countdown.start(5_u32.millis());
        let _ = nb::block!(countdown.wait());
        countdown.cancel().unwrap();
        critical_section::with(|cs| {
            let rom_load_state = ROM_LOAD_STATE.borrow(cs);
            let rom_size = ROM_SIZE.borrow(cs);
            if rom_load_state.is_some() {
                let rom_buffer = ROM_BUFFER.borrow(cs);
            }
        });
    }
}

#[interrupt]
fn UART0_IRQ() {
    critical_section::with(|cs| {
        let mut rom_buffer = ROM_BUFFER.borrow_ref_mut(cs);
        let rom_buffer = rom_buffer.as_mut().unwrap();
        let mut rom_size = ROM_SIZE.borrow_ref_mut(cs);
        let rom_size = rom_size.as_mut().unwrap();
        let mut rom_load_state = ROM_LOAD_STATE.borrow_ref_mut(cs);
        let rom_load_state = rom_load_state.as_mut().unwrap();
        let mut esp_serial = ESP_SERIAL.borrow_ref_mut(cs);
        let esp_serial = esp_serial.as_mut().unwrap();
        if esp_serial.uart_is_readable() {
            let mut buff = [0_u8; 2];
            if esp_serial.read_full_blocking(&mut buff).is_ok() {
                let read = (buff[0] << 8 | buff[1]) as usize;
                if read > 0 {
                    *rom_size = Some(read);
                    _ = esp_serial.flush();
                }
            }
        }
    });
}
