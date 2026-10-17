# Capture `05-macros-and-toggles` — macros, key assignment, Checkpoint

Source: `captures/05-macros-and-toggles.pcapng` / `.txt` (1001 packets).
Action: turn effects and lighting on/off, select Checkpoint and change its
Transition Point, record a macro `A B C`, duplicate it, assign the duplicate to
G1 and assign Ctrl to G6.
Confidence: **[C]** except where marked.

## 0. A NAMING CORRECTION

What the earlier docs call a "profile" in `0x01`/`0x02` is really the
**Mode (M1/M2/M3)**. The app's SETTING screen shows the hierarchy:

    Profile1                       <- a PC-side concept (it has "Link Program")
      Mode 1 = "Mode1"             <- M1, on the keyboard
      Mode 2 = "Mode4"   (active)  <- M2
      Mode 3 = "Mode3"             <- M3

It matches the ASCII names found in `0x0A` page `0x00`: `Profile1`, `Mode1`,
`Mode4`, `Mode3`. So `07 01 02` = **select Mode 2**, and the three `0x02`
packets are the three modes. There are 3 modes, not 3 profiles.

## 1. Macro format — `0x05`  [C]

A new command. Sent in three banks (`buf[3]=0x00`, `buf[4]=1, 2, 3`); only
bank 1 carried data.

The macro recorded in the app was, literally from the screen:

    A | 0.07 s | A | 1.68 s | B | 0.07 s | B | 2.07 s | C | 0.09 s | C

The payload from byte 8 on:

    00 01 | 80 00 04 | 00 07 04 | 80 a8 05 | 00 07 05 | 80 cf 06 | 00 09 06

| Field | Size | Meaning |
|---|---|---|
| header | 2 | `00 01` — macro index/counter [I] |
| record | 3 | `[flag] [delay] [HID usage]` |

- **flag**: `0x80` = press, `0x00` = release
- **delay**: the time **before** the event, in units of **10 ms**
- **HID usage**: `0x04`=A, `0x05`=B, `0x06`=C

Checking, event by event:

| Record | Reading | Screen |
|---|---|---|
| `80 00 04` | press A, +0 ms | A |
| `00 07 04` | release A, +70 ms | 0.07 s |
| `80 a8 05` | press B, +1680 ms | 1.68 s |
| `00 07 05` | release B, +70 ms | 0.07 s |
| `80 cf 06` | press C, +2070 ms | 2.07 s |
| `00 09 06` | release C, +90 ms | 0.09 s |

Six events, five delays, all matching. **Format settled.**

## 2. Macro name — `0x0A` page `0x09`  [C]

    07 0a 09 00 ... 4d 61 63 72 6f 31 28 31 29   = "Macro1(1)"

That is the name of the macro's **duplicate**, exactly as the app named it. It
confirms that page `0x09` of `0x0A` is the macro name database, the same way
page `0x1c` is the effect database and `0x00` the profile/mode one.

## 3. The 144-byte table is a key ASSIGNMENT table  [C]

Assigning a function to a key rewrites the corresponding entry of the table
that starts at byte 120 of the mode payload (`0x02`):

| UI action | Byte changed | Slot | Value |
|---|---|---|---|
| macro on **G1** | 120 | 0 | `00` -> `b4` |
| **Ctrl** on **G6** | 125 | 5 | `00` -> `e0` |

`0xe0` is the HID usage of LeftCtrl — in other words, **the value stored is the
HID usage of the assigned function**. And slots 0 and 5 are exactly G1 and G6
in `10-key-map.md`.

This corrects the reading in `12-capture-01-open.md`: the table is not just
"the HID usage of each LED slot", it is the **table of what each key does**,
indexed by LED scan order. The factory content coincides with the key's own HID
usage because nothing had been remapped.

`0xb4` in G1's slot is the "run macro" code. Which macro probably comes from
the index in the `0x05` header.  [I]

## 4. Checkpoint = effect `0x08`  [C]

It was the missing ID. With it the catalog is complete:

| byte 10 | Effect |
|---|---|
| `0x00` | none (effects off) — **confirmed in this capture** |
| `0x01` | Wave |
| `0x02` | Ripple |
| `0x03` | Sonic |
| `0x04` | Breathing |
| `0x07` | Changeling |
| `0x08` | **Checkpoint** |
| `0x09` | Cycle |
| `0x0A` | Rain |

`0x05` and `0x06` are missing — the UI only offers 8 effects.

> **Solved in `docs/27` §10, on hardware.** `0x06` **exists**: the matrix is
> dark at rest, and a pressed key lights up **and stays lit**. And the DLL's
> builder has no branch for it, so it is a firmware effect the official software
> does not know how to configure. `0x05` reacts to nothing (static red) and
> still has no known function.  [?]

Checkpoint has neither Duration nor Direction: it has a **"Transition Point",
in number of keys pressed**. Changing it from 3 to 6 wrote `0c` into bytes
15/19/23 and `06` into bytes 27..71 — **the same field as duration**, with the
same 2x rule in the first three records.

## 5. Lighting toggles  [I]

The toggle pair shows up in **byte 108** of the mode payload, going on and off.
The exact order of the clicks was not recorded, so which of the two ("Disable
All Lighting" or "Disable Volume Display") byte 108 is remains pending an
isolated capture.

Two `0x0E` with zeroed payloads and two `0x08` also appeared, one of them with
`buf[4]=0x02`.

## 6. Known commands after this capture

    write: 0x01 0x02 0x05 0x07 0x08 0x09 0x0A 0x0B 0x0D 0x0E 0x0F 0x12 0x15 0x16
    read:  0x81 0x82 0x89 0x8A
