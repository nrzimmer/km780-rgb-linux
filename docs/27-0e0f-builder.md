# `0x0E` / `0x0F` via the builder, and the DLL's effect catalog

Source: `FUN_100d5060` of `Setting.dll` v2.04 (6,911 B), decompiled in full.
It is the function `docs/09` points at as the build site of `0x0E`
(`0x100d5c69`) and `0x0F` (`0x100d5983`). `docs/22` §4 deduced the layout from
the **shape on the wire**; here it comes out of the **code**, and that promotes
three things from `[I]` to `[C]` and corrects one.

## 1. The envelope, confirmed in the code  [C]

The function's final stretch builds the packet in contiguous stack variables:

    local_e10  = 7                        -> buf[0]  Report ID
    uStack_e0f = 0xe  or  0xf             -> buf[1]  command
    cStack_e0e = <param> + 1              -> buf[2]  mode, 1-based

A third independent confirmation that `buf[0]=0x07` is the Report ID and that
the mode in byte 2 is 1-based. After that the function calls `FUN_100d1490`,
`Ordinal_7()` and **`Sleep(500)`** — half a second of waiting after writing an
effect, which the official app respects and we did not know about.

## 2. The 71-byte slot, with exact bases  [C]

The three 23-byte copies (`for (i=5; i--;)` of dwords, plus 2+1) go to offsets
that, translated from the frame to the buffer, give:

    base+0            stop count
    base+1            single scalar (see section 4)
    base+2  .. +24    channel 0  (23 bytes)
    base+25 .. +47    channel 1  (23 bytes)
    base+48 .. +70    channel 2  (23 bytes)

Total **71 bytes**, exactly as `docs/22` §4 deduced — and now with the channel
bases pinned down, which the shape on the wire did not give. The packet's three
slots sit at 8, 79 and 150.

This explains `docs/22`'s "diagonal" without appealing to anything: a value at
`channel 0 step 0`, `channel 1 step 1`, `channel 2 step 2` lands at absolute
offsets 10, 34, 58 — the difference of 24 that looked like a structural stride
is just `23 + 1`, planar plus the index. **It checks out across all 5 packets**
(`docs/22` §4).

## 3. The 23 is a ceiling, not a coincidence  [C]

    if (uStack_7450c < 0x17) uStack_7450c = uStack_7450c + 1;

`0x17` = 23. The stop counter is **limited by the channel's length**, and it is
the same `uStack_7450c` that:

- bounds the loop filling the stops (`while (i < uStack_7450c)`)
- is written into `base+0`

So **`base+0` is the number of gradient stops, at most 23**. The captured
headers (`03`, `04`, `06`) are stop counts; the packets have 3 stops with color
and the rest black, which is consistent with an R→G→B gradient edited in a UI
with 3, 4 or 6 points.

## 4. What `docs/22` §4 got wrong

> "The header's second byte is `03` in almost all of them and `04` in one; the
> first varies `03`/`04`/`06`. By position and by range, they are **duration and
> direction**, as in `0x0A`."

**They are not.** The first is the stop count (§3 above). The second is
`uStack_74510`, a **single scalar** copied straight through, unrelated to the
first and not per stop — it is not a duration/direction pair. What it means is
still `[I]`, but now we know what it is *not*, and that it is one value coming
from the caller.

## 5. The quantization explains the `fc`  [C]

The loop that fills the stops, for each 5-byte entry:

    channel0[i] = ((c0 * pct) / 100 >> 2) * 4
    channel1[i] = ((c1 * pct) / 100 >> 2) * 4
    channel2[i] = ((c2 * pct) / 100 >> 2) * 4

That is: the component scaled by a **percentage** and then quantized to
**multiples of 4** — 6 effective bits per channel on the effect path. Hence the
ubiquitous `fc` in the captures: `fc` = 252 = `255` at 100%, rounded down to a
multiple of 4. The value never reaches `ff` by construction.

The input is a **5-byte** record: three color components, the percentage, and a
fifth byte unused on this path.  [I] on the fifth.

## 6. Wave and Ripple go through `0x08`, same function  [C]

In the same builder, the branch for effects 1 and 2 writes `uStack_e0f = 8`. So
`0x08`, `0x0E` and `0x0F` are **the same gradient format**, built by the same
code, varying only the command byte per effect family. It confirms the reading
in `docs/22` §4 ("they are siblings of `0x08`") and the table in `docs/15` §4.

## 7. The DLL's effect catalog: an effect the UI never showed  [C]

The function's first half dispatches by effect index and picks the profile
file's extension. The index is the **same** as byte 10 of the mode payload
(`docs/15` §2):

