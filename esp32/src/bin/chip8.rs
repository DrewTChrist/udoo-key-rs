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
use esp_println::{print, println};
use esp_wifi::wifi::utils::create_network_interface;
use esp_wifi::wifi::WifiMode;
use esp_wifi::wifi_interface::{IoError, Socket, WifiStack};
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

#[repr(u8)]
enum Command {
    RequestRomList = 0x1,
    RequestRom = 0x2,
}

#[derive(Clone, Copy, Debug)]
struct RomInfo<const S: usize> {
    pub rom_id: u16,
    pub name: [u8; S],
}

impl<const S: usize> RomInfo<S> {
    fn new() -> Self {
        Self {
            rom_id: 0,
            name: [0; S],
        }
    }

    fn name(&self) -> &str {
        core::str::from_utf8(&self.name).unwrap()
    }
}

struct RomGetter<'a, UART, const R: usize, const N: usize>
where
    UART: SerialRead<u8> + SerialWrite<u8>,
{
    rom_buffer: [u8; 4096],
    rom_size: usize,
    pub roms: [Option<RomInfo<N>>; R],
    pub socket: Socket<'a, 'a>,
    uart: UART,
}

impl<'a, UART, const R: usize, const N: usize> RomGetter<'a, UART, R, N>
where
    UART: SerialRead<u8> + SerialWrite<u8>,
{
    fn new(uart: UART, socket: Socket<'a, 'a>) -> Self {
        Self {
            rom_buffer: [0; 4096],
            rom_size: 0,
            roms: [None; R],
            socket,
            uart,
        }
    }

    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, IoError> {
        self.socket.work();
        self.socket.read(buffer)
    }

    fn write(&mut self, buffer: &[u8]) -> Result<usize, IoError> {
        self.socket.work();
        self.socket.write(buffer)
    }

    /// Get list of roms from the
    /// socket server
    fn get_rom_list(&mut self) {
        let mut num_roms = [0_u8; 2];
        _ = self.write(&[0x0, 0x0, 0x0, Command::RequestRomList as u8]);
        while let Ok(len) = self.read(&mut num_roms) {
            if len > 0 {
                break;
            }
        }
        let num_roms = ((num_roms[0] as u16) << 8 | num_roms[1] as u16) as usize;
        for i in 0..num_roms {
            if i < R {
                let mut rom_id = [0_u8; 2];
                let mut rom_name_size = [0_u8; 2];
                let mut rom_info: RomInfo<N> = RomInfo::new();
                // Read rom id
                _ = self.read(&mut rom_id);
                rom_info.rom_id = (rom_id[0] as u16) << 8 | rom_id[1] as u16;

                // Read size of rom name
                _ = self.read(&mut rom_name_size);
                //println!("rom_name_size: {:?}\n\r", rom_name_size);
                let rom_name_size =
                    ((rom_name_size[0] as u16) << 8 | rom_name_size[1] as u16) as usize;

                // Read rom name
                match rom_info.name.get(0..rom_name_size) {
                    Some(slice) => match self.read(&mut rom_info.name[0..rom_name_size]) {
                        Ok(len) => {
                            println!(
                                "rom_name_size for {:?} is :{}\n\r",
                                rom_id, rom_name_size
                            );
                        }
                        Err(e) => println!("get_rom_list error: {e:?}\n\r"),
                    },
                    None => println!("Invalid slice range\n\r"),
                }
                self.roms[i] = Some(rom_info);
            }
        }
    }

    /// Get a rom from the socket server
    fn get_rom(&mut self, rom_id: u16) {
        println!("get_rom called\n\r");
        let mut rom_size: [u8; 2] = [0; 2];
        _ = self.write(&[
            (rom_id >> 8) as u8,
            (rom_id & 0xff) as u8,
            0x0,
            Command::RequestRom as u8,
        ]);
        loop {
            match self.read(&mut rom_size) {
                Ok(len) if len > 0 => {
                    println!("get_rom length: {}\n\r", len);
                    let rom_size = ((rom_size[0] as usize) << 8 | rom_size[1] as usize) as usize;
                    self.rom_size = rom_size;
                    self.socket.work();
                    _ = self.socket.read(&mut self.rom_buffer[0..self.rom_size]);
                    break;
                }
                Err(e) => {
                    println!("get_rom error: {e:?}\n\r");
                    break;
                }
                _ => {}
            }
        }
    }

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

    let mut rom_getter: RomGetter<_, 8, 32> = RomGetter::new(rp_serial, socket);

    //rom_getter.get_rom_list();
    //for rom in rom_getter.roms {
    //    if rom.is_some() {
    //        let rom = rom.unwrap();
    //        println!("{}: {}\n\r", rom.rom_id, rom.name());
    //    }
    //}

    rom_getter.get_rom(1);

    for (idx, byte) in rom_getter.rom_buffer[0..rom_getter.rom_size]
        .iter()
        .enumerate()
    {
        if idx % 17 == 0 {
            println!("\n\r");
        } else {
            print!("{byte:02x} ");
        }
    }

    rom_getter.socket.disconnect();

    let wait_end = current_millis() + 5 * 1000;
    while current_millis() < wait_end {
        rom_getter.socket.work();
    }
    loop {}
}
