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

## 2. Packet envelope

Every packet is exactly 264 bytes. **The header is 8 bytes**, not 2 as the
abandoned repo suggested:

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 0 | 1 | `0x07` — magic / report ID | [C] |
| 1 | 1 | command | [C] |
| 2 | 1 | index — profile? (written as `value + 1`, so 1-based) | [I] |
| 3 | 1 | `0x00` — always zeroed in the paths seen | [C] |
| 4 | 1 | **page number** (written as `counter + 1`, 1-based) | [C] |
| 5 | 1 | a boolean flag, meaning depends on the command | [I] |
| 6–7 | 2 | they exist but are not written on the paths seen | [?] |
| 8 | 256 | payload | [I] |

The 8-byte header is inferred from the offsets of the local variables in
`FUN_100db460`'s decompilation: the buffer starts at `local_20c` (offset 0) and
the payload array is `local_204` (offset 8), with bytes 6 and 7 declared but not
assigned on that path.

The `0x07` at offset 0 matches the report ID in `wValue=0x0307`.  [I]

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

### 3.1 Named (from the abandoned repo, the app's symbolic names)  [C]

| Command | Set | Get |
|---|---|---|
| Current Profile | `0x01` | `0x81` |
| Profile Information | `0x02` | `0x82` |
| LED Effect Database | `0x08` | `0x88` |
| LED Matrix Database | `0x09` | `0x89` |

### 3.2 Discovered in `Setting.dll`, unnamed  [C]

| Command | Where it showed up | Notes |
|---|---|---|
| `0x8B` | `FUN_100db460`, `param_2` ∈ {2, 3} | paged; `buf[5]` = `param_2 != 2` |
| `0x92` | `FUN_100db460`, `param_2 == 10` | paged; without the `buf[5]` flag |

**The abandoned repo's list is incomplete.** If the bit 7 convention holds
(bit 7 = read), the write counterparts would be `0x0B` and `0x12`.  [I]

Apparent convention: bit 7 set = read.  [I]
The complete command set is still unknown.  [?]

---

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

## 6. `0x09` / `0x89` — LED Matrix Database

**The central command of Phase 1: static per-key color.**

| Offset | Size | Field | Confidence |
|---|---|---|---|
| 2 | ? | page index / block offset | [I] |
| ? | ? | RGB data | [?] |

Open questions:

- Channel order: R/G/B or B/G/R?  **[?]**
- Bytes per key: 3 (RGB) or more (with brightness)?  **[?]**
- **Paging: partly settled.** The page field is `buf[4]`, 1-based
  (see section 2). In `FUN_100db460` the loop accumulates into a **768-byte** buffer
  (`memset(.., 0x300)`, three areas of 256), which suggests **3 pages of 256
  bytes**. Whether the `0x09` uses the same geometry still has to be confirmed.  [I]
- Key order: physical (matrix scan), by scancode, or by an index of
  its own?  **[?]**
- Is there a commit/apply command after sending all the pages?  **[?]**
- Do the side zones go into the same matrix or do they have a command of their own?  **[?]**
- **Intensity:** is there a separate brightness (global or per key), or is brightness
  only the magnitude of the RGB values?  **[?]**  The keyboard has a physical
  brightness control, which suggests a register of its own — possibly outside the `0x09`.

### 6.1 Key → offset map  [?]

    (to be determined — and the most laborious item of the specification)

---

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

### Phase 1 — these block the color change

| # | Item | Section |
|---|---|---|
| 1 | layout of the `0x09` payload, channel order | 6 |
| 2 | key → offset map (the page field is already known: `buf[4]`) | 6.1 |
| 3 | commit/apply command | 6 |
| 4 | how intensity/brightness is controlled | 6 |

### Phase 2 — these complete the specification

| # | Item | Section |
|---|---|---|
| 5 | format of the `0x08` and the effect catalog | 7 |
| 6 | payloads de `0x01` e `0x02` | 4, 5 |
| 7 | bytes 3–7 of the ACK (and whether the ACK matters, since the official app ignores it) | 1.3 |
| 8 | the complete command set — `0x8B` and `0x92` have already poked a hole in the list | 3 |
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
