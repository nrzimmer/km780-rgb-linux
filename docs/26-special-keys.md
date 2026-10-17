# The 19 special keys: LED slot identified

> 2026-10-17, on hardware, with `tools/km780slots.py`. **[C]**
>
> Method: light one group at a time, each slot in a different color, and read
> the legend off the keyboard. Four rounds for 18 keys.

The factory table (`10-key-map.md`) gives 19 slots with a **synthetic ID**
(`0x801`–`0x813`) instead of a virtual-key, which are the keys with no
equivalent on an ordinary keyboard. Their grouping came from the HID usage,
but **which slot was which key, within the group, had never been determined**.
Now it is.

## The table

| Slot | HID | Key | Matrix (col, row) |
|---|---|---|---|
| 0 | `0x00` | **G1** | (0, 0) |
| 1 | `0x00` | **G2** | (0, 1) |
| 2 | `0x00` | **G3** | (0, 2) |
| 3 | `0x00` | **G4** | (0, 3) |
| 4 | `0x00` | **G5** | (0, 4) |
| 5 | `0x00` | **G6** | (0, 5) |
| 6 | `0xb3` | **MR** | (0, 6) |
| 7 | `0xad` | **M1** | (0, 7) |
| 8 | `0xae` | **M2** | (0, 8) |
| 24 | `0xaf` | **M3** | (2, 6) |
| 42 | `0xb9` | **WinLock** (the one with the bag symbol) | (4, 6) |
| 43 | `0xbb` | **Brightness** | (4, 7) |
| 44 | `0xba` | **Timer** | (4, 8) |
| 85 | `0x87` | international, no physical key | — |
| 105 | `0xa5` | **Stop** | (11, 6) |
| 106 | `0xa6` | **Prev** | (11, 7) |
| 107 | `0xa7` | **Play/Pause** | (11, 8) |
| 115 | `0xa9` | **Mute** | (12, 7) |
| 116 | `0xa8` | **Next** | (12, 8) |

## What corroborates the reading

**The G keys' slots come out in order**: 0 to 5 = G1 to G6, and so do the
matrix coordinates (column 0, rows 0 to 5). Two independent numberings in
agreement.

**`M1`, `M2` and `M3` have consecutive HID codes**: `0xad`, `0xae`, `0xaf`.
`MR`, which is not a mode but the macro recorder, sits outside the sequence, at
`0xb3`.

**The column-4 trio does not come out in slot order**: `42` = WinLock,
`43` = Brightness, `44` = Timer, but the HID codes are `b9`, `bb`, `ba`.
Brightness has the highest code of the three despite sitting in the middle
slot. A reminder that slot, HID and physical position are three different
orderings.

## The multimedia codes, now with names  [C]

`09-protocol.md` §5.2 lists the range `0xA5`–`0xAB` as "Multimedia", seven
codes. Five of them are these keys:

    a5 = Stop        a6 = Prev Track   a7 = Play/Pause
    a8 = Next Track  a9 = Mute Volume

**`aa` and `ab` are left over.** The app's KEY ASSIGNMENT dialog offers seven
Multimedia options — the five above plus **Volume Up** and **Volume Down**. And
the volume wheel, the only control still unaccounted for, **emits no `f6` event
and has no LED slot** (`25-physical-matrix.md`). Both ends meet: `aa` and `ab`
are the two directions of the wheel, which is a control with no key and no LED.

**Confirmed for `ab` on 2026-10-17.** Assigned to byte 164 (the Timer key),
`0xAB` **turns the volume up** — the deduction was right. `aa` has no direct
test yet, but getting one end of the pair right reinforces the other. See
`docs/09` §5.2. **[C]** for `ab`, **[I]** for `aa`.

## Consequence for the tooling

`km780lib.py` now uses these names in place of `Z801`…`Z813`. So this works
straight away:

    tools/km780ctl.py set G1=ff0000 M2=blue Play=green

## A note on method

This cost four rounds because each group was lit whole, with one color per
slot, instead of one key at a time. Eighteen questions became four. Restoring
the colors sits in a `finally`, and the script refuses to run if the mode has
an active effect — the animation would overwrite the matrix and the test would
be worth nothing.
