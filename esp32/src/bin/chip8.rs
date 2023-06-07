#![no_std]
#![no_main]

use embedded_hal::serial::{Read as SerialRead, Write as SerialWrite};
use embedded_io::blocking::*;
use embedded_svc::ipv4::Interface;
use embedded_svc::wifi::{ClientConfiguration, Configuration, Wifi};

use esp32_hal::clock::{ClockControl, CpuClock};
use esp32_hal::Rng;
use esp32_hal::{
    peripherals::Peripherals,
    prelude::*,
    uart::{
        config::{Config, DataBits, Parity, StopBits},
        TxRxPins,
    },
    Rtc, Uart, IO,
};
use esp_backtrace as _;
use esp_println::logger::init_logger;
use esp_println::println;
use esp_wifi::wifi::utils::create_network_interface;
use esp_wifi::wifi::WifiMode;
use esp_wifi::wifi_interface::{Socket, WifiStack};
use esp_wifi::{current_millis, initialize, EspWifiInitFor};
use smoltcp::iface::SocketStorage;
use smoltcp::wire::IpAddress;
use smoltcp::wire::Ipv4Address;

const SSID: &str = env!("SSID");
const PASSWORD: &str = env!("PASSWORD");
const RP_SERIAL_CONFIG: Config = Config {
    baudrate: 9600,
    data_bits: DataBits::DataBits8,
    parity: Parity::ParityNone,
    stop_bits: StopBits::STOP1,
};

struct RomGetter<'a, UART>
where
    UART: SerialRead<u8> + SerialWrite<u8>,
{
    rom_buffer: [u8; 4096],
    pub socket: Socket<'a, 'a>,
    uart: UART,
}

impl<'a, UART> RomGetter<'a, UART>
where
    UART: SerialRead<u8> + SerialWrite<u8>,
{
    fn new(uart: UART, socket: Socket<'a, 'a>) -> Self {
        Self {
            rom_buffer: [0; 4096],
            socket,
            uart,
        }
    }

    /// Get list of roms from the
    /// socket server
    fn get_rom_list(&self) {}

    /// Get a rom from the socket server
    fn get_rom(&self) {}

    /// Send a rom to the rp2040
    fn send_rom(&self) {}
}

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);

    let peripherals = Peripherals::take();

    let system = peripherals.DPORT.split();
    let mut peripheral_clock_control = system.peripheral_clock_control;
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock240MHz).freeze();
    let mut rtc = Rtc::new(peripherals.RTC_CNTL);
    rtc.rwdt.disable();

    let timer = esp32_hal::timer::TimerGroup::new(
        peripherals.TIMG1,
        &clocks,
        &mut peripheral_clock_control,
    )
    .timer0;
    let init = initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio19.into_push_pull_output(),
        io.pins.gpio22.into_floating_input(),
    );

    let rp_serial = Uart::new_with_config(
        peripherals.UART1,
        Some(RP_SERIAL_CONFIG),
        Some(pins),
        &clocks,
        &mut peripheral_clock_control,
    );

    let local_address = core::env!("ADDRESS");
    let mut parts = local_address.split(':');
    let ip = parts.next().unwrap();
    let port = parts.next().unwrap().parse::<u16>().unwrap();
    let mut local_ip: [u8; 4] = [0; 4];
    let mut index = 0;
    for part in ip.split('.') {
        local_ip[index] = part.parse::<u8>().unwrap();
        index += 1;
    }

    let (wifi, _) = peripherals.RADIO.split();
    let mut socket_set_entries: [SocketStorage; 3] = Default::default();
    let (iface, device, mut controller, sockets) =
        create_network_interface(&init, wifi, WifiMode::Sta, &mut socket_set_entries);
    let wifi_stack = WifiStack::new(iface, device, sockets, current_millis);

    let client_config = Configuration::Client(ClientConfiguration {
        ssid: SSID.into(),
        password: PASSWORD.into(),
        ..Default::default()
    });
    let res = controller.set_configuration(&client_config);
    println!("wifi_set_configuration returned {:?}\n\r", res);

    controller.start().unwrap();
    println!("is wifi started: {:?}\n\r", controller.is_started());

    //println!("capabilities: {:?}\n\r", controller.get_capabilities());

    // wait to get connected
    println!("wifi_connect {:?}\n\r", controller.connect());
    println!("Wait to get connected\n\r");
    loop {
        let res = controller.is_connected();
        match res {
            Ok(connected) => {
                if connected {
                    break;
                }
            }
            Err(err) => {
                println!("Error: {:?}\n\r", err);
                println!("Retrying...\n\r");
                _ = controller.connect();
            }
        }
    }
    println!("is_connected: {:?}\n\r", controller.is_connected());

    // wait for getting an ip address
    println!("Wait to get an ip address\n\r");
    loop {
        wifi_stack.work();

        if wifi_stack.is_iface_up() {
            println!("got ip {:?}\n\r", wifi_stack.get_ip_info());
            break;
        }
    }

    let mut rx_buffer = [0u8; 1536];
    let mut tx_buffer = [0u8; 1536];
    let mut socket = wifi_stack.get_socket(&mut rx_buffer, &mut tx_buffer);

    socket.work();
    socket
        .open(
            IpAddress::Ipv4(Ipv4Address::new(
                local_ip[0],
                local_ip[1],
                local_ip[2],
                local_ip[3],
            )),
            port,
        )
        .unwrap();

    let mut rom_getter = RomGetter::new(rp_serial, socket);

    //let mut length_read = false;
    //let mut buffer = [0u8; 512];
    //let mut length: usize = 0;
    //loop {
    //    socket.work();
    //    if !length_read {
    //        match socket.read(&mut buffer[0..1]) {
    //            Ok(len) => {
    //                if len > 0 && buffer[0] > 0 {
    //                    length_read = true;
    //                    length = buffer[0] as usize;
    //                    println!("Reading length: {:?}\n\r", length);
    //                }
    //            }
    //            Err(e) => println!("Error reading data from socket: {:?}\n\r", e),
    //        }
    //    } else {
    //        match socket.read(&mut buffer[0..length]) {
    //            Ok(len) if len > 0 => {
    //                println!("Read {} bytes\n\r", len);
    //                println!("Buffer: {:?}\n\r", &buffer[0..length]);
    //                break;
    //            }
    //            Err(e) => println!("Error reading data from socket: {:?}\n\r", e),
    //            _ => {}
    //        }
    //    }
    //}

    rom_getter.socket.disconnect();

    let wait_end = current_millis() + 5 * 1000;
    while current_millis() < wait_end {
        rom_getter.socket.work();
    }
    loop {}
}
