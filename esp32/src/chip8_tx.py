import socket
import sys

""" Receive a chip8 rom from a socket """
def receive_rom():
    host, port = sys.argv[1].split(":")
    rom_bytes = bytearray()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.connect((host, int(port)))
        rom_size = sock.recv(1)
        rom_bytes = bytearray(sock.recv(int.from_bytes(rom_size, 'big')))
        print(rom_size)
    print(rom_bytes)

    return rom_bytes

def try_index_error(func, msg):
    try:
        func()
    except IndexError:
        print(f"error: {msg}")
        exit(1);

""" 
Simple program to serve up chip8 roms over a socket 

Sends the size of the rom first, then sends the rom
"""
def main():
    host, port = None, None
    rom_file = None
    rom_bytes = None

    try:
        sys.argv[2]
    except IndexError:
        print("error: expects chip8_tx.py host:port file")
        exit(1);

    host, port = sys.argv[1].split(":")
    rom_file = sys.argv[2]

    with open(rom_file, 'rb') as file:
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
                conn.send(length.to_bytes(1, 'big'))
                conn.sendall(rom_bytes)

if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        sys.exit()
