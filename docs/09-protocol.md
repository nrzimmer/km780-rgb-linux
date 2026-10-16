# KM780 RGB protocol — specification

> **Status:** incomplete. This is the canonical document; the others (`01`, `06`,
> `08`) are the trail of how we got here.
>
> Confidence convention for each field:
> **[C]** confirmed in the binary or on hardware · **[I]** inferred, not
> **[?]** unknown

## Current scope

**Phase 1 (now):** fully document the path to **change color and
intensity** — section 6 (`0x09`/`0x89`, LED Matrix Database) and whatever it
requires (paging, commit/apply, key map).

**Phase 2 (later):** survey which messages are left and what each one does —
sections 4, 5, 7 and 8.

The lighting is **per key** [I]: the KM780 RGB has per-key RGB, the
command is called "LED Matrix Database", and ~104 keys × 3 bytes (~312) do not
fit in the 262 payload bytes, which by itself requires paging.

---

## 1. Transport layer  [C]

USB device `28da:1101`, three HID interfaces:

| Interface | Protocol | Endpoint | Report desc. | Papel |
|---|---|---|---|---|
| 0 | 1 (Keyboard) | `0x81` IN | 65 B | HID keyboard — do not touch |
| 1 | 2 (Mouse) | `0x82` IN | 252 B | command output (Feature report ID 7) |
| 2 | 0 | `0x83` IN | 26 B | answer input (8-byte Input) |

Confirmed in two independent places: `lsusb -v` on the user's keyboard, and the
descriptor chain inside the firmware (`0x0000a432`).

### 1.0 Device descriptor  [C]

Do firmware FW 77, em `0x0000a432`:

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

### 1.2 Read  [C]

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
| 3–7 | 5 | **unknown** | [?] |

---

## 2. Packet envelope

