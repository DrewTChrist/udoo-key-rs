"""
This program can be used to receive a rom from the rom server
in rom_server.py

Usage:
    python src/rx.py ip:port list 
    python src/rx.py ip:port rom 0 
    python src/rx.py ip:port rom 1 
"""
import os
import socket
import sys
from typing import Optional
import sys
from command import Command


def receive_rom(host: str, port: int, rom_id: int):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        command = bytearray(Command.RequestRom.value.to_bytes(4, "big"))
        command[0] = 0x0
        command[1] = int(rom_id)
        sock.send(command)
        rom_size = int.from_bytes(sock.recv(2), "big")
        print(rom_size)
        rom_bytes = bytearray(sock.recv(rom_size))

    return rom_bytes


def receive_rom_list(host: str, port: int):
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        sock.send(Command.RequestRomList.value.to_bytes(4, "big"))
        num_roms = int.from_bytes(sock.recv(2), "big")
        print(f"num_roms: {num_roms}")
        for i in range(num_roms):
            # read rom id
            rom_id = int.from_bytes(sock.recv(2), "big")
            print(f"rom_id: {rom_id}")
            # read rom file name size
            rom_name_size = int.from_bytes(sock.recv(2), "big")
            print(f"rom_name_size: {rom_name_size}")
            # read rom file name
            rom_name = str(sock.recv(rom_name_size), "utf-8")
            print(f"rom_name: {rom_name}")


if __name__ == "__main__":
    host, port = sys.argv[1].split(":")
    if sys.argv[2] == "list":
        receive_rom_list(host, int(port))
    elif sys.argv[2] == "rom":
        print(receive_rom(host, int(port), sys.argv[3]))
