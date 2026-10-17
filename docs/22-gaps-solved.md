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

### 2.3 The physical matrix is 15 columns × 9 rows  [C]

> **Corrected.** This section said 13 × 9, deduced from the range of values
> appearing in the captures (column `01`–`0d`). The range was short: **column 0
> was never pressed at all on Windows** — those are the G keys, MR and the
> modes, which do not type. Mapped on hardware, it is **15 × 9**, with 122 of
> the 135 positions occupied. Full map in `25-physical-matrix.md`.

Of the 1,535 `f6 01 <column> <row>` events in the captures: 53 distinct pairs,
fewer than the 122 keys, because only part of the keyboard was touched.

---

## 3. `0x08`: the 69 bytes are right, but there are blocks after them  [C]

> **A correction to an earlier version of this section**, which said `0x08` "is
> bigger than documented". That came from looking at the **variance** across
> the 37 packets, not at a whole packet. Looking at a complete packet, the
> description in §7.3 is right.

The gradient block is exactly as documented. A real `0x08`, cut into lines of
23 from byte 8 on, aligns by itself:

     8: fc fc fc fc e0 e0 e0 e0 e0 e0 e0 c4 c4 c4 c4 60 60 .. .. 7c 7c 7c 7c   <- R
    31: fc fc fc fc 80 80 80 80 80 80 80 .. .. .. .. .. .. .. .. 7c 7c 7c 7c   <- G
    54: fc fc fc fc 84 84 84 84 84 84 84 08 08 08 08 84 84 fc fc fc fc fc fc   <- B
    77: (zero to the end)

**3 channels × 23 steps, planar, bytes 8–76.** Confirmed.

What the variance showed is something else: **31 of the 37 packets stop at byte
76, and 6 carry on.** Of those six:

- one (`04-effects` f2057) carries a **second 69-byte block at 77–145**, with
  the same cyclic shape as the first
- five carry only three `fc` bytes, at **152, 176 and 200** — 24 apart

So `0x08` has **more than one block**, and not every block is a 23-step
gradient: the one based at 150 has a stride of 24. How many blocks there are
and what selects the format is still `[?]` — but the gradient block is not
wrong.

---

## 4. `0x0E` / `0x0F`: solved  [C]

The shape on the wire seemed to have two regularities that did not fit — a base
every **71** bytes and values every **24**. They do fit: the 24 was an artefact.

### What the binary said

Sweeping the 6,906 bytes of `FUN_100d5060` for the structural constants (71,
69, 72, 24, 23), only **23** appears — as `imul r,r,23`, `cmp r,23` and
`add r,23`, and in a floating-point context (`fild`, `fadd`, `fstp`), which is
the gradient interpolation. **`24` does not exist in the function.** So a stride
of 24 could not be real.

### The layout

    effect slot = 71 bytes = [2 of header][3 channels x 23 steps, planar]
    the packet's three slots sit at 8, 79 and 150   (8 + 71k)

The "+24" was the sum of two things: a planar stride of **23** plus a step of
**+1** in the index within the channel. The captured packets carry a value at

    channel 0, step 0  ·  channel 1, step 1  ·  channel 2, step 2

that is, the diagonal — R at step 0, G at step 1, B at step 2. In absolute
offsets that gives 10, 34, 58: a difference of 24, without a 24 existing
anywhere.

### It checks out against all five packets

| Packet | slot | base | header | values |
|---|---|---|---|---|
| `0x0E` f2261 | 0 | 8 | `04 03` | ch0[0] ch1[1] ch2[2] = `fc` |
| `0x0E` f789 | 1 | 79 | `03 03` | same |
| `0x0E` f877 | 1 | 79 | `06 03` | same |
| `0x0E` f1975 | 2 | 150 | `03 03` | same |
| `0x0F` f2363 | 0 | 8 | `03 03` | same (and slot 1 with header `00 04`) |

**Five out of five.** And the slot used changes from packet to packet, which
confirms that the three bases are the **three effect slots per mode** — the same
ones as subpages `02`/`05`/`08` of `0x0A` (§7.4 of the specification).

The second header byte is `03` in almost all of them and `04` in one; the first
varies between `03`/`04`/`06`. By position and by range, these are **duration
and direction**, as in `0x0A` — but that is still `[I]`, it has not been
exercised.

### What this means in practice

`0x0E` and `0x0F` carry a **gradient per effect slot**, not per-key data. They
are siblings of `0x08`, not of `0x0B`/`0x12`/`0x15`. That makes sense: the same
function builds all three, and the per-key ones come out of `FUN_100d7170`.

Since the color tooling does not need to write effects, this is specification
completeness, not a blocker.

### What was left out

A single `0x08` packet (`04-effects` f2057) has its first block **zeroed** and a
complete gradient starting at byte 77, which aligns with neither 69- nor
71-byte blocks. It is the only one of the 37 like that. It stays `[?]` — one
anomalous packet does not bring down a model that explains the other 41.

## 5. The send sites: there are 23, not 19 and not 37  [C]

§3.3 said the immediate sweep found 19 sites against 37 `push 0x108` in the
binary, and that the difference would be commands loaded from a register.

The xrefs of the `HidDevice_SetFeature` thunk (`Ordinal_7`, at `0x101e96ce`)
give the exact number:

    100d1ccc  100d1d80  100d1e27  100d1ee3  100d1f93  100d4ee8  100d502b
    100d6b06  100d7127  100d7670  100d7776  100d7d4e  100d9032  100d9117
    100d9b41  100da945  100db092  100db523  100db59a  100db621  100dbabb
    100dbbc3  100dbc82

**23 calls.** The 37 `push 0x108` include the `GetFeature`s and other uses of
the size, so there were never 37 send sites.

---

## 5b. Three missing read commands — and one never seen before  [C]

Three of the unattributed calls are all in **`FUN_100db460`** (the function the
8-byte header came from, §2 of the specification). Extracting its immediates:

    0x100db4e6   mov byte [esp+0x320], 0x07      <- buf[0], Report ID
    0x100db4ee   mov byte [esp+0x321], 0x92      <- buf[1]   -> send at db523
    0x100db4fd   mov byte [esp+0x323], 0x00      <- buf[3]
    0x100db565   mov byte [esp+0x321], 0x95      <- buf[1]   -> send at db59a
    0x100db5e4   mov byte [esp+0x320], 0x07
    0x100db5ec   mov byte [esp+0x321], 0x8b      <- buf[1]   -> send at db621

| Read | Matching write | Previous state |
|---|---|---|
| `0x8B` | `0x0B` (Ripple / Sonic) | "cited in the timing, never seen on the wire" |
| `0x92` | `0x12` (Rain) | same |
| **`0x95`** | `0x15` (Changeling) | **did not exist in any document** |

They are exactly the reads of the three commands whose per-key payload was
solved in §1 — which **confirms the bit-7 rule for three more pairs** and takes
the confirmed set from 6 to 9.

None of them appears in the captures because `FUN_100db460` does not run on the
paths that were exercised. **They are confirmed in the binary, not on the wire**
— lower confidence than the four that answer in a capture, and that is why they
sit in `tools/km780_probe.py` in a separate list, not on the default path.

### `0x96` and the bootloader  [I]

With `0x95` being a valid read, the next byte is `0x96` — and `0x96` would be
the read of `0x16`, the command that **opens the session**, the init's first
message. `0x96` is one of the bytes the old sweep probed.

There is no proving which byte it was, but the upper limit of the read space
landing at `0x95` and the sweep starting to fail at `0x96` is too consistent to
ignore. **Treat `0x96` as the prime suspect** and do not probe it.

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
