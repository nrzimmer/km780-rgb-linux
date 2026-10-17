# Key -> LED slot map

Extracted from `Setting.dll` (RGB v2.04), table at `0x1023b3b0` in `.data`.
Regenerate with `tools/dump_keymap.py`.

## Table format

8-byte rows:

    dword 0 = Windows virtual-key code, or synthetic ID >= 0x800
    dword 1 = USB HID usage code

**The row's position is the slot index** in the 144-byte payload of command
`0x09`. The order is NOT the keyboard's physical order: it is the hardware's
LED scan order.

There are **132 slots** used out of the 144 available in the payload.

## Table

| Slot | VK | HID | Key |
|---|---|---|---|
| 0 | 2049 | `0x00` | G/zone 0x801 |
| 1 | 2050 | `0x00` | G/zone 0x802 |
| 2 | 2051 | `0x00` | G/zone 0x803 |
| 3 | 2052 | `0x00` | G/zone 0x804 |
| 4 | 2053 | `0x00` | G/zone 0x805 |
| 5 | 2054 | `0x00` | G/zone 0x806 |
| 6 | 2055 | `0xb3` | G/zone 0x807 |
| 7 | 2056 | `0xad` | G/zone 0x808 |
| 8 | 2057 | `0xae` | G/zone 0x809 |
| 9 | 27 | `0x29` | Esc |
| 10 | 192 | `0x35` | ` |
| 11 | 9 | `0x2b` | Tab |
| 12 | 20 | `0x39` | CapsLock |
| 13 | 160 | `0xe1` | LShift |
| 14 | 162 | `0xe0` | LCtrl |
| 15 | 49 | `0x1e` | 1 |
| 16 | 226 | `0x64` | \\| |
| 17 | 91 | `0xe3` | LWin |
| 18 | 112 | `0x3a` | F1 |
| 19 | 50 | `0x1f` | 2 |
| 20 | 81 | `0x14` | Q |
| 21 | 65 | `0x04` | A |
| 22 | 90 | `0x1d` | Z |
| 23 | 164 | `0xe2` | LAlt |
| 24 | 2058 | `0xaf` | G/zone 0x80a |
| 25 | 87 | `0x1a` | W |
| 26 | 88 | `0x1b` | X |
| 27 | 113 | `0x3b` | F2 |
| 28 | 51 | `0x20` | 3 |
| 29 | 69 | `0x08` | E |
| 30 | 83 | `0x16` | S |
| 31 | 67 | `0x06` | C |
| 32 | 25 | `0x91` | VK25 |
| 33 | 114 | `0x3c` | F3 |
| 34 | 68 | `0x07` | D |
| 35 | 0 | `0x8b` | international (hid 0x8b) |
| 36 | 115 | `0x3d` | F4 |
| 37 | 52 | `0x21` | 4 |
| 38 | 82 | `0x15` | R |
| 39 | 70 | `0x09` | F |
| 40 | 86 | `0x19` | V |
| 41 | 53 | `0x22` | 5 |
| 42 | 2059 | `0xb9` | G/zone 0x80b |
| 43 | 2060 | `0xbb` | G/zone 0x80c |
| 44 | 2061 | `0xba` | G/zone 0x80d |
| 45 | 116 | `0x3e` | F5 |
| 46 | 54 | `0x23` | 6 |
| 47 | 84 | `0x17` | T |
| 48 | 71 | `0x0a` | G |
| 49 | 66 | `0x05` | B |
| 50 | 32 | `0x2c` | Space |
| 51 | 55 | `0x24` | 7 |
| 52 | 89 | `0x1c` | Y |
| 53 | 72 | `0x0b` | H |
| 54 | 117 | `0x3f` | F6 |
| 55 | 56 | `0x25` | 8 |
| 56 | 85 | `0x18` | U |
| 57 | 74 | `0x0d` | J |
| 58 | 78 | `0x11` | N |
| 59 | 118 | `0x40` | F7 |
| 60 | 73 | `0x0c` | I |
| 61 | 75 | `0x0e` | K |
| 62 | 77 | `0x10` | M |
| 63 | 119 | `0x41` | F8 |
| 64 | 57 | `0x26` | 9 |
| 65 | 79 | `0x12` | O |
| 66 | 76 | `0x0f` | L |
| 67 | 188 | `0x36` | , |
| 68 | 61 | `0x90` | VK61 |
| 69 | 120 | `0x42` | F9 |
| 70 | 48 | `0x27` | 0 |
| 71 | 28 | `0x8a` | VK28 |
| 72 | 219 | `0x2f` | [ |
| 73 | 189 | `0x2d` | - |
| 74 | 80 | `0x13` | P |
| 75 | 186 | `0x33` | ; |
| 76 | 190 | `0x37` | . |
| 77 | 165 | `0xe6` | RAlt |
| 78 | 191 | `0x38` | / |
| 79 | 92 | `0xe7` | RWin |
| 80 | 0 | `0x88` | international (hid 0x88) |
| 81 | 122 | `0x44` | F11 |
| 82 | 187 | `0x2e` | = |
| 83 | 121 | `0x43` | F10 |
| 84 | 222 | `0x34` | ' |
| 85 | 2067 | `0x87` | G/zone 0x813 |
| 86 | 93 | `0x65` | Menu |
| 87 | 221 | `0x30` | ] |
| 88 | 22 | `0x32` | VK22 |
| 89 | 163 | `0xe4` | RCtrl |
| 90 | 123 | `0x45` | F12 |
| 91 | 0 | `0x89` | international (hid 0x89) |
| 92 | 220 | `0x31` | \ |
| 93 | 13 | `0x28` | Enter |
| 94 | 161 | `0xe5` | RShift |
| 95 | 37 | `0x50` | Left |
| 96 | 8 | `0x2a` | Backspace |
| 97 | 38 | `0x52` | Up |
| 98 | 40 | `0x51` | Down |
| 99 | 44 | `0x46` | PrtSc |
| 100 | 45 | `0x49` | Insert |
| 101 | 46 | `0x4c` | Delete |
| 102 | 145 | `0x47` | ScrLk |
| 103 | 36 | `0x4a` | Home |
| 104 | 35 | `0x4d` | End |
| 105 | 2062 | `0xa5` | G/zone 0x80e |
| 106 | 2063 | `0xa6` | G/zone 0x80f |
| 107 | 2064 | `0xa7` | G/zone 0x810 |
| 108 | 19 | `0x48` | Pause |
| 109 | 33 | `0x4b` | PgUp |
| 110 | 34 | `0x4e` | PgDn |
| 111 | 100 | `0x5c` | Num4 |
| 112 | 97 | `0x59` | Num1 |
| 113 | 144 | `0x53` | NumLock |
| 114 | 103 | `0x5f` | Num7 |
| 115 | 2065 | `0xa9` | G/zone 0x811 |
| 116 | 2066 | `0xa8` | G/zone 0x812 |
| 117 | 106 | `0x55` | Num* |
| 118 | 111 | `0x54` | Num/ |
| 119 | 104 | `0x60` | Num8 |
| 120 | 101 | `0x5d` | Num5 |
| 121 | 98 | `0x5a` | Num2 |
| 122 | 96 | `0x62` | Num0 |
| 123 | 105 | `0x61` | Num9 |
| 124 | 102 | `0x5e` | Num6 |
| 125 | 39 | `0x4f` | Right |
| 126 | 108 | `0x58` | NumSep |
| 127 | 109 | `0x56` | Num- |
| 128 | 107 | `0x57` | Num+ |
| 129 | 0 | `0x85` | international (hid 0x85) |
| 130 | 99 | `0x5b` | Num3 |
| 131 | 110 | `0x63` | Num. |

## Notes

### The 19 special keys, by HID usage range  [I]

The 19 synthetic IDs separate cleanly by the HID usage the factory table
carries, and the split matches the physical keyboard:

| HID | slots | how many | what it is |
|---|---|---|---|
| `0x00` | 0, 1, 2, 3, 4, 5 | 6 | the **G1-G6** keys (no factory function) |
| `b3 ad ae af` | 6, 7, 8, 24 | 4 | **MR, M1, M2, M3** |
| `b9 ba bb` | 42, 43, 44 | 3 | **brightness, timer, and the bag-symbol one** |
| `a5 a6 a7 a8 a9` | 105, 106, 107, 115, 116 | 5 | the five **multimedia** ones |
| `0x87` | 85 | 1 | international (stays dark in modes 1 and 2) |

6 + 4 + 3 + 5 + 1 = 19. Which slot is which within each range (which one is M1
and which is M3, for instance) has not been determined yet --
`tools/km780matrix.py` is the way.

- **19 synthetic IDs** (`0x801`-`0x813`), in slots 0, 1, 2, 3, 4, 5, 6, 7, 8,
  24, 42, 43, 44, 85, 105, 106, 107, 115, 116: these are the G keys, media keys
  and lighting zones, which have no virtual-key code.
- **4 international slots** (slots 35, 80, 91, 129) have `vk=0` with HID usage
  `0x8b, 0x88, 0x89, 0x85`: keys from JIS/ABNT layouts that exist in the matrix
  but not on the US physical layout. Slot 85 (`hid 0x87`) behaves the same way
  in a matrix read, so there are 5 in all.
- **Careful about reading the matrix as proof of absence.** A slot that comes
  back dark in `0x89` only says nobody painted a color into it: the set of dark
  slots changes with the mode. See `23-hardware-validation.md` §6.
- The remaining 12 of the 144 slots are not addressed by this table. [?]
