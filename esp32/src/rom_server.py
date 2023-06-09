"""
This is a socket server that serves chip8 roms

The socket server accepts two commands, REQUEST_ROM_LIST (0x1) and
REQUEST_ROM (0x2).

Usage:
    python src/rom_server.py localhost:5000 roms
    python src/rom_server.py :4321 roms
"""
import os
import socket
import sys
from typing import Optional, Tuple
from command import Command, command_from_int


class RomServer:
    """Simple socket server for serving up chip8 roms"""

    def __init__(self, host: str, port: int, directory: str):
        self.host = host
        self.port = port
        self.rom_directory = directory
        self.roms = list(enumerate(os.listdir(self.rom_directory)))

    def receive_command(self, conn) -> Tuple[Optional[Command], Optional[int]]:
        """Receives a command from a client"""
        command_arg: Tuple[Optional[Command], Optional[int]] = (None, None)
        if not conn is None:
            # two byte for command, two for possible rom id
            data = conn.recv(4)
            command = command_from_int(data[3])
            rom_id = data[0] << 8 | data[1]
            print(f"Receiving command: {command} rom_id: {rom_id}")
            match command:
                case Command.REQUEST_ROM_LIST:
                    command_arg = (command, None)
                case Command.REQUEST_ROM:
                    command_arg = (command, rom_id)
        return command_arg

    def respond(self, conn, command: Command, arg: Optional[int]) -> None:
        """Responds to the command received from the client"""
        if not conn is None:
            match command:
                case Command.REQUEST_ROM_LIST:
                    print("Requesting rom list")
                    num_roms = len(self.roms).to_bytes(2, "big")
                    conn.sendall(num_roms)
                    for rom in self.roms:
                        # send rom id
                        conn.sendall(rom[0].to_bytes(2, "big"))
                        # print(''.join('{:02x}'.format(x) for x in rom[0].to_bytes(2, 'big')))
                        # send length of file name
                        conn.sendall(len(rom[1]).to_bytes(2, "big"))
                        # print(''.join('{:02x}'.format(x) for x in len(rom[1]).to_bytes(2, 'big')))
                        # send rom file name
                        conn.sendall(bytes(rom[1], "utf-8"))
                        # print(''.join('{:02x} '.format(x) for x in bytes(rom[1], 'utf-8')))
                case Command.REQUEST_ROM:
                    print("Requesting rom")
                    rom_file = self.roms[arg][1]
                    with open(self.rom_directory + "/" + rom_file, "rb") as file:
                        rom_bytes = file.read()
                        length = len(rom_bytes).to_bytes(2, "big")
                        conn.sendall(length)
                        conn.sendall(rom_bytes)

    def run(self) -> None:
        """start the socket server"""
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            sock.bind((self.host, self.port))
            sock.listen()
            while True:
                conn, _ = sock.accept()
                print(conn)
                with conn:
                    # wait for command
                    command = self.receive_command(conn)
                    argument = command[1]
                    # send response
                    self.respond(conn, command[0], argument)


def main():
    """Simple program to serve up chip8 roms over a socket"""
    host, port = None, None

    try:
        sys.argv[2]
    except IndexError:
        print("error: expects rom_server.py host:port file")
        sys.exit(1)

    host, port = sys.argv[1].split(":")
    rom_dir = sys.argv[2]

    server = RomServer(host, int(port), rom_dir)
    server.run()


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit()
