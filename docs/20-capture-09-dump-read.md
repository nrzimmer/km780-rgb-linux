# Capture `09-dump-read` — the read side, with memory full

Source: `captures/09-dump-read.pcapng` / `.txt`.
Action: close and reopen `App.exe` with the capture running, then switch
M1 -> M2 -> M3 -> M1 to force each mode to be re-read.
Confidence: **[C]**.

## Why this capture exists

In the earlier init captures (`12-capture-01-open.md`) **every** read answer
came back pure `0xFF`: the keyboard had just been reflashed and the memory was
empty. The whole specification was deduced from the **writing** side.

After captures 05 to 08 the keyboard had real content, and content we created
and understand ourselves. Reopening the app makes it dump that memory back,
which allows comparing **read bytes with known meaning**.

## 1. Read and write are the same structure  [C]

Comparing the last `0x02` written to mode 2 (end of capture `08`) with the
`0x82` read now, across the 264 bytes:

    byte 3:  written 00  /  read 03
    all the other 263 bytes: IDENTICAL

In other words, **the keyboard returns exactly what was written**, and the only
field that differs is byte 3, already documented as a status flag (`0x00` on
write, `0x03` in the answer).

This defines the safe operating model for the Linux tooling:

> read the mode with `0x82`, change the desired byte, write it back with `0x02`.

There is no need to understand the fields still marked `[?]` in order to touch
the ones marked `[C]` — the unknown ones travel intact.

## 2. What the app reads, and what it does not  [C]

| Command | Target | Answer |
|---|---|---|
| `0x8A` page `00` sub `00`–`03` | names | `Profile2`, `Mode1`, `Mode2`, `Mode3` |
| `0x81` | active mode | mode 2 |
| `0x82` | modes 1, 2, 3 | full payload of all three |
| `0x89` | LED matrix | three channels, per mode |
| `0x8A` page `1b`/`1c`/`1d` | each mode's database | effect names and parameters |

**The app never reads page `0x09`** — the macro and text database. It only
writes to it. Whether the read works was simply not exercised; it is a
one-command test to run on Linux (`0x8A` with `buf[2]=0x09`), and it is safe
because it is a read of an already-confirmed command.

## 3. The matrix read back  [C]

`0x89` returns the matrix in the same format as `0x09`: one channel per
packet, 144 slots. Mode 2 came back with `ff fb 2f` in practically every slot
— the yellow that had been applied last.

**Ten slots came back dark**, and all of them correspond to keys that do not
exist on the physical layout:

    16, 32, 35, 68, 71, 80, 85, 88, 91, 129

By the HID usages in `10-key-map.md`: `0x64` (nonUS backslash), `0x91`,
`0x90`, `0x8a`, `0x8b`, `0x88`, `0x89`, `0x85`, `0x87` (international JIS/Lang
keys) and `0x32` (nonUS hash). In `13-capture-02-brightness.md` only five were
dark; now there are ten. That refines the earlier observation: **the set of
slots with no physical key is larger than the old capture suggested.**

## 4. Smaller confirmations

- `0x8A` page `1d` (mode 3) returned `Default Wave` and the parameter block
  `00 07 01 03 ff 00 00 64 00 ...` — duration 7, direction 1 (Down), 3 stops.
  The same format as the write, confirming section 7.4 from the read side.
- `0x8A` page `1b`/`1c` sub `0a` returned the pair `01 1f`, same as written.
- `0x81` answered mode 2, which was the active one.
