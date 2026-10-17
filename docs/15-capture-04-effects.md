# Capture `04-effects` — effect catalog, `0x01`, duration and direction

Source: `captures/04-effects.pcapng` / `.txt` (1212 packets).
Action: switch to profile 2, walk the 8 effects in the list, go back to the
first, change duration from 3 to 7, change direction twice.
Confidence: **[C]** except where marked.

## 0. Note: the app crashes on profile 1

Enabling effects on profile 1 kills the app. Windows records
`APPCRASH / Setting.dll / 0xc0000094` (**integer division by zero**) at offset
`0x0004aa99` — with the DLL's default base, **`0x1004aa99`**, a direct target
for Ghidra. Switching to profile 2 works around it. The capture shows that **no
packet goes out** before the crash, so the bug is the app's, not the
protocol's.

## 1. `0x01` — select profile  [C]

    07 01 02 00 00 00 00 00  00 ... (rest zeroed)
          ^^ profile number, 1-based

The same convention as `0x02`/`0x82`. It settles section 4 of the
specification.

## 2. The active effect is byte 10 of the profile  [C]

Switching effect in the UI changes **a single byte** of the profile: byte 10.
The names come in ASCII through `0x0A` page `0x1c`, written right after the
selection, which pairs ID with name unambiguously:

| byte 10 | Effect |
|---|---|
| `0x01` | Wave |
| `0x02` | Ripple |
| `0x03` | Sonic |
| `0x04` | Breathing |
| `0x07` | Changeling |
| `0x09` | Cycle |
| `0x0A` | Rain |

`0x05`, `0x06` and `0x08` did not appear. `docs/27` §7 identifies
**`0x08` as "Checkpoint"** -- it exists in the DLL, with the profile `.chk`,
and the v2.04 UI did not offer it. `0x05` and `0x06` have no branch in the
builder, and probably do not exist. Byte 10 was `0x00` when the lighting
was static, so **`0` is probably
"no effect"**.  [I]

## 3. Duration and direction  [C]

Changing **duration from 3 to 7** in the UI:

    bytes 15, 19, 23        06 -> 0e     (2x the UI value)
    bytes 27,31,...,71      03 -> 07     (1x the UI value)

Changing **direction** twice:

    bytes 12, 13, 14        03 -> 00 -> 01

So the profile's configuration block has **4-byte records starting at byte
12**, 15 of them (bytes 12-71). Bytes 12-14 hold the direction (written
identically in all three positions — three zones?) and the 4th byte of each
record holds the duration. The first three records hold twice the UI value. [I]

Direction values seen: `0`, `1`, `3`.

## 4. One helper command per effect family  [C]

Besides `0x08` and `0x02`, each effect fires commands of its own:

| Effect | Extra commands |
|---|---|
| Wave (`01`), Breathing (`04`) | none |
| Ripple (`02`) | `0x0B` x3, `buf[4]=0x00` |
| Sonic (`03`) | `0x0B` x3, `buf[4]=0x01` |
| Cycle (`09`) | `0x0E` (zeroed payload) |
| Changeling (`07`) | `0x0E` with `04 03 fc` in bytes 8-10, then `0x15` x3 |
| Rain (`0A`) | `0x0F` with `03 03 fc` in bytes 8-10, then `0x12` x3 |

The "x3" commands use `buf[3] = 1, 2, 3` — **the same R/G/B channel convention
as `0x09`**. In this capture they all came with a zeroed payload (no custom key
selection), so they are per-key masks/colors not yet exercised.

This also explains `0x0B`: it is not the brightness command `11-firmware.md`
assumed, it is the data pair of the Ripple effect.

## 5. `0x0A` page `0x1c` — the effect database, with ASCII names  [C]

Records of 3 subpages (`00`/`01`/`02`, `03`/`04`/`05`, `06`/`07`/`08`, ...):

| Subpage | Content |
|---|---|
| `+0` | **name in ASCII**, e.g. `Default Wave`, `Default Ripple` |
| `+1` | zeroed |
| `+2` | **parameters** |

Parameters (`+2`), 16 useful bytes:

    00 <duration> <direction> 03  ff 00 00 64 00  00 ff 00 64 32  00 00 ff 64 64  00
       ^^^^^^^^^^ ^^^^^^^^^^^     \__ R __/ \pos/ \__ G __/ \pos/ \__ B __/ \pos/

- byte 1 = duration (the UI value: `03` or `07`)
- byte 2 = direction (`00`, `01`, `03`)
- then **three gradient stops**, each one `RGB` + `0x64` (=100, intensity) +
  position (`0x00`, `0x32`=50, `0x64`=100)

That is: red at 0%, green at 50%, blue at 100% — the effects' default rainbow.
[I] on the exact split of the last two bytes of each stop.

Page `0x00` stores profile and mode names: `Profile1`, `Mode1`, `Mode3`,
`Mode4`.

## 6. Enabling effects clears `0x0A` pages  [C]

On turning effects on, the app writes **zeros** into pages `0x09` to `0x10` of
`0x0A`, 16 subpages each (~120 packets). It is an area format, not useful data.

## 7. New commands in this capture

Joining the known set: `0x01` (solved), `0x0B`, `0x0E`, `0x0F`, `0x12`,
`0x15`. With the earlier ones, the set observed on the wire reaches:

    0x01 0x02 0x07 0x08 0x09 0x0A 0x0B 0x0D 0x0E 0x0F 0x12 0x15 0x16
    + reads 0x81 0x82 0x89 0x8A
