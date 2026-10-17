# KM780 RGB protocol — specification

> **Status:** Phase 1 closed, Phase 2 open. This is the canonical document;
> the others (`01`, `06`, `08`) are the trail of how we got here.
>
> Confidence convention for each field:
> **[C]** confirmed in the binary or on hardware · **[I]** inferred, not
> verified · **[?]** unknown

## Current scope

**Phase 1 — closed.** Color and intensity are documented and confirmed on
hardware: color in `0x09` (section 6), intensity in byte 11 of `0x02`
(section 6.6), the slot map confirmed by two independent sources
(section 6.5). The color tooling for Linux can be written.

**Phase 2 (now):** work out which messages are left and what each one does —
sections 4, 7, and the commands still without semantics (`0x05`, `0x07`,
`0x0A`, `0x0D`, `0x12`, `0x16`).

The lighting is **per key** [C]: 144 slots of 1 byte per channel, 132 of them
used, one packet per channel.

---

## 1. Transport layer  [C]

USB device `28da:1101`, three HID interfaces:

| Interface | Protocol | Endpoint | Report desc. | Role |
|---|---|---|---|---|
| 0 | 1 (Keyboard) | `0x81` IN | 65 B | HID keyboard — do not touch |
| 1 | 2 (Mouse) | `0x82` IN | 252 B | command output (Feature report ID 7) |
| 2 | 0 | `0x83` IN | 26 B | answer input (8-byte Input) |

Confirmed in two independent places: `lsusb -v` on the user's keyboard, and the
descriptor chain inside the firmware (`0x0000a432`).

### 1.0 Device descriptor  [C]

From firmware FW 77, at `0x0000a432`:

    12 01 0002 00 00 00 40 DA28 0111 0077 01 02 00 01

| Field | Value |
|---|---|
| `bcdUSB` | 2.00 |
| `bMaxPacketSize0` | 64 |
| `idVendor` | `0x28DA` |
| `idProduct` | `0x1101` |
| **`bcdDevice`** | **`0x0077`** — the firmware version (FW 77) |

`bcdDevice` carries the firmware version, so **`lsusb` reveals the keyboard's
firmware version with no software at all**.

### 1.0.1 Three config descriptors  [C]

The firmware carries **three copies** of the configuration descriptor
(`0x0000a444`, `0x0000a498`, `0x0000a4ec`), identical except in the endpoints'
`bInterval`: **1, 4 and 8**. These are the polling rate options
(1000 / 250 / 125 Hz) the UI offers.  [I]

### 1.1 Sending a command  [C]

Control transfer OUT (`HidDevice_SetFeature`):

| Field | Value | Meaning |
|---|---|---|
| `bmRequestType` | `0x21` | CLASS \| INTERFACE \| OUT |
| `bRequest` | `0x09` | SET_REPORT |
| `wValue` | `0x0307` | report type 3 (feature), report ID 7 |
| `wIndex` | `0x0001` | interface 1 |
| `wLength` | `0x0108` | 264 bytes |

### 1.2 Reading  [C]

Control transfer IN (`HidDevice_GetFeature`): `bmRequestType` `0xA1`,
`bRequest` `0x01` (GET_REPORT), same `wValue`/`wIndex`, 264 bytes.

### 1.3 ACK  [C]

The channel is confirmed by interface 2's report descriptor (`0x0000a412` in
FW 77): vendor page `0xFF00`, an **8-byte Input** (Usage `0x30`) and an 8-byte
Output (Usage `0x31`), with no Report ID. It is EP `0x83`'s 8-byte channel.

8 bytes on endpoint `0x83` IN (interrupt). Valid when:

    ack[0] == 0x00 && ack[1] == 0x00 && ack[2] == <command sent>

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x00` — status? always zero on success | [I] |
| 1 | 1 | `0x00` — status? always zero on success | [I] |
| 2 | 1 | echo of the command byte | [C] |
| 3 | 1 | zero in the 6,185 captured ACKs | [C] |
| **4** | 1 | **mode** (1–3) in the ACKs of `0x01`/`0x81`; zero in the rest | **[C]** |
| 5–7 | 3 | zero in the 6,185 captured ACKs | [C] |

`0x81` returns the active mode **through here**, in byte 4 of the 8-byte event,
and not through the control IN. See `22-gaps-solved.md` §2.1.

---

## 2. Packet envelope

Every packet is exactly 264 bytes. **The header is 8 bytes**, not 2 as the
abandoned repo suggested:

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x07` — the **HID Report ID** (not a "magic") | [C] |
| 1 | 1 | command | [C] |
| 2 | 1 | **profile index**, 1-based | [C] |
| 3 | 1 | depends on the command — in `0x09` it is the **color channel** | [C] |
| 4 | 1 | depends on the command — page in `0x8B`/`0x92`, `0` in `0x09` | [C] |
| 5 | 1 | a boolean flag, meaning depends on the command | [I] |
| 6–7 | 2 | they exist but are not written on the paths seen | [?] |
| 8 | 256 | payload | [C] |

The 8-byte header is inferred from the offsets of the local variables in
`FUN_100db460`'s decompilation: the buffer starts at `local_20c` (offset 0) and
the payload array is `local_204` (offset 8), with bytes 6 and 7 declared but not
assigned on that path.

### 2.0 Why 264 bytes, and why 8 of header  [C]

