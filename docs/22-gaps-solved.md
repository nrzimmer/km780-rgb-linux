# Phase 2 gaps — what this round solved

> Session of 2026-10-16 (Linux), after bringing the captures over from Windows.
> Method: statistical analysis of the 11 captures (`tools/vary.py`) +
> decompilation of `Setting.dll` v2.04 with Ghidra 12.1.4 via REA.
>
> Confidence: **[C]** confirmed · **[I]** inferred · **[?]** open

Four items that `09-protocol.md` §10 listed as open are solved, and **three
claims in the specification were wrong**. The corrections come first.

---

## 0. Corrections to the specification

### 0.1 `0x0B`'s channel is in `buf[4]`, not in `buf[3]`  [C]

`09-protocol.md` §7.1 said:

> `0x02` | Ripple | `0x0B` ×3 channels, `buf[4]=0x00`
> `0x03` | Sonic  | `0x0B` ×3 channels, `buf[4]=0x01`

That assigns the `00`/`01` to `buf[4]` and implies the channel is in `buf[3]`,
as in `0x09`. **The two are swapped.** The six `0x0B` packets of the
`04-effects` capture, bytes 0 to 5:

    07 0b 02 00 01 00      Ripple, channel 1
    07 0b 02 00 02 00      Ripple, channel 2
    07 0b 02 00 03 00      Ripple, channel 3
    07 0b 02 00 01 01      Sonic,  channel 1
    07 0b 02 00 02 01      Sonic,  channel 2
    07 0b 02 00 03 01      Sonic,  channel 3

`buf[3]` is always `0x00`; `buf[4]` walks 1, 2, 3 (the channel); `buf[5]`
carries `0` for Ripple and `1` for Sonic.

Confirmed in the binary — see §1, where the source of all three fields appears.

**Consequence:** `0x0B` does **not** follow `0x09`'s `buf[3] = channel`
convention. Any implementation that generalizes "the channel goes in `buf[3]`"
to the effect commands writes to the wrong field.

### 0.2 `buf[5]` is not always zero  [C]

§9 item 11 said `buf[5]` and bytes 6–7 "came back zero in everything
captured". False for `0x0B`: that is where the Ripple/Sonic selector lives.

Sweeping the 6,277 SET/GET packets of the 11 captures, `buf[5]` is non-zero
only in `0x0B`. **Bytes 6 and 7 are still zero in all of them** — that part of
item 11 holds.

### 0.3 The brightness event is not `0x0B`  [C]

§6.6 said:

> `0x0B` remains the device→host event for brightness changes made with the
> keyboard's keys.

It is not. `0x0B` only appears as a **write** host→keyboard (Ripple and Sonic
data, §1). The brightness event is something else — see §2.2.

---

## 1. `0x0B`, `0x12`, `0x15` carry 144 bytes, one per slot  [C]

**This was the biggest gap**: `09-protocol.md` §10 said "the UI only applies a
global effect; **a capture cannot settle it**, only decompilation can". Settled
by decompilation.

All three commands come out of `FUN_100d7170` (`Setting.dll` v2.04), which
builds the packet like this:

    buf[0] = 0x07                      Report ID
    buf[1] = 0x0B | 0x12 | 0x15        depends on the effect, see below
    buf[2] = mode + 1                  1-based
    buf[3] = <parameter A>
    buf[4] = <channel>                 compared against 1, 2 and 3 throughout
    buf[5] = (effect != 2)             only in the 0x0B branch
    buf[8..151] = 144 bytes, one per LED slot

The dispatch by effect, at the end of the function:

| Effect ID | Effect | Command emitted |
|---|---|---|
| 2 | Ripple | `0x0B`, with `buf[5] = 0` |
| 3 | Sonic | `0x0B`, with `buf[5] = 1` |
| 7 | Changeling | `0x15` |
| 10 (`0x0A`) | Rain | `0x12` |

