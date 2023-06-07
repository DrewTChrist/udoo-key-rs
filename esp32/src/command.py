""" Command code shared between rom_server.py and rx.py """
from enum import Enum


class Command(Enum):
    """Enum of available commands"""

    RequestRomList = 0x1
    RequestRom = 0x2


def command_from_int(value: int) -> Command:
    """Convert and int into a Command"""
    command = None
    if value == Command.RequestRomList.value:
        command = Command.RequestRomList
    elif value == Command.RequestRom.value:
        command = Command.RequestRom
    return command
