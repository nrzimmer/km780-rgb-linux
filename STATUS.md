# Project state and next step

> Updated 2026-10-17. This file is the resume point: it says where things
> stand, what to do next, and the rules learned the hard way.
> `START-HERE.md` is the **historical** handoff from the Windows session; this
> one succeeds it.

## The goal, and how much of it is left

Control the KM780's RGB on Linux. **Done.** The protocol is documented and
validated on hardware, and the tooling was exercised end to end on 2026-10-17:
color write, brightness, full repaint and restore, all checked by diff against
a prior portrait.

| | |
|---|---|
| Protocol specification | `docs/09-protocol.md`, canonical |
| Visual view | `docs/protocol.html` — opens in a browser, or claude.ai/artifact/K9mu4VENac49xSE2cR3YXM (v12, up to date with `docs/30`; the published artifact is still v11) |
| Color tooling | `tools/km780ctl.py` — **exercised end to end on hardware** |
| Hardware-free tests | `tools/test_km780.py` — 24, all passing |

## DONE: the Rust GUI app, in `app/`

The user asked for a real app — GUI, Rust, buildable for Windows, to replace
G.SKILL's. **It is written, running and validated on hardware.**
`docs/29` was the plan; **`docs/30` is what happened**, including three things
the plan got wrong.

    cargo build --release                                  # Linux
    cargo build --release --target x86_64-pc-windows-gnu   # the .exe

It does: reads mode/brightness/effect/colors; draws the keyboard in the
manufacturer's layout; edits color, brightness, effect and key function in a
local buffer; `Apply` writes, `Discard` re-reads, `Undo` goes back. Profiles
saved in `~/.config/km780/profiles/`. All four items of `docs/29`'s "what comes
after" are done.

Without the GUI, for diagnostics and scripting:

    km780 --probe               reads everything and prints it
    km780 --brightness 2        read-change-write + diff of the whole payload
    km780 --effect 4            same, byte 10
    km780 --function Timer=f13  remaps in all 3 modes, recording the previous value

The keyboard layout does **not** come from `physical-matrix.json` (which is the
electrical matrix): it comes from the official app's `KeyboardButtons.xml`, in
`external/gds/`. 122 keys, the same number `docs/25` counted in the matrix.

The crate's interface, identifiers and comments are **in English**, like the
rest of the repository. The `SLOTS` table keeps the canonical names from
`docs/10` — only the `Brightness` key is shortened when drawing, via
`slots::display_name`.

Start by reading `docs/30`.

### What the app does NOT do yet

In order of size. Nothing here is blocked by missing reverse engineering — what
is missing is code, except where marked.

**Macros — nothing done.** This is the biggest block. The protocol is
documented (`0x05` = content, 3-byte records `[flag][delay ×10 ms][HID
usage]`, with an extended field in the flag's nibble for delays up to ~41 s;
the name goes to the `0x0A` base, page `0x09`; the key receives the code
`0xB4`). The app does not write, edit, assign or list any macro. Recording
with the MR key happens inside the keyboard and needs no host — but editing
what was recorded does.

**Editing effects — only the on/off is done.** Today the app changes byte 10 and
that is it; the animation comes out with whatever is already stored. Missing:

- **duration** (bytes 15–71, the 4th of each 4-byte record) and **direction**
  (bytes 12–14). Documented, it is read-change-write.
- **the per-effect gradients** — `0x0B` (Ripple/Sonic), `0x0E` (Changeling,
  Checkpoint, Cycle), `0x0F`/`0x12` (Rain), `0x15` (Changeling). The 71-byte
  structure per slot is confirmed (`docs/27`: stop count, scalar, 3 channels ×
  23 bytes, quantized to multiples of 4). **Mind `buf[4]`**: these commands put
  the channel in `buf[4]`, not in `buf[3]` like `0x09`.
- **per-key data of the effects** — this one does have a reverse-engineering
  gap: `START-HERE.md` records that the official UI only applies a global
  effect, so it cannot be exercised through it. `[?]`

**Other mode fields, all documented and none in the app:**

- **polling rate**, byte 8 (1/2/4/8 ms = 1000/500/250/125 Hz), `docs/14`
- **the three toggles** in bytes 107, 108 and 109 — 109 is the Windows key
  lock, confirmed on the hardware (`docs/23` §8). Which is which in the other
  two is still missing. `[?]` partial
- **Macro and Text as a key function** (`0xB4`, `0xB8`): the app's remapping
  offers only F13–F24 and raw hex. Text, by the way, **does not work** — the
  content never reaches the keyboard, only the name, so a key with Text only
  works with the G.SKILL app running.
- **LED Sleep** — never identified. `[?]`

**A protocol gap closed along the way:** `0xAB` = Volume Up, measured on
2026-10-17 by assigning the code to the Timer key from the app itself. It was
`[I]`. `0xAA` (Volume Down) is still untested directly. See `docs/09` §5.2.

**App quality, not protocol:**

- the 12 nameless tail slots (132–143) are not reachable by name even from the
  CLI, only by number
- no command line for profiles (only the GUI saves and loads)
- no packaging, no systemd unit to restore the color at boot

**And the Windows block**, which needs a Windows machine: which device path to
open among the several top-level collections, and whether Windows allows opening
the collection with read access. See `docs/30`.

## `pyusb` is in the venv, not in the system

Every command runs with the venv's interpreter:

    venv/bin/python tools/km780ctl.py info

