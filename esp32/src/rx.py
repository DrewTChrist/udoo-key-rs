"""
This program can be used to receive a rom from the rom server
in rom_server.py

Usage:
    python src/rx.py ip:port list 
    python src/rx.py ip:port rom 0 
    python src/rx.py ip:port rom 1 
"""
import socket
import sys
from command import Command


def receive_rom(host: str, port: int, rom_id: int):
    """Receive a rom from the server"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        command = bytearray(Command.REQUEST_ROM.value.to_bytes(4, "big"))
        command[0] = 0x0
        command[1] = int(rom_id)
        sock.send(command)
        rom_size = int.from_bytes(sock.recv(2), "big")
        rom_bytes = bytearray(sock.recv(rom_size))

    return (rom_size, rom_bytes)


def receive_rom_list(host: str, port: int) -> list[tuple[int, str]]:
    """Receive the list of roms from the server"""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        sock.send(Command.REQUEST_ROM_LIST.value.to_bytes(4, "big"))
        num_roms = int.from_bytes(sock.recv(2), "big")
        roms = []
        for _ in range(num_roms):
            # read rom id
            rom_id = int.from_bytes(sock.recv(2), "big")
            print(f"rom_id: {rom_id}")
            # read rom file name size
            rom_name_size = int.from_bytes(sock.recv(2), "big")
            print(f"rom_name_size: {rom_name_size}")
            # read rom file name
            rom_name = str(sock.recv(rom_name_size), "utf-8")
            print(f"rom_name: {rom_name}")
            roms.append((rom_id, rom_name))
    return roms

def kinda_pretty(rom: bytes) -> None: 
    row = 0
    for idx, byte in enumerate(rom[1]):
        if not idx % 17 == 0:
            sys.stdout.write(f"{byte:02x} ")
        else:
            sys.stdout.write("\n")
            row += 1


if __name__ == "__main__":
    server_host, server_port = sys.argv[1].split(":")
    if sys.argv[2] == "list":
        receive_rom_list(server_host, int(server_port))
    elif sys.argv[2] == "rom":
        kinda_pretty(receive_rom(server_host, int(server_port), sys.argv[3]))
