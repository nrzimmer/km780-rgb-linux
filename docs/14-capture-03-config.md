# Capture `03-config` — polling rate, repeat delay, volume display

Source: `captures/03-config.pcapng` / `.txt`.
Action: walk through the settings the official UI lets you change.
Confidence: **[C]** where indicated; the action->byte attribution of two items
is **[I]** until the user confirms the exact order of the clicks.

## 1. What showed up on the wire

11 actions, each with the same sequence of 2 profile writes. No new command:
only `0x0D`, `0x09`, `0x02` and `0x0A`.

Of the profile's 264 bytes, **only three varied** across the whole capture:

| Byte | Variation | Reading |
|---|---|---|
| 8 | `08 → 04 → 02 → 01 → 08` | **polling rate** |
| 106 | alternates `01`/`00` on every write | first/second write flag |
| 119 | `00 … 01 01 00 00` | **volume display** (toggle) [I] |

## 2. Polling rate = byte 8 of the profile, in milliseconds  [C]

    byte 8 = 1  ->  1000 Hz
    byte 8 = 2  ->   500 Hz
    byte 8 = 4  ->   250 Hz
    byte 8 = 8  ->   125 Hz

It is literally the **endpoint's `bInterval`**. That closes the circle with
`09-protocol.md` §1.0.1: the firmware carries three copies of the configuration
descriptor that differ only in `bInterval` (1, 4, 8). The capture shows that
value 2 also exists, so either there is a fourth copy, or the firmware patches
the descriptor at run time.  [I]

## 3. Repeat delay goes in `0x0A`, not in the profile  [C for the format, I for the name]

Four consecutive actions touched no profile byte at all. What changed was the
second `0x0A` packet:

    07 0a 1b 0a 00 00 00 00  00 00 01 1f 00 ...   <- "normal" value
    07 0a 1b 0a 00 00 00 00  00 00 01 05 03 ...   <- action 1
    07 0a 1b 0a 00 00 00 00  00 00 01 05 02 ...   <- action 2
    07 0a 1b 0a 00 00 00 00  00 00 01 05 01 ...   <- action 3
    07 0a 1b 0a 00 00 00 00  00 00 01 05 00 ...   <- action 4

Structure of `0x0A` page `1b`/`0a`:

| Offset | Field |
|---|---|
| 2 | `0x1b` — page |
| 3 | `0x0a` — subpage |
| 10 | `0x01` — counter? always 1 |
| 11 | **parameter ID** (`0x1f` at rest, `0x05` here) |
| 12 | **value** (0–3) |

In other words, `0x0A` carries **(id, value)** pairs of settings that do not
live in the profile. This is the first concrete semantics for `0x0A`.

## 4. Controls that emit nothing

**N-key rollover and repeat rate do not work** in this version's UI — the
user's report, and consistent with the capture: there is no leftover packet
that could correspond to them. There is no byte to look for.

## 5. Pending confirmation

The exact order of the clicks was not recorded, so the attribution
"4 `0x0A` actions = repeat delay" and "2 byte-119 actions = volume display" is
inferred from the shape (4 values vs. a 2-state toggle). A short capture
touching only the volume display settles it.
