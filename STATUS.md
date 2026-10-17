# Project state and next step

> Updated 2026-10-17. This file is the resume point: it says where things
> stand, what to do next, and the rules learned the hard way.
> `START-HERE.md` is the **historical** handoff from the Windows session; this
> one succeeds it.

## The goal, and how much of it is left

Control the KM780's RGB on Linux. **The protocol needed for that is
documented and validated on hardware.** What is left is exercising the tool.

| | |
|---|---|
| Protocol specification | `docs/09-protocol.md`, canonical |
| Visual view | `docs/protocol.html` — opens in a browser, or claude.ai/artifact/K9mu4VENac49xSE2cR3YXM |
| Color tool | `tools/km780ctl.py` — **writing never exercised end to end** |
| Hardware-free tests | `tools/test_km780.py` — 24, all passing |

## NEXT STEP: exercise `km780ctl.py` end to end

The library underneath **has already written color to the keyboard**
successfully — that is what `km780slots.py` did to identify the 18 special
keys, with a clean, checked restore. What never ran is `km780ctl.py` itself.

Proposed sequence, in this order, with the keyboard plugged in:

    tools/km780ctl.py info                    # so le
    tools/km780ctl.py backup state.json       # read only, saves everything
    tools/km780ctl.py -n all blue             # dry run, does not write
    tools/km780ctl.py set F5=ff0000 G1=blue   # the first real write
    tools/km780ctl.py colors                  # check what stuck
    tools/km780ctl.py brightness 2            # read-change-write on 0x02
    tools/km780ctl.py todas 101010
    tools/km780ctl.py restore state.json      # puts everything back

After each write, look at the keyboard. **The active mode has to have no
effect** (2 is static); on a mode with an effect the animation overwrites the
matrix and nothing the tool writes will show up.

If it all passes, the project's stated goal is met and what comes next is
convenience: a `--profile` with saved themes, restoring at boot, packaging.


## Hardware state

- FW 77, `28da:1101`, needs no root (the `uaccess` ACL on the USB node)
- Backup of the original state in `backup-first-contact.json` (outside the index)
- Checked at the end of the last session: payload and colors **identical** to the backup

## What is still missing in the protocol (nothing blocks the tooling)

| Gap | How it comes out |
|---|---|
| internal layout of `0x0E` / `0x0F` | decompile `FUN_100d5060` (6,911 B, 5 stretches) — the function is already located |
| the role of `0x07` (`07 07 00 15`) | the only command on the wire with no build site located |
| the macro's Playback Option | decompilation |
| LED Sleep: do the parameters reach the keyboard? | `SleepMode`/`SleepMin`/`SleepEffect*` exist in the app; whether they go on the wire is unknown |
| which of the inert ones is `LightDisable` | decompilation only, and it gives the **name**, not the behaviour |
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
    12-21  one analysis per capture
    01-08, 11  the trail of how we got here