| Index | Extension | Name in the UI | Helper command |
|---|---|---|---|
| 1 | `.Wave` | Wave | `0x08` |
| 2 | `.rip` | Ripple | `0x08`, and `0x0B` x3 |
| 3 | `.rea` | **Sonic** (internally: *Reactive*) | — |
| 4 | `.bre` | Breathing | — |
| 5 | — | not in the function | — |
| 6 | — | not in the function | — |
| 7 | `.chg` | Changeling | `0x0E`, and `0x15` x3 |
| 8 | `.chk` | **Checkpoint** | `0x0E` |
| 9 | `.cycle` | Cycle | `0x0E` |
| 10 | `.rain` | Rain | `0x0F`, and `0x12` x3 |

Two findings:

- **Index 8 is "Checkpoint"**, and `docs/15` §2 records that `0x08` *"did not
  appear"* in the UI during the capture of the 8 effects. The name comes from
  the function's `.chk` crossed with the UTF-16 string
  `Checkpoint`/`CHECKPOINT` in `.rdata` (next to `WAVE`, `RIPPLE`,
  `CHANGELING`, `CYCLE`, `RAIN`). It is a ninth effect, one the hardware may
  accept in byte 10 and the v2.04 UI did not offer.
- **Index 3 is `.rea`, for *Reactive***, but the UI calls it **Sonic**. The DLL
  also has `REACTIVER` in its strings. It is a UI label diverging from the
  internal name, not two effects.

`5` and `6` have no branch in the function — consistent with their never
appearing in a capture. They probably do not exist.

## 8. The caller: why the parameters got lost  [C]

`FUN_100d5060` has two callers, `0x100d1fd0` and `0x100d24c0`. Both do the same
thing before calling:

    puVar3 = &DAT_102fe6b8;          // the global configuration blob
    puVar4 = auStack_73698 + 1;
    for (i = 0x1cd9d; i != 0; i--)   // 118,173 dwords = 472,692 bytes
      *puVar4++ = *puVar3++;
    FUN_100d5060();

**The entire configuration blob goes by value on the stack**, 472 KB per call.
That is why Ghidra recovers neither a signature nor an origin for
`uStack_7450c` and `uStack_74510`: they are neither computed nor passed as
arguments, they are **fields of that blob at a fixed offset**, read from a
region the decompiler attributed to the wrong frame.

Practical consequence: **pinning down `base+1`'s offset requires the assembly**,
not the pseudocode. The pseudocode has already given everything it can here.

The blob is a per-device array, with a stride of `0x264c4` = **157,380 bytes**
(the same stride appears as a dword index, `0x9931 * 4`).

## 9. The per-key data rule, and seven out of seven  [C]

After building the gradient, the caller enters the `FUN_100d7170` loop — the
per-key data builder (`docs/22` §1) — **only** if the effect index is `2`, `3`,
`10` or `7`, and the loop runs **3 times**:

    if (ef == 2 || ef == 3 || ef == 10 || ef == 7) {
      i = 1; do { ...; FUN_100d7170(); i++; } while (i < 4);
    }

This predicts which effects fire three helper packets. Against the table in
`docs/15` §4, built from the capture and unrelated to this code:

| Effect | Predicted | Captured |
|---|---|---|
| Ripple (2) | 3 packets | `0x0B` x3 |
| Sonic (3) | 3 packets | `0x0B` x3 |
| Changeling (7) | 3 packets | `0x15` x3 |
| Rain (10) | 3 packets | `0x12` x3 |
| Wave (1) | none | none |
| Breathing (4) | none | none |
| Cycle (9) | none | none |

**Seven out of seven, including the three negatives.** And the "x3" is not a
coincidence with the R/G/B channels: it is the loop's count.

## 10. Checkpoint on the hardware: evidence against  [I]

Test with `tools/km780effect.py 2 8 5 6`, mode 2 (static, brightness 4):

| byte 10 | what the keyboard did |
|---|---|
| `8` (Checkpoint) | all red |
| `5` (no branch in the builder) | all red |
| `6` (no branch in the builder) | dark |

The firmware **does tell** the three values apart — `6` goes dark, the other
two do not — so it is not a single "default" for the unknown. But `8` did
exactly the same as `5`, and `5` is precisely an index that does not exist in
the app. That weighs against Checkpoint being a live effect on FW 77.

Caveat: mode 2 was static, so its effect slots are probably zeroed, and a
legitimate effect might not animate there either. **The control is missing**:
run Wave (index 1) on the same mode. If Wave animates and `8` does not, that
settles it.

## 11. What stays open

- `base+1`: a scalar, originating in the caller.  [I]
- The fifth byte of the stop record.  [I]
- **Checkpoint is testable on hardware**: write `8` into byte 10 of a mode via
  read-change-write, and look. Same path as `km780ctl.py effect`, reversible.
  Likewise `5` and `6`, to confirm they do nothing.
- The `Sleep(500)` after an effect suggests the keyboard needs time to digest an
  effect packet. Nothing of ours writes effects yet; when it does, respect that.