Confirmed by the **HID report descriptor inside the firmware itself**
(`0x0000a37a` in FW 77), decoded:

    06 01 ff   Usage Page vendor 0xFF01
    09 01      Usage 1
    a1 01      Collection (Application)
    85 07      Report ID = 7
    15 00      Logical Minimum 0
    26 ff 00   Logical Maximum 255
    09 20      Usage 0x20
    75 08      Report Size  = 8 bits
    96 07 01   Report Count = 0x0107 = 263
    b1 02      FEATURE (Data, Variable, Absolute)

Therefore:

    263 data bytes + 1 Report ID byte = 264 = 0x108    -> matches wLength
    8 header bytes + 256 payload bytes = 264           -> matches the offsets

**The `0x07` byte at offset 0 is not a "manufacturer magic character"**, as the
abandoned repo claims (`KM_START_CHAR`). It is the **HID Report ID**, the same
7 that makes up `wValue = 0x0307` (`0x03` = report type FEATURE, `0x07` = report
ID).

The 256-byte block is not arbitrary: it is what is left of 264 after an 8-byte
header. That explains `FUN_100db460`'s `memset(buf, 0, 0x300)` — 768 = exactly
3 pages of 256 bytes.

### 2.1 Sequence of a transaction  [C]

Taken from `FUN_100d8fc0` (`Setting.dll` RGB v2.04) — the minimal template:

    memset(buf, 0, 0x200);
    buf[0] = 0x07;
    buf[1] = <command>;
    h = get_handle();
    HidDevice_SetFeature(h, buf, 0x108, 0);      // sends  (ordinal 7)
    Sleep(10);
    r = HidDevice_GetFeature(h, buf, 0x108, 0);  // reads  (ordinal 8)
    Sleep(10);

**The official software does NOT read the ACK on the interrupt endpoint
`0x83`.** It does a SetFeature, sleeps, and does a GetFeature into the **same
buffer**. The abandoned repo's approach (waiting for an 8-byte ACK on EP3)
works, but it is not what the manufacturer does — and its ACK wait has an
infinite loop if the ACK never arrives.
Ordinals confirmed by cross-checking `HidDevice.dll`'s export table:
`Ordinal_7` = `HidDevice_SetFeature`, `Ordinal_8` = `HidDevice_GetFeature`.

### 2.2 Timing  [C]

The `Sleep`s are not decorative; the official software always waits:

| Command | After sending | After reading |
|---|---|---|
| `0x81` (simple transaction) | 10 ms | 10 ms |
| `0x8B`, `0x92` (paged) | **50 ms** | 100 ms |

Paged commands use much longer waits — probably because the firmware has to
read/write the onboard flash between pages.

---

## 3. Command table

### 3.1 The set observed in `Setting.dll` v2.04  [C]

Obtained by sweeping `.text` for the pattern `mov byte [buf+0], 0x07` followed
by `mov byte [buf+1], <cmd>`, in the four addressing forms MSVC emitted
(`esp`/`ebp`, disp8/disp32). Tool: `tools/pescan.py`.

| Write | Read | Known name | Sites |
|---|---|---|---|
| `0x01` | `0x81` | Current Profile | `0x100d1cb5` `0x100d1e0f` / `0x100d9014` |
| `0x02` | `0x82` | Profile Information | `0x100d4ebd` / `0x100d90f5` |
| `0x05` | `0x85` | **macro content** — §7b | `0x100d7d14` `0x100d500b` / `0x100d9b05` `0x100da909` |
| `0x09` | `0x89` | **LED Matrix Database** | **`0x100d6c12`** / `0x100db05d` |
| `0x0A` | `0x8A` | — | `0x100dbb94` `0x100dba93` / `0x100dbc5c` |
| `0x0D` | `0x8D`? | — | `0x100d1ecb` / not observed |
| `0x0B` | `0x8B` | **Ripple / Sonic** | `FUN_100d7170` / `0x100db5ec` |
| `0x0E` | 71 B/slot, 3 slots | **gradient per effect slot** — `docs/27` | `0x100d5c69` (`FUN_100d5060`) |
| `0x0F` | same | **same** (Rain family) — `docs/27` | `0x100d5983` (`FUN_100d5060`) |
| `0x12` | `0x92` | **Rain** | `0x100d7642` / `0x100db4ee` |
| `0x15` | **`0x95`** | **Changeling** | `FUN_100d7170` / `0x100db565` |
| `0x16` | `0x96`? | opens the session | `0x100d1f7e` / not observed |

**15 distinct commands across 19 sites.** The abandoned repo's list (4 commands)
covered a little over a quarter of that.

### 3.2 The bit-7 convention  [C]