`tools/km780ctl.py info` on its own fails with `ModuleNotFoundError: No module
named 'usb'` — the shebang is `/usr/bin/env python3` and Fedora does not have
pyusb installed globally. The venv (pyusb 1.3.1) is not versioned; recreate it
with `python3 -m venv venv && venv/bin/pip install pyusb`.

## End-to-end exercise: done, eight steps, all checked

Run on 2026-10-17 with the keyboard plugged in, active mode 2 (static):

| Step | Result |
|---|---|
| `info` | active mode 2; brightness 4/4/4; Wave effect only on 3 |
| `backup state.json` | the 3 modes, colors + payload |
| `-n all blue` | a dry run, nothing written |
| `set F5=ff0000 G1=blue` | slots 45 and 0; **diff: exactly 5 bytes**, across the 3 channels, mode payloads intact |
| `colors` | the re-read confirms `F5=ff0000`, `G1=0000ff` |
| `brightness 2` | **diff: only byte 11 of mode 2**; colors intact |
| `all 101010` | 144 slots |
| `restore state.json` | **diff against the initial portrait: zero difference** across the 3 modes, colors and payload |

The two diffs in bold are what matters: a write touches only what was asked for,
and read-change-write preserves the unknown fields on real hardware — not just
in `test_km780.py`'s simulator.

## NEXT STEP: convenience, or closing a protocol gap

The declared goal is met. Two directions, neither blocking the other:

- **Usability** — saved themes **done** (the app's profiles, `docs/30`). Missing:
  restoring at boot (a systemd user unit) and packaging. Needs no more reverse
  engineering.
  Already done here: **remapping a key in the keyboard's memory** (`docs/28`),
  which solves "give a key a function with no service". Timer is on F13 in all
  three modes.
- **Protocol** — the gaps in the table below. `FUN_100d5060` has already been
  decompiled (`docs/27`): it closed `0x0E`/`0x0F` and revealed a ninth effect,
  **Checkpoint** (byte 10 = `8`), which the v2.04 UI did not offer. Testing it on
  hardware is the next cheap and reversible step — write `8` into byte 10 of a
  mode via read-change-write and look at the keyboard.

## Hardware state

- FW 77, `28da:1101`, no root needed (a `uaccess` ACL on the USB node)
- **This unit has LEDs with the R stuck** (only the brightness varies). It shows
  in static color, not in animation; it is a few keys. Healthy ones checked:
  `F1 F2 F3`, `1 2 3 4`; suspect: `P`, `[`. In a color test, check by
  **re-reading the payload** (`0x89`), not by eye. See `docs/27` §10.
- Backup of the original state in `backup-first-contact.json` (outside the index)
- Verified at the end of the last session: payload and colors **identical** to
  the backup

## What is still missing in the protocol (nothing blocks the tooling)

| Gap | How it comes out |
|---|---|
| ~~internal layout of `0x0E` / `0x0F`~~ | **closed** — `docs/27`, via the builder |
| the role of `0x07` (`07 07 00 15`) | the only command on the wire with no build site located |
| the macro's Playback Option | decompilation |
| LED Sleep: do the parameters reach the keyboard? | `SleepMode`/`SleepMin`/`SleepEffect*` exist in the app; whether they go on the wire is unknown |
| which of the inert ones is `LightDisable` | decompilation only, and it gives the **name**, not the behaviour |
| `base+1` of the effect slot | **only through the assembly** — the caller passes 472 KB of config by value on the stack, `docs/27` §8 |
| the 5th byte of the stop record | same |
| ~~the effect byte 10 = 8~~ | **Checkpoint, reactive** — it was already in `docs/16` §4; hardware corroborates |
| ~~characterizing effect **6**~~ | **done and discarded** — real, cumulative, useless. `docs/27` §10 |
| what byte 10 = `5` is | no branch in the DLL and no reaction in the firmware  `[?]` |
| the `02 04 M 04 …` event | 6 occurrences, no context |

## Operating rules, learned from two scares

1. **Never probe an unobserved `0x8x` command.** It was a sweep between `0x96`
   and `0xAF` that put the keyboard into bootloader mode. The confirmed read
   space ends at `0x95`, and `0x96` would be the read of `0x16` — the prime
   suspect.
2. **Whoever sends `0x16` must guarantee a USB reset on the way out.** The
   session it opens has no closing command, and the keyboard may stop typing.
   `K.Session` does that in a `finally`. Do not depend on Ctrl+C: it comes from
   the keyboard the test may knock out.
3. **Use the versioned tools, not an inline script.** Both of the session's
   incidents came from throwaway code written in the terminal — one opened a
   session with no undo, the other did 792 reads in a loop by mistake and left
   the keyboard flashing. The tools in `tools/` restore in a `finally` and read
   once.
4. **Read the mode, change the byte, write it back.** Never build a mode payload
   from scratch: the unknown fields travel intact through the round-trip, and
   that is what makes it possible to touch what you know without understanding
   the rest.
5. **Backup first, diff after.** It was a diff against a prior portrait that
   found byte 109 (the Windows key lock) first time, without writing anything.

## Document map

    09   canonical protocol specification
    10   key -> LED slot map (132 slots)
    22   gaps closed by capture analysis + decompilation
    23   hardware validation, byte 109, the three toggles
    24   0x16 and the session that nothing closes
    25   the physical matrix 15x9, 122 keys
    26   the 18 special keys, slot by slot
    27   0x0E/0x0F via the builder, effect catalog, effect 6
    28   remapping a key in the keyboard's memory, with no service
    29   the Rust app: decisions and starting point
    30   the app's v1 live, and the three corrections to doc 29  <-- start here
    12-21  one analysis per capture
    01-08, 11  the trail of how we got here
