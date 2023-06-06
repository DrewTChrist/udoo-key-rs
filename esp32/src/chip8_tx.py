from enum import Enum
import socket
import sys
from typing import Optional


def receive_rom():
    """ Receive a chip8 rom from a socket """
    host, port = sys.argv[1].split(":")
    rom_bytes = bytearray()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        rom_size = sock.recv(1)
        rom_bytes = bytearray(sock.recv(int.from_bytes(rom_size, "big")))
        print(rom_size)
    print(rom_bytes)

    return rom_bytes

class Command(Enum):
    RequestRomList = 0x1,
    RequestRom = 0x2


class RomServer:
    def __init__(self, host: str, port: int, directory: str):
        self.host = host
        self.port = port
        self.rom_directory = directory
        self.roms = [
            (idx, rom) for (idx, rom) in enumerate(os.listdir(self.rom_directory))
        ]
        self.sock = None

    def receive_command(self) -> (Command, Optional[int]):
        if not self.sock is None:
            # two byte for command, two for possible rom id
            data = self.sock.recv(4)
            command = data[0] << 8 | data[1]
            rom_id = data[2] << data[3]
            match command: 
                case Command.RequestRomList.value:
                    return Command.RequestRomList, None
                case Command.RequestRom.value:
                    return Command.RequestRom, rom_id


    def respond(self, command: Command, arg: Optional[int]) -> None:
        if not self.sock is None:
            match command:
                case Command.RequestRomList:
                    num_roms = len(self.roms).to_bytes(1, 'big')
                    self.sock.send(len(self.roms))
                    for rom in self.roms:
                        # send rom info
                        self.sock.send()
                case Command.RequestRom:
                    rom_file = self.roms[arg + 1][1]
                    with open(rom_file, 'rb') as file:
                        rom = file.read()
                        length = len(rom).to_bytes(2, 'big')
                        self.sock.send(length)
                        self.sock.sendall(rom)


    def run(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            self.sock = sock
            self.sock.bind((host, int(port)))
            self.sock.listen()
            while True:
                conn, addr = self.sock.accept()
                with conn:
                    """wait for command"""
                    command, arg = self.receive_command()
                    """ send response """
                    self.respond(command, arg)


def main():
    """
    Simple program to serve up chip8 roms over a socket

    Sends the size of the rom first, then sends the rom
    """
    host, port = None, None
    rom_file = None
    rom_bytes = None

    try:
        sys.argv[2]
    except IndexError:
        print("error: expects chip8_tx.py host:port file")
        exit(1)

    host, port = sys.argv[1].split(":")
    rom_file = sys.argv[2]

    with open(rom_file, "rb") as file:
        rom_bytes = bytearray(file.read())

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind((host, int(port)))
        sock.listen()
        while True:
            conn, addr = sock.accept()
            with conn:
                print(f"Connection found: {addr}")
                length = len(rom_bytes)
                print(f"Sending {length} bytes")
                conn.send(length.to_bytes(1, "big"))
                conn.sendall(rom_bytes)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit()
