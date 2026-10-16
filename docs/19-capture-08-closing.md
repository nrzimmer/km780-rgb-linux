# Capture `08-closing` — gradient, shortcuts, `0x08`, the Timer key

Source: `captures/08-closing.pcapng` / `.txt` (3259 packets).
Confidence: **[C]** except where marked.

## 1. CORRECTION: the effect parameter has a stop count  [C]

Byte 3 of the parameter block (`0x0A` sub `02`/`05`/`08`) is **not a constant
`03`**: it is the **number of color stops**. It became `04` when a fourth stop
was added in the UI.

    00 <duration> <direction> <N>  followed by N stops of 5 bytes each

Each stop: **`R G B <intensity> <position>`**, intensity and position in
0-100 (`0x64` = 100).

A real example with N=4:

    00 03 03 04 | ff 00 00 64 00 | ff ff ff 64 17 | 00 ff 00 16 2f | 00 00 ff 64 64
                   red              white            green            blue
                   int100 pos0      int100 pos23     int 22 pos47     int100 pos100

The green stop with intensity 22 is exactly the one that was dragged down in
the UI, and the positions come out in increasing order. That promotes the
reading in `15-capture-04-effects.md` §5 from **[I] to [C]**, and corrects the
"always three colors".

The stop count also appears in the mode payload, in **bytes 75, 76, 77** (they
changed `03` -> `04` along with it).

## 2. CORRECTION: Windows Shortcuts is not a contiguous range  [C]

Walking the 7 items in the list in UI order, on a G key:

| Code | Function |
|---|---|
| `0xC0` | Launch Task Manager |
| `0xC1` | Open Start Menu |
| `0xC2` | Show Desktop |
| `0xC5` | Close Active Window |
| `0xC6` | Cut |
| `0xC7` | **Copy** |
| `0xC8` | Paste |

**There is a hole at `0xC3` and `0xC4`.** The prediction in
`17-capture-06-key-assignment.md` §4 (a sequential `C2`-`C8` range) was
**wrong** and is corrected here.

The alignment validates itself: Copy landed on `0xC7`, the same value capture
`06` had measured independently.

## 3. `0x08` finally varied  [C] on the format, [?] on the semantics

Changing the gradient changes the `0x08` payload. It has a **fixed size of 69
data bytes** (bytes 8 to 76), that is **3 x 23**, and uses an alphabet of only
six values:

    00  38  4c  7c  98  fc

All multiples of 4 — they are **6-bit levels shifted left by 2** (0, 14, 19,
31, 38, 63). It looks like an **already-rasterized gradient animation table**,
in 23 steps, probably three channels. The exact correspondence with the color
stops was not resolved.  [?]

It is the first real progress on this command: before, only the frozen value
`fc fc fc 7c 7c 7c 7c` was known.

## 4. The two toggles are different bytes  [I]

In this capture both appear, separately:

| Byte | Observed |
|---|---|
| 119 | on/off (also what moved in `03-config`) |
| 108 | on/off (also what moved in `05-macros`) |

By correspondence with what was asked for in each capture, the most likely
attribution is **119 = Disable Volume Display** and **108 = Disable All
Lighting**, but neither was isolated enough to become [C].

**A UI bug, reported by the user:** changes to those toggles have no effect on
the keyboard until the mode is switched. It had been seen before. That explains
why the attribution is hard to isolate.

## 5. Timer key = column 4, row 7  [C]

Nine presses in three groups of three, isolated in time:

    t=493.71   04 07  x3
    t=496.04   04 07  x3
    t=499.46   04 07  x3

The Timer key **has no HID usage** — it is a proprietary G.SKILL function.
Even so it emits `f6 01 04 07`. That is, the `f6 01 XX YY` event covers **every
physical key**, including the ones that do not exist in HID, which makes it the
complete path for mapping the physical matrix. Reinforces
`18-capture-07-final.md` §3.

Another isolated pair appeared at `04 08` (row 8).

## 6. Smaller observations

- The `02 04 XX 08` event has its third byte equal to the **mode number**
  (`01` in the earlier captures, `02` here).  [I]
- **The OSD (On Screen Display) belongs to the app**, drawn on the PC's screen;
  it generates no USB traffic and is of no interest to the protocol. The user's
  report.
- Mode switching confirmed in use: eight `0x01` with `buf[2]` = 1, 2, 3.

## 7. The keyboard switches mode and records macros on its own

The KM780 has the physical keys **MR, M1, M2 and M3**: they switch the active
mode and record macros straight into the keyboard, with no software at all.
That was not captured here because it does not need Windows — it can be
investigated on Linux, just by listening to EP `0x83`.

Two practical consequences:

1. **The state changes underneath the host.** Tooling of ours cannot assume the
   active mode is the last one it wrote: it has to re-read with `0x81`, or
   listen to the EP `0x83` events.
2. Recording a macro from the keyboard should produce its own device->host
   events, and may reveal how the firmware indexes the macro database —
   something the app capture only showed from the writing side.