**Confirmed empirically**, no longer inferred. **Nine pairs**: four seen
answering on the wire (`0x01`/`0x81`, `0x02`/`0x82`, `0x09`/`0x89`,
`0x0A`/`0x8A`), and five confirmed in the binary — `0x05`/`0x85`, plus
`0x0B`/`0x8B`, `0x12`/`0x92` and `0x15`/**`0x95`**, the three emitted by
`FUN_100db460` (`22-gaps-solved.md` §5b). Bit 7 set = a read.

**But the rule does not extend to arbitrary bytes.** The confirmed read space
ends at `0x95`; probing above that is what put the keyboard into bootloader
mode, and `0x96` — which would be the read of `0x16` — is the prime suspect.
See the warning in the README.

### 3.3 Limits of this list  [?]

**It is a floor, not the complete set.** 19 sites with an *immediate* command
byte were found, but there are **37** `push 0x108` in the binary. The difference
is sites that load the command from a register (a command variable at run time),
out of this sweep's reach.

That also explains why `0x08`/`0x88` — which the abandoned repo calls "LED
Effect Database" — **do not appear** here: either they are written via a
register, or v2.04 stopped using them.

How to close it: disassemble the remaining 18 sites, and probe the keyboard with
unobserved `0x8x` values (reads are safe).

## PHASE 1 — color and intensity

## 4. `0x01` / `0x81` — Current Mode  [C]

Selects (`0x01`) or reads (`0x81`) the **active mode** (M1/M2/M3 in the UI).
The UI's "profile" is a PC-side concept and does not exist in the protocol —
see `16-capture-05-macros.md` §0.

    07 01 02 00 00 00 00 00  00 ... (rest zeroed)
          ^^ mode number, 1-based

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 2 | 1 | mode number, 1-based (1–3) | [C] |
| 3–263 | 261 | zero | [C] |

`0x81`'s answer has the same format: `07 81 01` = mode 1 active.
**There are 3 modes.**

---

## 5. `0x02` / `0x82` — Mode Information

> The command's name in the binary is "Profile Information", but what it
> carries is a **mode** (M1/M2/M3). See `16-capture-05-macros.md` §0.

Solved in the `01-open`, `02-brightness` and `05-macros` captures (`docs/12`,
`docs/13`, `docs/16`).
Read and write use **the same 264-byte layout**: the official app reads the
profile with `0x82` and rewrites it unchanged with `0x02` at boot.

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x07`, magic | [C] |
| 1 | 1 | `0x82` (get) / `0x02` (set) | [C] |
| 2 | 1 | **mode number, 1-based** (1, 2, 3) | [C] |
| 3 | 1 | `0x03` in the answer, `0x00` on the write — a status flag | [C] |
| 4–7 | 4 | zero | [C] |
| **8** | 1 | **polling rate: bInterval in ms** (1, 2, 4, 8 = 1000/500/250/125 Hz) | **[C]** |
| 9 | 1 | `00` in everything seen | [?] |
| **10** | 1 | **active effect ID** (0 = static; see section 7.1) | **[C]** |
| **11** | **1** | **intensity / brightness, 0–4** | **[C]** |
| 12–14 | 3 | the effect's **direction** (see 7.2) | [C] |
| 15–71 | 57 | 4-byte records; the 4th of each is the **duration** (see 7.2) | [I] |
| 72–105 | 34 | the rest of the configuration block | [?] |
| 106 | 1 | a flag alternating between the two writes of each action | [C] |
| **107** | 1 | **inert state**: `LightDisable` or `BacklightDisable`, default 1. Written and read back, with no observable effect | **[C]** |
| **108** | 1 | **inert state**: the other of the two, default 1 | **[C]** |
| **109** | 1 | **Windows key lock** (0/1) — the bag-symbol key | **[C]** |
| 110–118 | 9 | **three RGB triples** stored in the profile — pure red `(255,0,0)` on this keyboard, matching mode 1's matrix | **[C]** |
| **119** | 1 | **volume display** (`LightVolumeDisable`), default 0 | **[C]** |
| **120–263** | **144** | **key assignment table**: 1 byte per slot, the HID usage of the assigned function (see 5.1) | **[C]** |

### 5.1 The table at bytes 120–263

One byte per slot, in the same order `dump_keymap.py` extracted from
`Setting.dll`, with `00` in the first 6 (G keys) and `00` in the last 12 (unused
slots). That confirms the slot map on hardware.

But the value is **not fixed**: it is the **HID usage of the function assigned
to the key**. Assigning Ctrl to G6 changed byte 125 (slot 5) from `00` to `e0`
(LeftCtrl); assigning a macro to G1 changed byte 120 (slot 0) to `b4`, which
appears to be the "run macro" code. From the factory each entry coincides with
the key's own HID usage because nothing had been remapped. See
`16-capture-05-macros.md` §3.

### 5.2 The byte is a function code  [C]

The KEY ASSIGNMENT dialog offers nine types per key, and they all write into the
**same byte** of the table. The code space is divided into ranges:

| Range | Meaning |
|---|---|
| `0x00` | **Disable** — the key stops emitting; it is not "use the native one" (`docs/17` §5.1) |
| `0x04`–`0xE7` (low) | HID usage — "Keyboard" type (e.g. `0xE0` = LeftCtrl) |
| `0xA5`–`0xAB` | Multimedia |
| `0xB4` | Macro |
| `0xB8` | Text |
| `0xC0`–`0xC8` | Windows Shortcuts — nine, **contiguous** (`docs/17` §4.1) |
| `0xCE`–`0xD5` | Mouse Function |

The complete map, item by item, is in `17-capture-06-key-assignment.md`. The
high ranges do not collide with the HID usages the keyboard uses (`0x04`–`0x65`,
`0xE0`–`0xE7`), so **one byte is enough** to address any function.

A cross-check: the Multimedia codes (`a5`–`a9`) are exactly the ones the factory
table already carries in the slots of the physical media keys (105–107, 115,
116) in `10-key-map.md`. **Identified one by one**
(`26-special-keys.md`):

    a5 = Stop   a6 = Prev   a7 = Play/Pause   a8 = Next   a9 = Mute

That leaves `aa` and `ab` of the range's seven, and leaves exactly **Volume Up**
and **Volume Down** in the app's dialog — which are the **wheel**, the only
control with no key and no LED slot.

### `aa` and `ab` are the wheel, confirmed on hardware  [C]

> Measured on 2026-10-17. They were `[I]`, deduced by elimination; now they are
> `[C]`, and **the range `0xA5`–`0xAB` is entirely closed**.

Assigning each one to byte 164 (slot 44, the Timer key) with the app in `app/`:

    0xAB  ->  the key turns the volume up
    0xAA  ->  the key turns the volume down

These are the first direct confirmations of codes in this range that **do not
come from the factory table**: `a5`–`a9` were already confirmed because the
physical media keys carry them from the start, but `aa`/`ab` appear on no key at
all and had never shown up in a capture. The deduction was right at both ends.

This also shows that the function byte is **not restricted to what the key is**:
a code that from the factory exists only on the **wheel** — a control with no
key, no `f6` event and no LED slot — works on an ordinary key.

Macro and Text need extra data, which goes into the `0x0A` database page `0x09`
(names) and `0x05` (content), indexed by `buf[3]`. A **Text**'s content never
appeared on the wire — see `docs/17` §7.

### 5.3 Read and write are the same structure  [C]

`0x82` returns **byte for byte** what `0x02` wrote: across the 264 bytes the
only field that differs is byte 3 (`0x00` on the write, `0x03` in the answer).
Confirmed in `20-capture-09-dump-read.md` §1.

Hence the safe operating model for the Linux tooling:

> read the mode with `0x82`, change the desired byte, write it back with `0x02`

There is no need to understand the fields still marked `[?]` in order to touch
the ones marked `[C]`: the unknown ones travel intact through the round-trip.

**There are 3 modes.** The app reads and writes exactly modes 1, 2 and 3.

---

## 6. `0x09` / `0x89` — LED Matrix Database  [C]

**Solved.** Decompiled from `FUN_100d6b80` (`Setting.dll` v2.04; the command's
site is at `0x100d6c12`).

### 6.1 Packet format

| Offset | Size | Field | Value |
|---|---|---|---|
| 0 | 1 | Report ID | `0x07` |
| 1 | 1 | command | `0x09` |
| 2 | 1 | profile | index + 1 (1-based) |
| 3 | 1 | **color channel** | `1`, `2` or `3` |
| 4 | 1 | — | `0x00` |
| 5–7 | 3 | — | `0x00` |
| 8 | 144 | **one byte per LED slot** | intensity 0–255 |
| 152–263 | 112 | unused | `0x00` |

### 6.2 One packet per channel, not per page  [C]

This was the most important open question, and the answer is not key paging:
    buf[3] == 1  ->  the firmware reads the 1st byte of each color triple
    buf[3] == 2  ->  the 2nd byte
    buf[3] == 3  ->  the 3rd byte

**A complete update is 3 packets**, one per channel. Each packet carries 144
bytes = **one byte per LED slot**, which is that channel's intensity on that
LED. That explains why 144 fits comfortably: `8 + 144 = 152` out of 264.

### Channel correspondence  [C]

**Determined by a hardware test** (2026-10-16), on the user's keyboard:

| `buf[3]` | Channel |
|---|---|
| `1` | **R** (red) |
| `2` | **G** (green) |
| `3` | **B** (blue) |

Plain RGB order. The test lit F5 (slot 45) on channel 1 only, F6 (slot 54) on
channel 2 only and F7 (slot 59) on channel 3 only, with `0xFF`, and the visual
result was F5 red, F6 green, F7 blue. Reproduce with `tools/km780_led.py test`.

This could not be found out from the binary: the firmware treats the three
channels symmetrically (`FUN_000081b8` only clamps to 1..3 and copies 144 bytes
per channel), and each one is read from a distinct 4 KB bank in the external SPI
flash (`0x1070`, `0x2070`, `0x3070`) — see `11-firmware.md`. The correspondence
is a convention of the LED hardware.

### `0x09` is authoritative for static color  [C]

In the same test, **every other key went dark**, because the other slots were
sent as zero. There is no effect or brightness layer sitting on top of what
`0x09` writes — if there were, the other keys would have stayed lit.

### 6.3 Write and read are symmetric  [C]

Verified on hardware: after writing with `0x09`, reading with `0x89` on the same
channel returns exactly the bytes sent. The device **echoes the header** in its
answer:

    sent:      07 09 01 02 ...        (profile 1, channel 2)
    returned:  07 89 01 02 ...        (command with bit 7, same profile and channel)

That confirmed on hardware two things that had been inferences: `buf[2]` is the
1-based profile index, and `buf[3]` is the channel.

### 6.4 There is no commit command  [C]

The third packet closes the update. The observed timing:

    for channel in (1, 2, 3):
        build the packet
        SetFeature(buf, 0x108)
        Sleep(20 ms)
        if channel == 3:
            Sleep(100 ms)       # an extra wait after the last one

That is: 20 ms after each channel, plus 100 ms after the third. No separate
apply command.

### 6.5 Key → slot map  [C]

**Extracted.** Table at `0x1023b3b0` in `.data`, 132 slots, every key
identified. It has its own document: **`10-key-map.md`**.
Regenerate with `tools/dump_keymap.py`.

Format: 8-byte rows, `dword0` = Windows virtual-key code (or a synthetic ID
≥ `0x800` for the G keys and zones), `dword1` = USB HID usage code.
**The row's position is the slot index.** The order is not the keyboard's
physical one — it is the hardware's LED scan order.

### 6.6 Intensity — solved  [C]

Intensity is **not in `0x09`**. It is **byte 11 of the profile payload**
(`0x02`), with values **0 to 4** — five levels, the same ones the UI offers.

Confirmed in `captures/02-brightness.pcapng`: walking through all the UI's
levels, the only byte that varies across the 12 `0x02` packets of the active
profile is byte 11, in the sequence `00 00 01 02 03 04`. The `0x09` packets sent
alongside keep their color values intact (`0xFF` on channel R), that is, **the
firmware applies the brightness; the host does not pre-multiply the color**.

`11-firmware.md`'s hypothesis (command `0x0B` as the counterpart of the
brightness event) **was not confirmed**. Nor was its correction: `0x0B` is not
the brightness event either — it only appears as a host→keyboard write, with
Ripple's and Sonic's per-key data. The brightness event is
**`02 04 <mode> 09 <level>`** on EP `0x83`, with the level in the same 0–4 range
as byte 11. See `22-gaps-solved.md` §0.3 and §2.2.  **[C]**

### 6.7 Still open

- what the remaining 12 slots are (144 − 132) — they are always zeroed, so they
  simply do not exist  **[I]**
- besides those, **5 slots within the 132 have no physical key** on the US/ABNT
  layout: 35, 80, 85, 91, 129 — all with a HID usage in the international range
  `0x85`–`0x8b`, and four with `vk = 0`  **[C]**
- slots 16, 32, 68, 71 and 88 were once listed here as having no physical key;
  **the evidence did not support that** and they were removed — see
  `23-hardware-validation.md` §6. A slot that is dark in a matrix read only says
  nobody painted a color into it, and the set of dark slots changes with the
  mode  **[?]**
- does `0x89` (read) return the matrix in the same format — **yes**, confirmed
  in `20-capture-09-dump-read.md` §3  **[C]**

## PHASE 2 — what is left

## 7. Animated effects  [C]

Full detail in `15-capture-04-effects.md`.

### 7.1 The active effect is byte 10 of the mode

| byte 10 | Effect | Helper commands |
|---|---|---|
| `0x00` | **none** (effects off) | — |
| `0x01` | Wave | — |
| `0x02` | Ripple | `0x0B` ×3 channels, `buf[4]`=channel, `buf[5]=0x00` |
| `0x03` | Sonic | `0x0B` ×3 channels, `buf[4]`=channel, `buf[5]=0x01` |
| `0x04` | Breathing | — |
| `0x07` | Changeling | `0x0E` (`04 03 fc`) + `0x15` ×3 channels |
| `0x08` | **Checkpoint** — uses "Transition Point" (number of keys) instead of duration/direction | — |
| `0x06` | **the key lights up on press and never goes out** — dark at rest, the color coming from the gradient slots. Outside the official UI and with no branch in the DLL; firmware only. Real, but useless. `docs/27` §10 | — |
| `0x09` | Cycle | `0x0E` (zeroed) |
| `0x0A` | Rain | `0x0F` (`03 03 fc`) + `0x12` ×3 channels |

Only `0x05` and `0x06` are missing: the UI offers exactly 8 effects, all
mapped.  [?]

> **Corrected in `22-gaps-solved.md` §0.1.** These commands do **not** follow
> `0x09`'s `buf[3] = channel` convention: the channel is in **`buf[4]`** and
> `buf[3]` is another parameter. `0x0B` also uses `buf[5]` to tell Ripple from
> Sonic.

The "×3 channels" commands use `buf[4] = 1, 2, 3` — the same R/G/B triple as
`0x09`, in a different field. They came with a zeroed payload in the capture,
but **the payload is an array of 144 bytes, one per LED slot, at byte 8** — the
same layout as `0x09`, solved by decompilation in `22-gaps-solved.md` §1.  **[C]**

### 7.2 Duration and direction, in the mode's config block

4-byte records starting at byte 12 (15 records, bytes 12–71):

- bytes 12, 13, 14 — **direction**: `0`=Up, `1`=Down, `2`=Left, `3`=Right
  (confirmed against the UI), written identically in all three positions
- the 4th byte of each record — **duration**; the first three (bytes 15, 19, 23)
  store **2×** the UI's value, the rest 1×  [C]
- **each effect keeps its own duration**: switching effect and coming back
  restores the previous value, and the mode payload carries only the active
  effect's (`21-capture-10-limits.md` §4)  **[C]**

### 7.3 `0x08` — the rasterized gradient  [C]

Sent with `buf[2]` = mode. **69 data bytes (8–76) = 3 channels × 23 steps**: the
effect's gradient interpolated step by step, per channel, **cyclically** (the
last step wraps to the first, because the effect loops).

An example with a 2-stop gradient, red at 0 and blue at 100:

    R:  252 252 252 252 252 124 124 124 124 124 124   0 …   0 124 124 124 124 124 124
    G:    0   0   0   0   0   0   0   0   0   0   0   0 …   0   0   0   0   0   0   0
    B:    0   0   0   0   0 124 124 124 124 124 124 252 … 252 124 124 124 124 124 124

G is zero across the 23 steps because there is no green in the gradient; R falls
and rises again at the end, B mirrors it.

Values quantized to **6 bits, stored shifted left by 2**:
`00 38 4c 60 7c 98 c4 e0 fc` = 0, 14, 19, 24, 31, 38, 49, 56, 63.

Detail and validation in `21-capture-10-limits.md` §3.

### 7.4 The effect database: `0x0A` subpages `02`/`05`/`08`  [C]

    00 <duration> <direction> <N>   followed by N stops of 5 bytes

| Field | Meaning |
|---|---|
| byte 1 | duration, in seconds (the UI's value directly) |
| byte 2 | direction: `0`=Up, `1`=Down, `2`=Left, `3`=Right |
| byte 3 | **number of color stops** |
| 5 bytes per stop | `R G B <intensity> <position>`, both 0–100 |

A real example with 4 stops:

    00 03 03 04 | ff 00 00 64 00 | ff ff ff 64 17 | 00 ff 00 16 2f | 00 00 ff 64 64
                   red              white            green            blue
                   int 100 pos 0    int 100 pos 23   int  22 pos 47   int 100 pos 100

Validated by working the Color Controller: changing a stop's color changed only
its RGB; dragging horizontally changed only the position; dragging vertically
changed only the intensity; and adding a fourth stop incremented byte 3.
See `19-capture-08-closing.md` §1.

The stop count also appears in the mode payload, at bytes 75–77.

The effect names sit in subpages `00`/`03`/`06`, in ASCII.

---

## 7b. `0x05` — macro content  [C]

Sent in three banks: `buf[2]` = mode, `buf[3] = 0x00`, `buf[4] = 1, 2, 3`.
Only bank 1 carried data in the capture.

    byte 8–9   header `00 01` — the macro's index/counter  [I]
    byte 10+   3-byte records: [flag] [delay] [HID usage]

- **flag** — bit 7: `1` = press, `0` = release. The **low nibble** carries the
  delay's high bits.
- **delay** — `((flag & 0x0F) << 8) | delay_byte`, in units of **10 ms**.
  A single byte saturates at 2.55 s; with the extension the ceiling goes to
  ~41 s.
- **HID usage** — the key. **Modifiers are ordinary keys** (`0xE0` = LeftCtrl),
  there is no modifier field.

Validated against an `A B C` macro (`docs/16` §1) and against a macro with an
8.65 s pause and another with Ctrl+C (`docs/21` §1 and §2): in all of them,
events and delays match one for one what the app shows on screen.

The official UI **does not allow mouse events inside a macro**, only keys.
A mouse function is a **type of key assignment**, a separate path: see 5.2.
The macro's **name** goes by another path: `0x0A` page `0x09`, in ASCII.

### All three banks are mandatory  [C]

Measured on hardware on 2026-10-17, writing through the app in `app/`. Writing
only bank 1 **does not store**: the keyboard answers ACK and the re-read returns
the previous content. With two banks, the same. With all three, it stores.

| Banks sent | Result |
|---|---|
| 1 | does not take |
| 1, 2 | does not take |
| 1, 2, 3 | **takes** |

Each attempt used a different macro, on mode 3, with the check in a separate
read — see below for why an immediate one is no good.

It is the same shape as `0x09`: there is no commit command, the **third packet
closes the write** (§6.4). The `0x0D` frame the official app wraps around its
writes is **not** part of it: with it and without it the result is the same at
both ends of the matrix.

### An immediate re-read returns the old content  [C]

Right after the write, `0x85` still answers with what was there before. Half a
second later it answers with the new one. Whoever checks a write needs to wait;
without that the check reports a failure that did not happen — which is exactly
what happened here before the matrix above was run.

### 7b.2 `0xB4` only plays on a G key  [C]

Measured on 2026-10-17, with the keyboard playing the macro and an X11 listener
on the other side. `0xB4` is accepted by the assignment table on **any** key,
but it only executes on a G key:

| Key | Who put the `0xB4` there | Result |
|---|---|---|
| G1 | the official app, during the capture | **runs the macro in bank 1 of the active mode** |
| G2 | the official app, during the capture | **runs the same macro** |
| Timer | this app | nothing |
| LCtrl | this app | nothing, and the key goes **mute** |

The `LCtrl` case is what gives the behaviour its shape: the key **stopped
emitting Ctrl**, so the firmware accepted the remap — it simply does not play
the macro. `0xB4` outside a G key is silence, same as `0x00` (`docs/17` §5.1).

Two consequences. The first is that G.SKILL's UI offering macros only on the G
keys **is not an interface choice, it is a firmware limit** — and that is the
first case where the official UI was right about a limit we thought was its own.
The two earlier cases (the `0x06` effect, the `0xC3`/`0xC4` shortcuts) were the
opposite: firmware with more in it than the UI showed.

The second is that **the index does not come from the key**: G1 and G2 played
the *same* macro, the one at index 0. What the old question "which macro does
`0xB4` run" (`docs/16` §3) wanted to know has a practical answer — the one in
bank 1 of the active mode —, even though the field that would hold a per-key
index, if it exists, remains unlocated.

The test is valid because `Timer` **had already worked** with another code: in
`docs/09` §5.2 it received `0xAB` and turned the volume up. The key acts; what
does not act is `0xB4` on it.

### 7b.1 `0x85` — reading the macro  [C]

It had never appeared in a capture: the official app was only ever seen
**writing** macros. The command is in the confirmed read space (which ends at
`0x95`) and appears in the init and in the decompilation, so reading it is safe.
Measured on 2026-10-17:

    07 85 <mode> <index> <bank> 00 00 00

The answer carries the same payload as `0x05`, from byte 8 on. Two things came
out of it:

**The macros from the Windows captures are still in the keyboard.** Mode 1
bank 1 returned the `A … B` macro with the 8.65 s pause from `docs/21` §1, and
mode 2 bank 1 returned the `A B C` from `docs/16` §1 — byte for byte, with the
delays matching. It is an independent validation of the decoder: it read back,
from the hardware, what the capture had shown on the wire months earlier.

**A never-written bank answers `0xFF`.** Mode 3, which the official app never
touched, returned `0xFF` in all three banks — the same convention as `0x8A` in
the init (`docs/20`). `0xFF` means "there is nothing here" and is **different
from zeroed**, which is an empty macro. Without telling the two apart, a virgin
bank decodes as 84 events of `0xFF` with 40.95 s of delay each.

An implementation detail worth recording: the answer comes back **one byte
short** (263 of 264), so the payload's last byte keeps whatever the caller had
in the buffer. Testing "all 256 are `0xFF`" is never true; the header
(`FF FF` against `00 01`) is the test that works.

---

## 7c. `0x0A` / `0x8A` — page-indexed database  [C]

| Page (`buf[2]`) | Content |
|---|---|
| `0x00` | names: `ProfileN`, `Mode1`, `Mode2`, `Mode3` |
| `0x09` | the **macro and text** database (names in ASCII) |
| `0x1b` | database of **mode 1** |
| `0x1c` | database of **mode 2** |
| `0x1d` | database of **mode 3** |

The three mode pages have an identical structure:

| Subpage (`buf[3]`) | Content |
|---|---|
| `00`, `03`, `06` | effect name in ASCII (`Default Wave`) |
| `01`, `04`, `07` | zeroed |
| `02`, `05`, `08` | effect parameters (see 7.4) |
| `0a` | **(id, value) pairs** of settings |

### (id, value) pairs in subpage `0a`

It always appears as a pair, in the middle of a mode write: `buf[3]=0x09` with a
zeroed payload, followed by `buf[3]=0x0a` with data. `buf[2]` is the active
mode's page.

    07 0a 1b 0a 00 00 00 00  00 00 01 <id> <value> 00 ...

| Offset | Field | Confidence |
|---|---|---|
| 2 | mode page (`0x1b`/`0x1c`/`0x1d`) | [C] |
| 3 | subpage — `0x09` (empty) / `0x0a` (data) | [C] |
| 10 | `0x01` — a pair counter? | [I] |
| 11 | **parameter id** — `0x1f` at rest, `0x05` for the repeat delay | [C] |
| 12 | **value** — 0–3 for the repeat delay | [C] |

### 7c.1 Page `0x09`: sixteen subpages, fourteen that keep  [C]

Measured on 2026-10-17 with the `0x8A` reader in the app in `app/`. Three
things, and the third undoes an assumption of this project.

**There are sixteen subpages, `0x00`–`0x0F`.** The capture shows the official
app writing all sixteen in sequence before moving to the next page. And the
whole sequence is what **commits**: writing only the name's subpage, or five of
them, leaves the database zeroed — the name vanishes and `0x8A` returns zero.
The same shape as `0x05`, where the third bank closes the write.

**The first two do not keep.** Writing `S00`…`S15` into all sixteen, the ones
from `0x02` up came back with their own mark; `0x00` came back with `0x02`'s
mark, and `0x01` came back `0xFF`. Writing a mark **only** into `0x00` and
`0x01`, nothing persisted anywhere — so it is the writes that are discarded, not
the read that lies. `0x00` is a mirror of `0x02`.

| Subpage | Behaviour |
|---|---|
| `0x00` | the write is discarded; the read mirrors `0x02` |
| `0x01` | the write is discarded; it always reads `0xFF` |
| `0x02`–`0x0F` | they keep — fourteen names |

**The macro's name was never stored, not even for G.SKILL.** The official app
writes the name into subpage `0x00` precisely (`docs/16` §2) — the discarded
one. And this page was **empty** when the project first read it, after all the
Windows captures. The name the capture shows on the wire lives on the PC, not in
the keyboard.

That closes, with no mystery, why the macro plays with no name at all: the name
was never part of what the keyboard stores. When `0xB4` would not play on an
ordinary key, I assumed the name was the missing piece and started writing it;
the missing piece was something else — the key has to be a G (§7b.2). Writing
the name stayed in the app anyway, now addressed at `0x02 + index`, because it
is the only place that persists and because it gives the user a way to see what
is in the keyboard.

### (id, value) pairs in subpage `0a`: byte 0 is the mode  [C]

Reading the three mode pages, subpage `0a` comes out like this:

    0x1b (mode 1):  00 00 01 1f ...
    0x1c (mode 2):  01 00 01 1f ...
    0x1d (mode 3):  02 00 01 1f ...

The payload's first byte is the **mode index, zero-based**. The capture had only
shown page `0x1b`, where it is `00` and indistinguishable from padding. It
confirms the page↔mode mapping from the other side.

Detail in `14-capture-03-config.md` §3. In the init the same address
`1b/09`, `1b/0a` is **read** with `0x8A` (the answer being pure `0xFF`), which
confirms that `0x0A`/`0x8A` are the write and the read of the same database
indexed by `buf[2]`/`buf[3]`.
---

## 8. Initialization sequence  [C]

Solved in `captures/01-open.pcapng` — detail in `docs/12-capture-01-open.md`.
It is 28 packets in 3.2 s:

    07 16 01                     opens the session
    07 0d 00                     enters configuration mode
    07 8a 00 00..03              reads 0x8A page 0 (all 0xFF)
    07 81 00                     reads the active profile  -> 07 81 01
    07 82 01 / 02 / 03           reads the information of the 3 profiles
    07 8a 1b/1c/1d 00,01,02,09,0a   reads more 0x8A pages (all 0xFF)
    07 02 01 <256 bytes>         rewrites profile 1 unchanged
    07 0d 01                     leaves configuration mode
    07 07 00 15                  command 0x07, unknown until then

`0x0D` appears as a begin/end pair (arg `00` before the writes, `01` after).
`0x16` is always the first message.

> **`0x16` is what turns on the `f6` key reporting** on EP `0x83` — tested on
> hardware, `24-session-0x16.md` §1. That is why no capture shows who turns it
> on: the keyboard already came in turned on from an earlier run of the
> software.
>
> **No command closes the session**, and while it is open the keyboard may stop
> typing. What gives it back is a **USB reset**. Any tool that sends `0x16`
> must guarantee the reset on the way out. §2 to §4 of the same document.

`07 07 00 15` is **not** what turns `f6` on — it was the prime suspect and was
ruled out in the test. It still has no known function. [?]

---

## 9. Known gaps

### Phase 1 — closed

| # | Item | State |
|---|---|---|
| 1 | layout of `0x09`'s payload | **solved** — section 6.1 |
| 2 | paging | **solved** — it is 1 packet per channel, section 6.2 |
| 3 | commit/apply command | **solved** — there is none, section 6.3 |
| 4 | key → slot map | **solved** — `10-key-map.md`, confirmed on hardware |
| 5 | which channel is R, G, B | **solved** — 1=R, 2=G, 3=B, tested on hardware |
| 6 | how intensity/brightness is controlled | **solved** — byte 11 of `0x02`, section 6.6 |

### Phase 2 — what is missing

| # | Item | State |
|---|---|---|
| 1 | effect catalog | **solved** — the 8 in the UI, with ID and name, section 7.1 |
| 2 | macro format (`0x05`) | **solved** — section 7b, write and read measured on hardware |
| 3 | payload of `0x01` / `0x02` | **solved** — sections 4 and 5 |
| 4 | `0x0A`/`0x8A` | **solved** — page-indexed database, section 7c |
| 5 | key assignment table | **solved** — section 5.1 |
| 6 | `0x08`'s payload | **partial** — the gradient's 69 bytes are right, but bytes up to 200 also vary, `22-gaps-solved.md` §3 [?] |
| 7 | `0x0B`, `0x12`, `0x15` | **solved** — 144 bytes per slot at byte 8, `22-gaps-solved.md` §1 [C] |
| 7b | `0x0E`, `0x0F` | **solved** — 3 slots of 71 bytes at 8/79/150, each one `[2 header][3 channels × 23 steps]`, `22-gaps-solved.md` §4 [C] |
| 8 | `0x07`, `0x0D`, `0x16` | role known (opening / frame / end of init), payload never varied [I] |
| 9 | the lighting toggles | **closed.** There are **three**, not two: 107, 108 and 119, and the app has exactly three keys. **119** = volume display **[C]**. **107 and 108 are inert** — written at both values, in the static mode and in the mode with Wave, with a mode switch: no effect. They are stored state, not control. Preserve them through the round-trip and do not touch them — `23-hardware-validation.md` §9 and §12 |
| 10 | bytes 72–107 of the config block | partial: 75–77 = number of color stops |
| 11 | bytes 6–7 of the header | zero in the 6,277 captured packets [C] |
| 11b | `buf[5]` | **carries Ripple/Sonic in `0x0B`**; zero everywhere else [C] |
| 12 | device→host events on EP `0x83` | **three formats mapped**, and the matrix **15×9 mapped key by key** — `25-physical-matrix.md`, 122 keys, validates 100% of the 1,535 captured events. `02 04 M 09 N` = **brightness**; `02 04 M 08 01` = mode/presence; only `02 04 M 04 …` is still [?] |
| 13 | effect IDs `0x05` and `0x06` | they do not exist in the UI [?] |
| 14 | key function codes | **solved** — section 5.2 and `docs/17` |
| 14b | a **Text**'s content | **does not go to the keyboard**, not even on "Save to Device Memory" — `docs/18` §6 [C] |
| 14c | Disable vs Default | **solved** — Disable writes `0x00`, Default restores the native HID usage, `docs/18` §1 |
| 15 | LED Sleep | **parameters known** from the binary's config block — `SleepMode`, `SleepMin` (minutes) and an effect `Kind/Name/ID`. Whether they go to the keyboard is still [?] — `23-hardware-validation.md` §10 |
| 17 | the macro's Playback Option | a dropdown with 4 options, byte not located — `docs/21` §6 |
| 15b | Timer Key | it is a **keyboard key**, not a setting: it emits `f6 01 04 07` — `docs/19` §5 |
| 15c | OSD (On Screen Display) | it belongs to the app, drawn on the PC's screen; it generates no USB traffic |
| 16 | Static Base Color | **not a separate path** — it is the inverse of "Enable Lighting Effect", `docs/18` §7 |

**Command set observed on the wire** (14 write + 4 read).
Only these four `0x8x` answer as reads — see the README's warning about not
sweeping the others:

    0x01 0x02 0x05 0x07 0x08 0x09 0x0A 0x0B 0x0D 0x0E 0x0F 0x12 0x15 0x16
    0x81 0x82 0x89 0x8A

The list in `3.1` is still a floor: only what the UI exercises shows up.

## Sources

- Transport: `legonigel/KM780-RGB-Linux-Driver` (`src/keyboard.cpp`), confirmed
  against the hardware — see `01-transport.md`
- Interfaces and endpoints: `lsusb -v` on the user's machine
- Official binaries: `06-binaries.md`
- Addresses in `Setting.dll`: `08-setting-dll.md`

---

## 10. Still open, in one list

After 10 captures, what is left:

| Item | Why it is still open |
|---|---|
| `0x0B`, `0x0E`, `0x0F`, `0x12`, `0x15` with per-key data | the UI only applies a global effect; **a capture cannot settle it**, only decompilation |
| the byte carrying the macro's **Playback Option** | not located |
| byte 108 vs byte 119 | they are two distinct toggles; which is which is still unsaid |
| bytes 72–107 of the config block | partially mapped (75–77 = number of stops) |
| `buf[5]` and bytes 6–7 of the header | they came back zero in everything captured |
| the `02 04 XX 08` event | third byte = mode; the rest unknown |
| effect IDs `0x05` and `0x06` | they do not exist in the UI |
| LED Sleep | the control was never captured |
| reading page `0x09` (macros) | the app never reads it; safe to test on Linux |
