# Capture `10-limits` — extended delay, `0x08` solved, duration per effect

Source: `captures/10-limits.pcapng` / `.txt`.
Actions: a macro with a long pause; a macro with Ctrl+C; duration on two
different effects; a series of controlled gradients.
Confidence: **[C]**.

## 1. The macro delay has an extended field  [C]

The delay field is 1 byte, which saturates at 2.55 s. A longer pause uses the
**low nibble of the flag byte** as the high bits:

    delay = ((flag & 0x0F) << 8) | delay_byte          in units of 10 ms
    flag:  bit 7 = press (0x80) / release (0x00)

Macro `A ... B` with a long pause:

    00 01 | 80 00 04 | 00 06 04 | 83 61 05 | 00 07 05
            press A    release A  press B    release B

`83 61` -> `(3 << 8) | 0x61` = 865 -> **8.65 s**. The app confirmed it on
screen: "8.65 seconds delay" between releasing A and pressing B.

Practical ceiling: `0x0FFF` = 4095 -> ~41 s.

## 2. A modifier is an ordinary key  [C]

Ctrl+C macro:

    80 00 e0 | 80 cf 06 | 00 06 06 | 00 3b e0
    press Ctrl press C    release C  release Ctrl

`0xE0` is just the HID usage of LeftCtrl. **There is no modifier field** —
Ctrl goes down and up like any other key. The app's screen lists exactly
`Ctrl / 2.07 s / C / 0.06 s / C / 0.59 s / Ctrl`, and the bytes `cf`=207,
`06`=6, `3b`=59 match one for one.

## 3. `0x08` SOLVED: the rasterized gradient  [C]

The 69 bytes are **3 channels x 23 steps**. With a two-stop gradient, red at 0
and blue at 100:

    channel 1 (R):  252 252 252 252 252 124 124 124 124 124 124   0   0   0   0   0   0 124 124 124 124 124 124
    channel 2 (G):    0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0   0
    channel 3 (B):    0   0   0   0   0 124 124 124 124 124 124 252 252 252 252 252 252 124 124 124 124 124 124

- **G is zero across all 23 steps** because there was no green in the gradient
- R falls from red and **rises again at the end**; B mirrors it

In other words: **the gradient interpolated over 23 steps, per channel,
cyclically** (the last step wraps to the first, because the effect loops).

The values are quantized to **6 bits**, stored shifted left by 2:
`00 38 4c 60 7c 98 c4 e0 fc` = 0, 14, 19, 24, 31, 38, 49, 56, 63.

Confirmed as well by the three-stop case with the middle one at `c6 01 0b`
(R=198): channel R started showing intermediate values (`e0`=224, `c4`=196,
`60`=96) between 252 and 0, exactly where the interpolation requires them.

And the color does **not** travel alone: changing the stop from red to white
without touching any position left the table identical in one of the two-stop
cases, because the affected channel was already saturated.

## 4. Each effect keeps its own duration  [C]

Sequence Wave (9) -> Breathing (2) -> Wave:

| Moment | byte 10 | byte 15 | bytes 27..71 |
|---|---|---|---|
| Wave with duration 9 | `01` | `12` (=18, 2x) | `09` |
| switched to Breathing | `04` | `06` | `03` |
| Breathing with duration 2 | `04` | `04` | `02` |
| back to Wave | `01` | `12` | `09` |

Going back to Wave **restored 9 on its own**. So duration is per effect, and
the mode payload carries only the active effect's — mirrored in bytes 15, 19,
23 (doubled) and 27..71 (the value directly).

## 5. Limits of the gradient UI

The user's report, confirmed by the payloads: the Color Controller requires
**at least 2 stops**, and the end ones are **fixed at 0 and 100** — only the
middle ones drag. That is why the factory default has 3: red at 0, blue at 100
and a draggable green in the middle.

## 6. The macro's Playback Option

The dropdown exists and has four options: **Play Once**, **Play Multiple
Times**, **Toggle Macro On/Off**, **Continuous Repeat While Pressed**. The byte
that carries it has not been located yet.  [?]
