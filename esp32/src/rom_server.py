from enum import Enum
import os
import socket
import sys
from typing import Optional


class Command(Enum):
    RequestRomList = 0x1,
    RequestRom = 0x2


def command_from_int(value: int) -> Command:
    if value == Command.RequestRomList.value[0]:
        return Command.RequestRomList
    elif value == Command.RequestRom.value:
        return Command.RequestRom


class RomServer:
    def __init__(self, host: str, port: int, directory: str):
        self.host = host
        self.port = port
        self.rom_directory = directory
        self.roms = [
            (idx, rom) for (idx, rom) in enumerate(os.listdir(self.rom_directory))
        ]
        self.sock = None


    def receive_command(self, conn) -> (Command, Optional[int]):
        if not conn is None:
            # two byte for command, two for possible rom id
            data = conn.recv(4)
            command = command_from_int(data[3])
            rom_id = data[0] << 8 | data[1]
            match command: 
                case Command.RequestRomList:
                    return (command, None)
                case Command.RequestRom:
                    return (command, rom_id)


    def respond(self, conn, command: Command, arg: Optional[int]) -> None:
        if not conn is None:
            match command:
                case Command.RequestRomList:
                    num_roms = len(self.roms).to_bytes(2, 'big')
                    conn.sendall(num_roms)
                    for rom in self.roms:
                        # send rom id
                        conn.sendall(rom[0].to_bytes(2, 'big'))
                        #print(''.join('{:02x}'.format(x) for x in rom[0].to_bytes(2, 'big')))
                        # send length of file name
                        conn.sendall(len(rom[1]).to_bytes(2, 'big'))
                        #print(''.join('{:02x}'.format(x) for x in len(rom[1]).to_bytes(2, 'big')))
                        # send rom file name
                        conn.sendall(bytes(rom[1], 'utf-8'))
                        #print(''.join('{:02x} '.format(x) for x in bytes(rom[1], 'utf-8')))
                case Command.RequestRom:
                    rom_file = self.roms[arg][1]
                    with open(self.rom_directory + '/' + rom_file, 'rb') as file:
                        rom = file.read()
                        length = len(rom).to_bytes(2, 'big')
                        conn.sendall(length)
                        conn.sendall(rom)


    def run(self) -> None:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            self.sock = sock
            self.sock.bind((self.host, self.port))
            self.sock.listen()
            while True:
                conn, addr = self.sock.accept()
                with conn:
                    """wait for command"""
                    command = self.receive_command(conn)
                    """ send response """
                    self.respond(conn, command[0], command[1])


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
        print("error: expects rom_server.py host:port file")
        exit(1)

    host, port = sys.argv[1].split(":")
    rom_dir = sys.argv[2]

    server = RomServer(host, int(port), rom_dir)
    server.run()

    #with open(rom_file, "rb") as file:
    #    rom_bytes = bytearray(file.read())

    #with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    #    sock.bind((host, int(port)))
    #    sock.listen()
    #    while True:
    #        conn, addr = sock.accept()
    #        with conn:
    #            print(f"Connection found: {addr}")
    #            length = len(rom_bytes)
    #            print(f"Sending {length} bytes")
    #            conn.send(length.to_bytes(1, "big"))
    #            conn.sendall(rom_bytes)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit()
