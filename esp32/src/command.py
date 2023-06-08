""" Command code shared between rom_server.py and rx.py """
from enum import Enum
from typing import Optional


class Command(Enum):
    """Enum of available commands"""

    # Request a list of roms from the server
    REQUEST_ROM_LIST = 0x1
    # Request a specific rom by id
    REQUEST_ROM = 0x2


def command_from_int(value: int) -> Optional[Command]:
    """Convert and int into a Command"""
    command = None
    if value == Command.REQUEST_ROM_LIST.value:
        command = Command.REQUEST_ROM_LIST
    elif value == Command.REQUEST_ROM.value:
        command = Command.REQUEST_ROM
    return command