It matches exactly the effect IDs in the byte-10 table of the mode
(`09-protocol.md` §7.1) and what the captures show on the wire.

### The 144 bytes

The function builds a 144-byte array (`local_29c[0..0x8f]`) walking the **same
slot table** as `0x09` — the one at `0x1023b3b0`, documented in
`10-key-map.md`; the loop compares against `(&DAT_1023b3b0)[i*2]`, which is
each row's `dword0` (virtual-key), unrolled four at a time.

Each byte comes from an RGB triple in the app's memory, picked by channel:

    channel 1 -> triple[0]    channel 2 -> triple[1]    channel 3 -> triple[2]

and is stored with **`& 0xfc`** — quantization to **6 bits, shifted left by 2**,
the same as the `0x08` gradient (§7.3 of the specification).

Then the 144-byte array is copied into `buf[8]`:

    local_204  =  local_29c + 0x98  =  buf[8]

That is 0x24 = 36 dwords = 144 bytes. **The same layout as `0x09`: one byte per
slot, same order, starting at byte 8.**

### Why the capture showed all zeros

The code has both branches, and the one that zeroes is explicit:

    if (buf[3] == 0 && <state> != 1)   ->   memset(buf+8, 0, 144)
    else                               ->   copy the 144 bytes

In the captures, `buf[3]` was always `0` and the state was not the one that
releases the copy — the UI applies the global effect, so the app sends a zeroed
array. **The field exists and is documented; what is missing is a UI state that
fills it**, and that is a limitation of the UI, not of the protocol.

### Timing

The same as `0x09`'s: `Sleep(20)` after each channel, and after channel 3 a
further `Sleep(200)` in the `0x0B`/`0x15` branch and `Sleep(300)` in the `0x12`
branch.

---

## 2. The `0x83` event channel

The captures carry **8,278 messages** of 8 bytes on EP `0x83`. Three formats,
told apart by byte 0:

| byte 0 | count | Format |
|---|---|---|
| `00` | 6,185 | ACK |
| `f6` | 1,535 | key pressed |
| `02` | 558 | keyboard state |

### 2.1 The ACK: byte 4 is the mode  [C]

§1.3 marked bytes 3–7 as `[?]`. Of the 6,185 ACKs:

    00 00 <cmd> 00 <mode> 00 00 00

- byte 3 and bytes 5–7: **zero in all 6,185**  [C]
- byte 4: zero in 6,167; 1, 2 or 3 in the other 18  [C]

Those 18 are exactly the ACKs of `0x01` and `0x81`, and the value is the mode:

    SET 07 01 02 ...   ->   ACK 00 00 01 00 02      (mode 2 was selected)
    SET 07 81 00 ...   ->   ACK 00 00 81 00 01      (the active mode is 1)

That is, **`0x81` answers through the event channel, not the control IN** —
which the `CMDS` table of `tools/km780.py` already assumed, now confirmed.

### 2.2 `02 04 <mode> <type> <value>` — keyboard state  [C]

The specification (§9 item 12) had only `02 04 XX 08 01` and marked it `[?]`.
There are three types, in byte 3:

| Message | count | Meaning |
|---|---|---|
| `02 04 <mode> 08 01` | 547 | active mode / keyboard present |
| `02 04 02 09 <0..4>` | 5 | **brightness level**  [C] |
| `02 04 02 04 ...` | 6 | still `[?]` |

The brightness one settles itself: in the `08-closing` capture, changing the
brightness with the keyboard's keys produces five events in sequence with the
last byte at `03, 02, 01, 00, 04` — **the same 0–4 range as byte 11 of the mode
payload**, which is the intensity. This is the device→host brightness event,
not `0x0B`.

**Consequence for the Linux tooling:** brightness changes underneath the host
via the keyboard's keys, just like the mode. Whoever wants to follow it listens
to EP `0x83`; whoever does not listen must re-read with `0x82` instead of
assuming.

