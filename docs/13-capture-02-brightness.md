# Capture `02-brightness` — intensity, solved

Source: `captures/02-brightness.pcapng` (extracted into
`captures/02-brightness.txt`).
Action: walk through every brightness level the official UI allows.
Confidence: **[C]**.

## 1. Result

**Intensity = byte 11 of the profile payload (`0x02`), values 0 to 4.**

Walking through the levels, the 12 `0x02` packets of the active profile differ
from one another in a single data byte:

    byte 11:  00  00  01  02  03  04

(byte 106 also alternates 01/00, but it alternates between the two writes of
each action, not with the level: it is a "first/second write" flag, not data.)

The `0x09` packets sent in the same action keep the color intact (`0xFF` on
channel 1 / R). In other words: **brightness is applied by the firmware. The
host does not pre-multiply the color.** For the Linux utility that means color
and brightness are two independent paths.

The hypothesis in `11-firmware.md` (command `0x0B` as the write counterpart of
the brightness event) **was not confirmed** — the official app never sends
`0x0B`.

## 2. What the app sends on every brightness change

Full sequence, repeated identically at each level (11 packets, ~1.2 s):

    07 0d 00                    enter configuration mode
    07 09 01 01 <144 bytes>     matrix, channel 1 (R)
    07 09 01 02 <144 bytes>     channel 2 (G), all zero
    07 09 01 03 <144 bytes>     channel 3 (B), all zero
    07 02 01 <256 bytes>        profile 1, with the new byte 11
    07 0a 1b 09                 0x0A, payload all zero
    07 0a 1b 0a  ... 01 1f      0x0A, with 01 1f in bytes 10-11
    07 02 01 <256 bytes>        profile 1 again
    07 02 02 <256 bytes>        profile 2 (unchanged, factory default)
    07 02 03 <256 bytes>        profile 3 (unchanged)
    07 0d 01                    leave configuration mode

The app is wasteful: it rewrites the whole matrix and all three profiles to
change one byte. **None of that is necessary** — the `0x02` of the active
profile is enough.

## 3. Extra confirmation of the slot map  [C]

Channel 1 (R) came back `0xFF` in every slot **except six**, which came back
`00`:

| Slot | HID | What it is |
|---|---|---|
| 35 | `0x8b` | international key (JIS/ABNT) |
| 80 | `0x88` | international key |
| 85 | `0x87` | international key |
| 91 | `0x89` | international key |
| 129 | `0x85` | international key |
| 132+ | — | the 12 unused slots |

They are exactly the keys that **do not exist on the US physical layout**. The
official software itself leaves them dark. A third independent confirmation of
the slot order in `10-key-map.md`.

## 4. Command `0x0A`  [I]

It always appears as a pair, in the middle of the profile write:

    07 0a 1b 09 00 00 00 00  00 00 00 00 ...
    07 0a 1b 0a 00 00 00 00  00 00 01 1f ...

`buf[2]=0x1b` in both, `buf[3]` = `0x09` and `0x0a`. The same `1b 09` /
`1b 0a` pair appears in the init as a **read** (`0x8A`), so `buf[2]`/`buf[3]`
are a page address in an indexed database, and `0x0A` is the write to that
database. The only useful content is `01 1f` in bytes 10-11 of the second
packet.  **[I]**

## 5. Device->host events on EP `0x83`

Besides the ACKs (`00 00 <cmd> 00 ...`), these showed up:

- `02 04 01 08 01 00 00 00` — 30 times, ~5 per brightness change. Looks like a
  "state changed" notification.  **[I]**
- `f6 01 XX YY 00 00 00 00` — 44 times, `XX` in `02..0a`, `YY` in `02..08`.
  The same format seen in `01-open` with no software running at all; the rhythm
  matches typing. A candidate for the **matrix (row, column) coordinate**.  **[I]**
