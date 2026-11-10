# KM780 RGB — state of the work on Windows

## Task 1: recover the keyboard — DONE

`FWUpdate.exe` ran and finished with "Update Finished!!".
The keyboard came back: `USB\VID_28DA&PID_1101&REV_0077`, the name back to
"G.SKILL RIPJAWS KM780 RGB GAMING KEYBOARD", the composite device complete.

## Task 2: capture the protocol — IN PROGRESS

Environment ready on this machine:

  - Wireshark + tshark + USBPcap installed; capturing does NOT need elevation
  - the keyboard is on `\.\USBPcap5`, device address 2
  - official software in `C:\Program Files (x86)\G.SKILL\G.SKILL RIPJAWS KM780 RGB`

How to capture:

    tshark -i \.\USBPcap5 -a duration:300 -w project\captures\NN-name.pcapng

How to extract (the payload is always the LAST 264 bytes of the frame):

    tools/extract.sh captures/NN-name.pcapng > captures/NN-name.txt
    tools/view.sh captures/NN-name.txt <frame>

Captures taken (all with the extracted .txt alongside):

| Capture | What it was | Doc |
|---|---|---|
| `01-open` | opening the app | docs/12 |
| `02-brightness` | brightness levels | docs/13 |
| `03-config` | polling, repeat delay | docs/14 |
| `04-effects` | the 8 effects | docs/15 |
| `04-effects-crash` | the app crashing | docs/15 (note) |
| `05-macros-and-toggles` | macro, G1, G6 | docs/16 |
| `06-key-assignment` | function types | docs/17 |
| `07-final` | color per key, matrix | docs/18 |
| `08-closing` | gradient, shortcuts | docs/19 |
| `09-dump-read` | the read side | docs/20 |
| `10-limits` | long macro, 0x08 | docs/21 |

## PHASE 1 CLOSED

    color       0x09, 3 packets (channel 1=R, 2=G, 3=B), 144 slots, no commit
    brightness  byte 11 of the profile payload (0x02), values 0..4
    map         132 slots, confirmed by 3 independent sources
    profile     layout of the 264 bytes of 0x02/0x82 documented
    init        28 packets, documented

The color tooling for Linux can be written.

## PHASE 2 — in progress

  - DONE   `0x01` = select MODE; `0x02` = full mode layout
  - DONE   effect catalog: the 8 in the UI, with ID and name (docs/15, docs/16)
  - DONE   `0x05` = macro content: `[flag][delay x10ms][HID usage]`
  - DONE   table at byte 120+ = key assignment, 1 byte per slot
  - DONE   function codes: mouse CE-D5, multimedia A5-AB, shortcuts C0-C8
           (C0-C8 contiguous), macro B4, text B8, HID in the low range (docs/17)
  - DONE   Disable writes `0x00`; Default restores the native HID usage
  - DONE   color per key confirmed on the wire: A=slot21, S=slot30, D=slot34
  - DONE   `f6 01 XX YY` = (column, row) of the physical matrix (docs/18)
  - DONE   `0x0A` = page-indexed database: `0x00` names, `0x09` macros/texts,
           `0x1b`/`0x1c`/`0x1d` = one database per mode
  - DONE   polling rate = byte 8 of the mode (docs/14)
  - DONE   gradient: byte 3 = number of stops; a stop = RGB + intensity + position
  - DONE   Windows Shortcuts: C0-C8, nine and contiguous. C3=Super+Tab and
           C4=Alt+Tab exist in the firmware and are not in the official UI (docs/17)
  - DONE   Timer key = coordinate (04, 07); the OSD belongs to the app, not the keyboard
  - DONE   read and write are symmetric: `0x82` returns what `0x02` wrote, byte
           for byte. The tooling's model: read, change one byte, write.
  - DONE   `0x08` = rasterized gradient: 3 channels x 23 steps, 6 bits (docs/21)
  - DONE   the macro delay has an extended field in the flag's nibble (up to ~41s)
  - DONE   a modifier is an ordinary key: Ctrl = HID `0xe0`, no field of its own
  - DONE   each effect keeps its own duration
  - MISSING per-key data of the effects (`0x0B`, `0x0E`, `0x0F`, `0x12`, `0x15`):
           the UI only applies a global effect, so it cannot be exercised through it
  - DONE   `0x85` = read a macro; a virgin bank answers `0xFF`; a re-read right
           after a write returns the old content (docs/09 7b)
  - DONE   `0x05` only writes with all THREE banks. One or two: an ACK and
           nothing. The third packet closes the write, like the 3rd channel of
           `0x09`. The `0x0D` frame is not part of it. Measured on mode 3.
  - DONE   `0xB4` runs the macro in bank 1 of the active mode, and ONLY on a G
           key. G1 and G2 played the same macro (index 0); Timer and LCtrl played
           nothing — and LCtrl went mute as well. The official UI offering macros
           only on the G keys is a firmware limit (docs/09 7b.2).
  - DONE   `0x8A` reads the database: `--base-dump` in the app. Page `0x09` has
           16 subpages and only `0x02`-`0x0F` keep; `0x00` mirrors `0x02` and
           discards writes. The official app writes the macro name into `0x00`,
           so the name was NEVER stored in the keyboard (docs/09 7c.1).
  - DONE   byte 0 of subpage `0a` of the mode pages = the mode index
  - MISSING LED Sleep
  - MISSING byte 108 vs byte 119: two toggles, which is which still unsaid
  - WON'T   Text: the content never goes to the keyboard, only the name.
            A key with a Text function only works with G.SKILL's app running.

## NAMING

The "profile" of `0x01`/`0x02` is really the MODE (M1/M2/M3). The UI's
"Profile" is a PC-side concept and does not exist in the protocol. See
docs/16 section 0.

## FOOTNOTE: THE OFFICIAL APP CRASHES

For the record, not a task: the official app crashes when enabling effects on
mode 1 (`Setting.dll`, integer division by zero at `0x1004aa99`). No packet
goes out before the crash, so it affects neither the protocol nor our captures.
If anyone ever needs to run the app, the workaround is switching to mode 2.

## THE MR/M1/M2/M3 KEYS

They switch the mode and record macros in the keyboard itself, with no
software. Windows is not needed: they can be investigated on Linux by
listening to EP `0x83`. Consequence for the tooling: the active mode changes
underneath the host, so re-read it with `0x81` instead of assuming the last
value written.

## IMPORTANT WARNING

DO NOT sweep unknown commands on the keyboard. That is exactly what put it into
bootloader mode: some byte between `0x96` and `0xAF` is not a read, despite
having bit 7 set. The "bit 7 means read" convention holds for the confirmed
commands, not for arbitrary bytes.
