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
| Visual view | `docs/protocol.html` — opens in a browser, or claude.ai/artifact/K9mu4VENac49xSE2cR3YXM (v11, up to date with `docs/27`) |
| Color tooling | `tools/km780ctl.py` — **exercised end to end on hardware** |
| Hardware-free tests | `tools/test_km780.py` — 24, all passing |

## IN PROGRESS: Rust app with a GUI

The user asked for a real app — GUI, Rust, buildable for Windows, to replace
G.SKILL's. **The decisions are made and documented in
`docs/29-app-rust.md`; not a line of code written yet.**

Summary: egui/eframe 0.37, hidapi 2.6.7 with the `linux-static-libusb` backend
(not hidraw, because `uaccess` gives an ACL to the USB node but not to
`hidraw` — measured). v1 = read + static color, with **batch editing and an
explicit `Apply`**: nothing goes to the keyboard on every click. On Windows the
backend is `windows-native`, not libusb — libusb there would mean swapping the
driver for WinUSB and would stop the keyboard being a keyboard.

Start by reading `docs/29`.

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

- **Usability** — `--profile` with saved themes, restoring at boot (a systemd
  user unit), packaging. Needs no more reverse engineering.
  Already done here: **remapping a key in the keyboard's memory** (`docs/28`),
  which solves "give a key a function with no service". Timer is on F13 in all
  modes.
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
