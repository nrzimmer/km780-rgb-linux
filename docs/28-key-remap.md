# Remapping a key in the keyboard's memory — with no service

Validated on hardware on 2026-10-17. It answers the question "can a key that
Linux never receives be made useful, without a process running?".
**It can.**

## The problem

The three keys in column 4 — WinLock, Brightness and Timer — are resolved
**inside** the keyboard and **never reach Linux**. Confirmed by reading
`/dev/input/event*` on all five of the keyboard's nodes: pressing Timer
produces neither `EV_KEY` nor `MSC_SCAN`.

Their internal codes (`b9`, `bb`, `ba` in `docs/10`) are **not HID usages** —
they are the keyboard's own function codes. That is why no layer of the OS
sees them: there is nothing on the wire to see.

## The way out: the assignment table is rewritable

Bytes 120–263 of the mode payload are the key assignment table, one byte per
LED slot (`docs/09` §5.1). Timer is slot 44, hence **byte 164**. From the
factory it holds `0xba`, its function code.

Swapping that byte for a HID usage makes **the keyboard emit that key**. And
the table lives in the keyboard's memory, so the remap survives a reboot,
survives replugging, and holds on another computer — **with no process running
at all**.

    tools/km780assign.py set Timer=f13      # in all 3 modes
    tools/km780assign.py show Timer
    tools/km780assign.py restore Timer

## Confirmed on the wire  [C]

With byte 164 = `0x68`, pressing Timer produces, on the main node:

    EV_KEY   code 183   (= KEY_F13)
    EV_MSC   MSC_SCAN   scancode 0x70068

`MSC_SCAN` closes it from the outside: `0x07` is the Keyboard/Keypad page and
`0x68` the usage for F13 — exactly the byte written. And the event arrives with
`repeat`, that is, the kernel treats it as an ordinary key, with auto-repeat.

**The firmware honours the assignment table on a special key.** That was not
obvious: it was possible that it ignored the table for the column-4 trio and
kept handling Timer internally.

In i3, from then on:

    bindsym F13 exec --no-startup-id <command>

## Why F13..F24

`0x68`–`0x73` are F13 to F24. They exist in HID, Linux maps them to
`KEY_F13`..`KEY_F24`, and no software uses them out of the box — the natural
space for a "free key". The other ranges of the byte are in `docs/09` §5.2:
macro (`b4`), text (`b8`), Windows shortcut (`c0`–`c8`), mouse (`ce`–`d5`),
multimedia (`a5`–`ab`).

## Two traps

**The table is per mode.** Remapping only in the active mode makes the key stop
working again when the M1/M2/M3 keys switch the mode — and they switch without
telling the host. That is why `km780assign.py` writes to all three by default.

**The undo depends on having saved the original.**
`~/.config/km780/assignments.json` keeps the previous value, and `restore` goes
back to exactly that. With no record, the tool **refuses** instead of guessing:
guessing a function code is worse than doing nothing, and the factory table is
in `docs/10`.

> A bug found and fixed in that same session: reapplying the same remap
> overwrote the saved original with the new value, destroying the undo. The
> record now uses `setdefault` — the first value seen is the one that stays.

## What this opens up

It works for **any** key, not just Timer. Any of the 132 in the map can receive
any function from the byte's space, stored in the keyboard. It is hardware
remapping, which does not compete with `keyd` or `input-remapper` — it works
before them, and without them.

And the HID descriptor shows there is more space than keys: interface 1
declares a **Consumer Control** (report `0x03`) with usage `0x000`–`0x2ff` and
a **System Control** (report `0x02`, Generic Desktop `0x81`–`0x83`). What the
function byte can address inside that has not been swept — and, by this
project's rules, you do not sweep blind.  [?]
