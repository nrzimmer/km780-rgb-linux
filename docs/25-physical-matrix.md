# The physical matrix, key by key

> Mapped on 2026-10-16 on the hardware, with `tools/km780matrix.py`.
> 122 keys, zero conflicts. **[C]**

The keyboard emits `f6 01 <column> <row>` on EP `0x83` for every key, once
`0x16` has opened the session (`24-session-0x16.md`). This is the complete map.

## Validation

The map was built on Linux. Applied to the `f6` events of the 11 captures made
on Windows, it **recognises 1535 out of 1535 — 100%** —, and decodes the real
typing into readable text:

    t=510 of 08-closing:  F I Z Space M A I S Space D Backspace C O I S A S
    t=0   of 01-open:     R I R Space O Space A P P . E X E Space ' E Space V C

No coordinate captured on Windows fell outside the map built on Linux.

## The matrix is 15 × 9  [C]

135 positions, **122 occupied**, 13 empty. `22-gaps-solved.md` §2.3 said
13 × 9, deduced from the range of values that showed up in the captures — the
range was short because **column 0 was never pressed at all on Windows**: those
are the G keys, MR and the modes, which type nothing.

## Grid

```
                 c0          c1          c2          c3          c4          c5          c6          c7          c8          c9         c10         c11         c12         c13         c14
  l0            G1         Esc          F1          F2          F4          F5          F6          F8           [         F11         F12       PrtSc       Pause        Num*    NumEnter
  l1            G2           `           2           3           4           6           8           9           -           =           ·      Insert        PgUp        Num/        Num-
  l2            G3         Tab           Q           E           R           T           U           O           P         F10           \      Delete        PgDn        Num8        Num+
  l3            G4    CapsLock           A           S           F           G           J           L           ;           '       Enter       ScrLk        Num4        Num5           ·
  l4            G5      LShift           Z           C           V           B           N           ,           .           ·      RShift        Home        Num1        Num2        Num3
  l5            G6       LCtrl        LAlt           ·           5       Space          F7           ·        RAlt        Menu        Left         End     NumLock        Num0        Num.
  l6            MR           1          M3          F3 the bag one           7           I          F9           /           ]   Backspace     media 1        Num7        Num9           ·
  l7            M1           ·           W           D      brightness           Y           K           0        RWin           ·          Up     media 2     media 5        Num6           ·
  l8            M2        LWin           X           ·       timer           H           M           ·           ·       RCtrl        Down     media 3     media 4       Right           ·
