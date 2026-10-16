#!/usr/bin/env python3
"""Extracts the key->LED slot map from the KM780 RGB's Setting.dll.

The table lives in .data, at 0x1023b3b0 (v2.04), as 8-byte rows:
    dword 0 = Windows virtual-key code, or synthetic ID >= 0x800
    dword 1 = USB HID usage code
The row's POSITION is the slot index in the 144-byte payload of command 0x09.

Usage: dump_keymap.py <Setting.dll> [table-address]
"""
import sys, struct
sys.path.insert(0, __file__.rsplit('/', 1)[0])
from pescan import sections

VK = {8: 'Backspace', 9: 'Tab', 13: 'Enter', 19: 'Pause', 20: 'CapsLock',
      22: 'VK22', 25: 'VK25', 27: 'Esc', 28: 'VK28', 32: 'Space', 33: 'PgUp',
      34: 'PgDn', 35: 'End', 36: 'Home', 37: 'Left', 38: 'Up', 39: 'Right',
      40: 'Down', 44: 'PrtSc', 45: 'Insert', 46: 'Delete', 61: 'VK61',
      91: 'LWin', 92: 'RWin', 93: 'Menu', 106: 'Num*', 107: 'Num+',
      108: 'NumSep', 109: 'Num-', 110: 'Num.', 111: 'Num/', 144: 'NumLock',
      145: 'ScrLk', 160: 'LShift', 161: 'RShift', 162: 'LCtrl', 163: 'RCtrl',
      164: 'LAlt', 165: 'RAlt', 186: ';', 187: '=', 188: ',', 189: '-',
      190: '.', 191: '/', 192: '`', 219: '[', 220: '\\', 221: ']',
      222: "'", 226: '\\|'}
for i in range(26):
    VK[65 + i] = chr(65 + i)
for i in range(10):
    VK[48 + i] = str(i)
for i in range(12):
    VK[112 + i] = 'F%d' % (i + 1)
for i in range(10):
    VK[96 + i] = 'Num%d' % i


def name(vk, hid):
    if vk >= 0x800:
        return 'G/zone 0x%03x' % vk
    if vk == 0:
        return 'international (hid 0x%02x)' % hid
    return VK.get(vk, 'VK %d' % vk)


def load(path, addr=0x1023b3b0):
    d = open(path, 'rb').read()
    base, secs = sections(d)
    off = None
    for n, sva, vs, rp, rs in secs:
        if base + sva <= addr < base + sva + vs:
            off = rp + (addr - base - sva)
    if off is None:
        sys.exit('address 0x%08x outside the sections' % addr)
    rows = []
    for i in range(256):
        a, b = struct.unpack_from('<II', d, off + i * 8)
        if a == 0 and b == 0 and i > 8:
            break
        rows.append((a, b))
    return rows


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    addr = int(sys.argv[2], 16) if len(sys.argv) > 2 else 0x1023b3b0
    rows = load(sys.argv[1], addr)
    print('%d slots' % len(rows))
    for i, (vk, hid) in enumerate(rows):
        print('%3d  vk=%-5d hid=0x%02x  %s' % (i, vk, hid, name(vk, hid)))