Every packet is exactly 264 bytes. **The header is 8 bytes**, not 2 as the
abandoned repo suggested:

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x07` — the **HID Report ID** (not a "magic") | [C] |
| 1 | 1 | command | [C] |
| 2 | 1 | index — profile? (written as `value + 1`, so 1-based) | [I] |
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

Portanto:

    263 data bytes + 1 Report ID byte = 264 = 0x108    -> matches wLength
    8 header bytes + 256 payload bytes = 264           -> matches the offsets

**The `0x07` byte at offset 0 is not a "vendor magic character"**, as
the abandoned repo claims (`KM_START_CHAR`). It is the **HID Report ID**, the same 7
that makes up `wValue = 0x0307` (`0x03` = report type FEATURE, `0x07` = report ID).

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
    r = HidDevice_GetFeature(h, buf, 0x108, 0);  // le    (ordinal 8)
    Sleep(10);

**The official software does NOT read the ACK on the interrupt endpoint
`0x83`.** It does a SetFeature, sleeps, and does a GetFeature into the **same
buffer**. The abandoned repo's approach (waiting for an 8-byte ACK on EP3)
works, but it is not what the manufacturer does — and its ACK wait has an

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
| `0x05` | `0x85` | — | `0x100d7d14` `0x100d500b` / `0x100d9b05` `0x100da909` |
| `0x09` | `0x89` | **LED Matrix Database** | **`0x100d6c12`** / `0x100db05d` |
| `0x0A` | `0x8A` | — | `0x100dbb94` `0x100dba93` / `0x100dbc5c` |
| `0x0D` | `0x8D`? | — | `0x100d1ecb` / not observed |
| `0x12` | `0x92` | — | `0x100d7642` / `0x100db4e6` |
| `0x16` | `0x96`? | — | `0x100d1f7e` / not observed |

**15 distinct commands across 19 sites.** The abandoned repo's list (4 commands)
covered a little over a quarter of that.

### 3.2 The bit-7 convention  [C]

**Confirmed empirically**, no longer inferred: six write/read pairs
observed (`0x01`/`0x81`, `0x02`/`0x82`, `0x05`/`0x85`, `0x09`/`0x89`,
`0x0A`/`0x8A`, `0x12`/`0x92`). Bit 7 set = read.

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

## 4. `0x01` / `0x81` — Current Profile

Selects or reads the active profile.

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 2 | ? | profile index | [?] |
| ... | | **to be determined** | [?] |

Number of profiles supported: **[?]** (the UI suggests several;
`LIGHTING_PROFILE_%d` indicates numbering)

---

## 5. `0x02` / `0x82` — Profile Information

| Offset | Size | Field | Confidence |
|---|---|---|---|
| | | **to be determined** | [?] |

---

## 6. `0x09` / `0x89` — LED Matrix Database  [C]

**Solved.** Decompiled from `FUN_100d6b80` (`Setting.dll` v2.04; the command's
site is at `0x100d6c12`).

### 6.1 Packet format

| Offset | Tam | Field | Value |
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

This was the most important open question, and the answer is not paging of
keys:

**A complete update is 3 packets**, one per channel. Each packet carries 144
    buf[3] == 2  ->  o 2o byte
    buf[3] == 3  ->  o 3o byte

**A complete update is 3 packets**, one per channel. Each packet carries
144 bytes = **one byte per LED slot**, which is that channel's intensity
on that LED. That explains why 144 fits comfortably: `8 + 144 = 152` of 264.

Which channel is R, G and B is still **not determined** — it requires a hardware test
or a USB capture.  [?]

### 6.3 There is no commit command  [C]

The third packet closes the update. The timing observed:

    for channel in (1, 2, 3):
        build packet
        SetFeature(buf, 0x108)
        Sleep(20 ms)
        if channel == 3:
            Sleep(100 ms)       # an extra wait after the last one

That is: 20 ms after each channel, plus 100 ms after the third. No separate
apply command.

### 6.4 Key → slot map  [C]

**Extracted.** Table at `0x1023b3b0` in `.data`, 132 slots, every key
identified. It has its own document: **`10-key-map.md`**.
Regenerar com `tools/dump_keymap.py`.

Formato: linhas de 8 bytes, `dword0` = virtual-key code do Windows (ou ID
≥ `0x800` for the G keys and zones), `dword1` = USB HID usage code.
**The row's position is the slot index.** The order is not the keyboard's
physical one — it is the hardware's LED scan order.

### 6.5 Open

- which channel (`buf[3]` = 1/2/3) corresponds to R, G and B  **[?]**
- what the remaining 12 slots are (144 − 132)  **[?]**
- does the `0x89` (read) return the matrix in the same format?  **[?]**
- intensity: is there a separate global brightness, or is it only the magnitude of the bytes?
  The keyboard has a physical brightness control, which suggests a register of its own —
  candidates among the new commands `0x05`, `0x0A`, `0x0D`, `0x12`, `0x16`
  **[?]**

## PHASE 2 — what is left

## 7. `0x08` / `0x88` — LED Effect Database

Animated effects run by the firmware.

| Offset | Size | Field | Confidence |
|---|---|---|---|
| | | **to be determined** | [?] |

Open questions: the effect catalog and their IDs; per-effect parameters
(speed, direction, color); whether an effect can coexist with static colors.

---

## 8. Initialization sequence  [?]

What the official software sends on startup, before any change.
**To be determined** — probably `0x81` (read profile) and version queries
via `HidDevice_GetDevVersion`.

---

## 9. Known gaps

### Phase 1 — nearly closed

| # | Item | Status |
|---|---|---|
| 1 | layout of the `0x09` payload | **solved** — section 6.1 |
| 2 | paging | **solved** — it is 1 packet per channel, section 6.2 |
| 3 | commit/apply command | **solved** — there is none, section 6.3 |
| 4 | key → slot map | **solved** — `10-key-map.md` |
| 5 | which channel is R, G, B | open — requires hardware or a capture |
| 6 | how intensity/brightness is controlled | open |

### Phase 2 — these complete the specification

| # | Item | Section |
|---|---|---|
| 5 | format of the `0x08` and the effect catalog | 7 |
| 6 | payloads de `0x01` e `0x02` | 4, 5 |
| 7 | bytes 3–7 of the ACK (and whether the ACK matters, since the official app ignores it) | 1.3 |
| 8 | the complete command set — 15 mapped so far, but it is a floor (see 3.3) | 3 |
| 9 | meaning of `buf[2]`, `buf[5]`, and of bytes 6–7 of the header | 2 |

For gap 8, two approaches: extract the command bytes from all 37
packet builders (that gives the set the software actually uses — that is how
`0x8B` and `0x92` showed up), and probe the keyboard with unlisted `0x8x`,
which are reads and therefore safe.

## Fontes

- Transport: `legonigel/KM780-RGB-Linux-Driver` (`src/keyboard.cpp`),
  confirmed against the hardware — see `01-transport.md`
- Interfaces and endpoints: `lsusb -v` on the user's machine
- Official binaries: `06-binaries.md`
- Addresses in `Setting.dll`: `08-setting-dll.md`