```

## Column 0: the entire left-hand side  [C]

    (0,0) G1   (0,1) G2   (0,2) G3   (0,3) G4   (0,4) G5   (0,5) G6
    (0,6) MR   (0,7) M1   (0,8) M2

Nine positions, nine keys, in order. **M3 does not fit there** — the column is
full — and came out at `(2,6)`. The matrix wiring does not follow physical
adjacency.

## The three nameless keys  [C]

They form a vertical trio in column 4:

    (4,6)  the bag-symbol one     (4,7)  brightness     (4,8)  timer

### Correction: `(4,7)` is brightness, not Timer

`19-capture-08-closing.md` §5 attributed `f6 01 04 07` to the **Timer** key.
That is wrong, and the capture itself proves it: each `f6 (4,7)` is followed
**40 ms later** by a brightness event, five times in a row, walking the levels
3, 2, 1, 0, 4.

    t=493.71  f6 (4,7)        t=493.75  brightness level 3
    t=496.04  f6 (4,7)        t=496.08  brightness level 2
    t=499.46  f6 (4,7)        t=499.50  brightness level 1
    t=503.50  f6 (4,7)        t=503.54  brightness level 0
    t=506.57  f6 (4,7)        t=506.61  brightness level 4

The "isolated pair at `04 08`" that document mentions in passing is the one
that is Timer, and it matches today's collection.

## MR, M1, M2 and M3 are ordinary matrix keys  [C]

They were expected to emit `02 04 <mode> 08 01`, the mode event. **They do
not**: of the 122 keys collected, **every one** produced an `f6` and none
produced an event of another kind. The mode switch happens inside the keyboard;
the host finds out from the key's `f6`, or by re-reading with `0x81`.

## The volume wheel emits nothing  [C]

Rolled both ways during the collection, it produced no event at all on
EP `0x83`.

## Complete table: key, LED slot, coordinate

| Key | LED slot | Matrix (col, row) |
|---|---|---|
| `Esc` | 9 | (1, 0) |
| ``` | 10 | (1, 1) |
| `Tab` | 11 | (1, 2) |
| `CapsLock` | 12 | (1, 3) |
| `LShift` | 13 | (1, 4) |
| `LCtrl` | 14 | (1, 5) |
| `1` | 15 | (1, 6) |
| `LWin` | 17 | (1, 8) |
| `F1` | 18 | (2, 0) |
| `2` | 19 | (2, 1) |
| `Q` | 20 | (2, 2) |
| `A` | 21 | (2, 3) |
| `Z` | 22 | (2, 4) |
| `LAlt` | 23 | (2, 5) |
| `W` | 25 | (2, 7) |
| `X` | 26 | (2, 8) |
| `F2` | 27 | (3, 0) |
| `3` | 28 | (3, 1) |
| `E` | 29 | (3, 2) |
| `S` | 30 | (3, 3) |
| `C` | 31 | (3, 4) |
| `F3` | 33 | (3, 6) |
| `D` | 34 | (3, 7) |
| `F4` | 36 | (4, 0) |
| `4` | 37 | (4, 1) |
| `R` | 38 | (4, 2) |
| `F` | 39 | (4, 3) |
| `V` | 40 | (4, 4) |
| `5` | 41 | (4, 5) |
| `F5` | 45 | (5, 0) |
| `6` | 46 | (5, 1) |
| `T` | 47 | (5, 2) |
| `G` | 48 | (5, 3) |
| `B` | 49 | (5, 4) |
| `Space` | 50 | (5, 5) |
| `7` | 51 | (5, 6) |
| `Y` | 52 | (5, 7) |
| `H` | 53 | (5, 8) |
| `F6` | 54 | (6, 0) |
| `8` | 55 | (6, 1) |
| `U` | 56 | (6, 2) |
| `J` | 57 | (6, 3) |
| `N` | 58 | (6, 4) |
| `F7` | 59 | (6, 5) |
| `I` | 60 | (6, 6) |
| `K` | 61 | (6, 7) |
| `M` | 62 | (6, 8) |
| `F8` | 63 | (7, 0) |
| `9` | 64 | (7, 1) |
| `O` | 65 | (7, 2) |
| `L` | 66 | (7, 3) |
| `,` | 67 | (7, 4) |
| `F9` | 69 | (7, 6) |
| `0` | 70 | (7, 7) |
| `[` | 72 | (8, 0) |
| `-` | 73 | (8, 1) |
| `P` | 74 | (8, 2) |
| `;` | 75 | (8, 3) |
| `.` | 76 | (8, 4) |
| `RAlt` | 77 | (8, 5) |
| `/` | 78 | (8, 6) |
| `RWin` | 79 | (8, 7) |
| `F11` | 81 | (9, 0) |
| `=` | 82 | (9, 1) |
| `F10` | 83 | (9, 2) |
| `'` | 84 | (9, 3) |
| `Menu` | 86 | (9, 5) |
| `]` | 87 | (9, 6) |
| `RCtrl` | 89 | (9, 8) |
| `F12` | 90 | (10, 0) |
| `\` | 92 | (10, 2) |
| `Enter` | 93 | (10, 3) |
| `RShift` | 94 | (10, 4) |
| `Left` | 95 | (10, 5) |
| `Backspace` | 96 | (10, 6) |
| `Up` | 97 | (10, 7) |
| `Down` | 98 | (10, 8) |
| `PrtSc` | 99 | (11, 0) |
| `Insert` | 100 | (11, 1) |
| `Delete` | 101 | (11, 2) |
| `ScrLk` | 102 | (11, 3) |
| `Home` | 103 | (11, 4) |
| `End` | 104 | (11, 5) |
| `Pause` | 108 | (12, 0) |
| `PgUp` | 109 | (12, 1) |
| `PgDn` | 110 | (12, 2) |
| `Num4` | 111 | (12, 3) |
| `Num1` | 112 | (12, 4) |
| `NumLock` | 113 | (12, 5) |
| `Num7` | 114 | (12, 6) |
| `Num*` | 117 | (13, 0) |
| `Num/` | 118 | (13, 1) |
| `Num8` | 119 | (13, 2) |
| `Num5` | 120 | (13, 3) |
| `Num2` | 121 | (13, 4) |
| `Num0` | 122 | (13, 5) |
| `Num9` | 123 | (13, 6) |
| `Num6` | 124 | (13, 7) |
| `Right` | 125 | (13, 8) |
| `NumSep` | 126 | (14, 0) |
| `Num-` | 127 | (14, 1) |
| `Num+` | 128 | (14, 2) |
| `Num3` | 130 | (14, 4) |
| `Num.` | 131 | (14, 5) |

### Special keys: group known, individual slot not  [I]

| Key | Matrix | LED slot |
|---|---|---|
| `G1` | (0, 0) | one of {0-5} |
| `G2` | (0, 1) | one of {0-5} |
| `G3` | (0, 2) | one of {0-5} |
| `G4` | (0, 3) | one of {0-5} |
| `G5` | (0, 4) | one of {0-5} |
| `G6` | (0, 5) | one of {0-5} |
| `MR` | (0, 6) | one of {6,7,8,24} |
| `M1` | (0, 7) | one of {6,7,8,24} |
| `M2` | (0, 8) | one of {6,7,8,24} |
| `M3` | (2, 6) | one of {6,7,8,24} |
| `brightness` | (4, 7) | one of {42,43,44} |
| `timer` | (4, 8) | one of {42,43,44} |
| `the bag one` | (4, 6) | one of {42,43,44} |
| `media 1` | (11, 6) | one of {105,106,107,115,116} |
| `media 2` | (11, 7) | one of {105,106,107,115,116} |
| `media 3` | (11, 8) | one of {105,106,107,115,116} |
| `media 4` | (12, 8) | one of {105,106,107,115,116} |
| `media 5` | (12, 7) | one of {105,106,107,115,116} |

The grouping comes from the HID usage in the factory table (`10-key-map.md`).
Which slot is which key within the group **only comes out by writing a color
into a slot and looking at which key lights up** — a write experiment, not done
yet.

## What was left out

- **13 empty positions** in the grid: (10,1), (14,3), (9,4), (3,5), (7,5), (14,6), (1,7), (9,7), (14,7), (3,8), (7,8), (8,8), (14,8)
- the extra ISO key next to LShift **emitted nothing**. The system is set to
  `pc105 intl`, but that is software configuration; the keyboard is physically
  US/ANSI and that key does not exist on it. Consistent with slot 16 still
  being `[?]` in `23-hardware-validation.md` §6.
- LED slots with no corresponding key in the collection:
  `Z801` (slot 0);  `Z802` (slot 1);  `Z803` (slot 2);  `Z804` (slot 3);  `Z805` (slot 4);  `Z806` (slot 5);  `Z807` (slot 6);  `Z808` (slot 7);  `Z809` (slot 8);  `\|` (slot 16);  `Z80a` (slot 24);  `VK25` (slot 32);  `INTL8b` (slot 35);  `Z80b` (slot 42);  `Z80c` (slot 43);  `Z80d` (slot 44);  `VK61` (slot 68);  `VK28` (slot 71);  `INTL88` (slot 80);  `Z813` (slot 85);  `VK22` (slot 88);  `INTL89` (slot 91);  `Z80e` (slot 105);  `Z80f` (slot 106);  `Z810` (slot 107);  `Z811` (slot 115);  `Z812` (slot 116);  `INTL85` (slot 129);
