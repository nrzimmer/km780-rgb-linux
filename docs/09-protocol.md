# KM780 RGB protocol — specification

> **Status:** incomplete. This is the canonical document; the others (`01`, `06`,
> `08`) are the trail of how we got here.
>
> Confidence convention for each field:
> **[C]** confirmed in the binary or on hardware · **[I]** inferred, not
> **[?]** unknown

---

## 1. Transport layer  [C]

USB device `28da:1101`, three HID interfaces:

| Interface | Protocol | Endpoint | Papel |
|---|---|---|---|
| 0 | 1 (Keyboard) | `0x81` IN | HID keyboard — do not touch |
| 1 | 2 (Mouse) | `0x82` IN | command output |
| 2 | 0 | `0x83` IN | reply input |

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

8 bytes on endpoint `0x83` IN (interrupt). Valid when:

    ack[0] == 0x00 && ack[1] == 0x00 && ack[2] == <command sent>

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x00` — status? always zero on success | [I] |
| 1 | 1 | `0x00` — status? always zero on success | [I] |
| 2 | 1 | echo of the command byte | [C] |
| 3–7 | 5 | **unknown** | [?] |

---

## 2. Packet envelope  [C]

Every packet is exactly 264 bytes:

    offset  tam   campo
    0       1     0x07   magic start                           [C]
    1       1     command                                      [C]
    2       262   payload, command-specific                    [C]

The `0x07` at offset 0 matches the report ID in `wValue=0x0307`.  [I]

---

## 3. Command table  [C]

| Command | Set | Get | Get reply |
|---|---|---|---|
| Current Profile | `0x01` | `0x81` | interrupt |
| Profile Information | `0x02` | `0x82` | control IN |
| LED Effect Database | `0x08` | `0x88` | control IN |
| LED Matrix Database | `0x09` | `0x89` | control IN |

Apparent convention: bit 7 set = read.  [I]
Whether there are commands outside this list is unknown.  [?]

---

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

## 6. `0x09` / `0x89` — LED Matrix Database

**The central command for static per-key color.**

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 2 | ? | page index / block offset | [I] |
| ? | ? | RGB data | [?] |

Open questions:

- Channel order: R/G/B or B/G/R?  **[?]**
- Bytes per key: 3 (RGB) or more (with brightness)?  **[?]**
- **Paging:** the KM780 has ~104 keys + side lighting. At 3 bytes/key
  that is ~312 bytes, which do not fit in the 262 of payload — so there is more than one packet
  per complete update, with some page field. How many packets, and what
  is the field?  **[?]**
- Key order: physical (matrix scan), by scancode, or by an index of
  its own?  **[?]**
- Is there a commit/apply command after sending all the pages?  **[?]**
- Do the side zones go into the same matrix or do they have a command of their own?  **[?]**

### 6.1 Key → offset map  [?]

    (to be determined — and the most laborious item of the specification)

---

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

| # | Item | Section |
|---|---|---|
| 1 | layout of the `0x09` payload, channel order | 6 |
| 2 | key → offset map and paging field | 6.1 |
| 3 | commit/apply command | 6 |
| 4 | format of the `0x08` and the effect catalog | 7 |
| 5 | payloads de `0x01` e `0x02` | 4, 5 |
| 6 | bytes 3–7 do ACK | 1.3 |
| 7 | existence of undocumented commands | 3 |

## Fontes

- Transport: `legonigel/KM780-RGB-Linux-Driver` (`src/keyboard.cpp`),
  confirmed against the hardware — see `01-transport.md`
- Interfaces and endpoints: `lsusb -v` on the user's machine
- Official binaries: `06-binaries.md`
- Addresses in `Setting.dll`: `08-setting-dll.md`
