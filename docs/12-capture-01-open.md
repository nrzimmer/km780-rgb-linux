# Capture `01-open` — init sequence and profile layout

Source: `captures/01-open.pcapng`, USBPcap5, device address 2, FW 77.
Action: open the official `App.exe` and touch nothing.
Extraction: `tools/extract.sh captures/01-open.pcapng`.

Confidence: **[C]** — observed on real hardware.

## 1. How to extract it

USBPcap puts 28 bytes of pseudo-header in front of the frame, plus 8 bytes of
setup on control OUT. The 264-byte payload is always the **last** bytes of the
frame, which makes extraction trivial:

    tools/extract.sh captures/XX.pcapng        # <frame> <t> <DIR> <cmd> <hex>
    tools/view.sh /tmp/XX.txt 186              # hexdump of one payload

Setup confirmed on the wire, identical to what `01-transport.md` predicted:

    21 09 07 03 01 00 08 01
    bmRequestType=0x21  bRequest=0x09  wValue=0x0307  wIndex=0x0001  wLength=264

## 2. The init sequence  [C]

28 SET packets in 3.2 s. Each one gets an 8-byte ACK on EP `0x83` in the format
`00 00 <cmd> 00 00 00 00 00`.

| # | Payload (header) | Reading |
|---|---|---|
| 1 | `07 16 01` | **`0x16` with arg 1** — the first thing the app does |
| 2 | `07 0d 00` | `0x0D` with arg 0 |
| 3-6 | `07 8a 00 00..03` | reads `0x8A` page 0, sub 0..3 |
| 7 | `07 81 00` | reads the current profile -> answer `07 81 01` = profile 1 |
| 8-10 | `07 82 01..03` | reads the information of profiles 1, 2 and 3 |
| 11-25 | `07 8a 1b/1c/1d <00,01,02,09,0a>` | reads `0x8A` pages 0x1B, 0x1C, 0x1D |
| 26 | `07 02 01` + 256 bytes | **rewrites profile 1 unchanged** |
| 27 | `07 0d 01` | `0x0D` with arg 1 |
| 28 | `07 07 00 15` | **`0x07`** — a new command, not on the list |

Facts the capture settles:

- **`0x16` opens the session and `0x0D` frames the configuration.** `0x0D`
  appears with arg 0 before the writes and arg 1 after — it looks like a
  "begin/end" or "suspend/resume the firmware" pair.
- **`0x07` is a command that was not mapped**, sent last, with `buf[2]=0x00`
  and `buf[3]=0x15` and the rest zeroed.
- **Every `0x8A` answer came back pure `0xFF`** (pages 0x00, 0x1B, 0x1C,
  0x1D): an empty macro/assignment database, i.e. unwritten memory.
- The app **reads the profile and writes it back identical** at boot (steps
  8-10 vs 26). That gives the `0x02` layout for free: it is the same as
  `0x82`'s.

## 3. Profile layout — `0x82` (get) / `0x02` (set)  [C]

Full 264-byte payload, of factory profile 1:

    0000  07 82 01 03 00 00 00 00  01 00 01 04 03 03 03 06
    0016  00 00 00 06 00 00 00 06  00 00 00 03 00 00 00 03
    0032  00 00 00 03 00 00 00 03  00 00 00 03 00 00 00 03
    0048  00 00 00 03 00 00 00 03  00 00 00 03 00 00 00 03
    0064  00 00 00 03 00 00 00 03  00 00 00 03 03 03 00 00
    0080  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00
    0096  01 01 01 00 00 00 01 00  00 00 00 01 01 00 ff 00
    0112  00 ff 00 00 ff 00 00 00  00 00 00 00 00 00 b3 ad
    0128  ae 29 35 2b 39 e1 e0 1e  64 e3 3a 1f 14 04 1d e2
    ...
    0240  5d 5a 62 61 5e 4f 58 56  57 85 5b 63 00 00 00 00
    0256  00 00 00 00 00 00 00 00

| Offset | Content |
|---|---|
| 0 | `0x07`, magic |
| 1 | `0x82` (answer) / `0x02` (write) |
| 2 | **profile number, 1-based** (1, 2, 3) — not profile+1 as in `0x09` |
| 3 | `0x03` in the answer, `0x00` in the write — a status/flag field, not data |
| 4-7 | zero |
| 8-119 | 112 bytes of profile configuration (see 3.1) |
| **120-263** | **144-byte table: the HID usage of each LED slot** |

### 3.1 Interesting bytes in the configuration area

- **`109-117` = `00 ff 00 | 00 ff 00 | 00 ff 00`** — three identical RGB
  triples, pure green. Three colors stored in the profile (effect colors?
  zones?). [I]
- `8-79` has a 4-byte structure (`06 00 00 00`, `03 00 00 00`, ...) with a
  small value every 4 bytes: 14 entries of 4 bytes ending in `03 03`. Looks
  like a table of 14 slots. [?]

### 3.2 The LED slot table confirms `10-key-map.md`  [C]

Bytes 120-263 are **one HID usage code per LED slot, in slot order**, exactly
the order `tools/dump_keymap.py` extracted from `Setting.dll`:

| Offset | Slot | Value | Key |
|---|---|---|---|
| 120-125 | 0-5 | `00` x6 | G keys with no HID usage |
| 126 | 6 | `b3` | G/zone 0x807 |
| 129 | 9 | `29` | Esc |
| 130 | 10 | `35` | `` ` `` |
| 135 | 15 | `1e` | 1 |
| ... | ... | ... | matches the table in doc 10 byte for byte |
| 251 | 131 | `63` | Num. |
| 252-263 | 132-143 | `00` x12 | the 12 unused slots |

In other words: **the order of the 144 slots in the color payload is confirmed
on the hardware, by a source independent of the decompilation.** It also
confirms there are 132 useful slots and 12 left over, zeroed.

## 4. Spontaneous messages on EP `0x83`  [I]

Before the app opened, with no software running at all, the keyboard emitted 21
8-byte messages in this format:

    f6 01 XX YY 00 00 00 00

with `XX` in `02..0a` and `YY` in `02..08`. The rhythm matches human typing on
the keyboard during the capture, and the pair `(XX, YY)` looks like a **matrix
(row, column) coordinate**. If confirmed, it gives a direct route to mapping
key -> physical position: just capture known keys, one at a time.

Right after the init a single different message also appeared:

    02 04 01 08 01 00 00 00

## 5. What this changes in the specification

- Gap 8 (command set): **`0x07`** joins it.
- Gap 6 (payloads of `0x01`/`0x02`): **`0x02` solved**, layout above.
- Gap 9 (`buf[2]`): in `0x02`/`0x82` it is the profile number, **1-based**.
- `0x16` and `0x0D` gain a role: opening the session and framing the
  configuration.
- The LED slot order goes from "extracted from the binary" to **confirmed on
  the hardware**.