### 2.3 The physical matrix has 13 columns × 9 rows  [C]

Of the 1,535 `f6 01 <column> <row>` events: column from `01` to `0d` (13),
row from `00` to `08` (9). Bytes 4–7 always zero. 53 distinct pairs
observed — fewer than the 132 keys, because only part of the keyboard was
touched during the captures.

---

## 3. `0x08` is bigger than documented  [C]

§7.3 describes `0x08` as "69 bytes of data (8–76) = 3 channels × 23 steps".
Across the 37 `0x08` packets captured, **bytes well past 76 vary**:

    8-84   97-100   105-116   137-149   152   176   200

The last three are 24 apart from each other. The 69-byte description covers
the first block and not the rest: **`0x08` has more structure, not yet
mapped**  [?]

---

## 4. `0x0E` / `0x0F`: the observed shape  [I]

Five packets captured, all of them with almost everything zero. The non-zero
bytes form a clean pattern:

    0x0e f2261:    8:04  9:03  10:fc   34:fc   58:fc
    0x0f f2363:    8:03  9:03  10:fc   34:fc   58:fc   80:04
    0x0e f789:    79:03 80:03  81:fc  105:fc  129:fc
    0x0e f877:    79:06 80:03  81:fc  105:fc  129:fc
    0x0e f1975:  150:03 151:03 152:fc 176:fc 200:fc

Two regularities:

- inside a record: two header bytes, then `0xfc` every **24**
- the three observed bases — 8, 79, 150 — are **71** apart from each other

71 = 2 + 3×23 and 24 does not square with 23, so the two regularities still do
not reconcile into a single layout. `0xfc` is again the maximum value of the
6-bit quantization. The three bases are probably the **three effect slots per
mode**, the same ones as sub-pages `02`/`05`/`08` of `0x0A`.  [I]

These two commands do **not** come out of `FUN_100d7170` — the function that
builds them has yet to be found.

---

## 5. The send sites: there are 23, not 19 and not 37  [C]

§3.3 said the immediate sweep found 19 sites against 37 `push 0x108` in the
binary, and that the difference would be commands loaded from a register.

The xrefs of the `HidDevice_SetFeature` thunk (`Ordinal_7`, at `0x101e96ce`)
give the exact number:

    100d1ccc  100d1d80  100d1e27  100d1ee3  100d1f93  100d4ee8  100d502b
    100d6b06  100d7127  100d7670  100d7776  100d7d4e  100d9032  100d9117
    100d9b41  100da945  100db092  100db523  100db59a  100db621  100dbabb
    100dbbc3  100dbc82

**23 calls.** The 37 `push 0x108` include the `GetFeature` ones and other uses
of the size, so there were never 37 send sites. Six calls are still not
attributed to a command: `100d1d80`, `100d6b06`, `100d7776`, `100db523`,
`100db59a`, `100db621`.

---

## 6. How to reproduce

The capture analysis:

    tools/vary.py --all             which bytes vary, per command
    tools/vary.py 0x02              detail of one command

The binary analysis, with REA/Ghidra over
`external/gds/rgb-v2.04/payload/app/Setting.dll`
(sha256 `314acb2f…`, image base `0x10000000`):

    xrefs 0x101e96ce                the 23 send sites
    procedure_pseudo_code 0x100d7170   the 0x0B/0x12/0x15 function

---

## 7. What is still open

| Item | State |
|---|---|
| the UI state that fills the 144 bytes of `0x0B`/`0x12`/`0x15` | a UI limitation, not a protocol one |
| the function that builds `0x0E` / `0x0F` | not located |
| the structure of `0x08` beyond byte 76 | `[?]` |
| the `02 04 02 04 ...` event | `[?]` |
| 6 send sites with no command attributed | `[?]` |
| the macro's Playback Option | byte not located |
| byte 108 vs byte 119 | two toggles, which is which still unsaid |
| LED Sleep | never captured |
