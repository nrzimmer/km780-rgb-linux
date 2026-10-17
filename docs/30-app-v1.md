## Key remapping: docs/28's two traps, closed  [C]

> 2026-10-17. Written after building, running and validating on hardware.
> The code is in `app/`. Doc 29 was the plan; this one says what the plan
> got wrong.

## O que existe

`app/`, five modules:

    src/proto.rs    HID Feature report 7 transport, mirroring km780lib.py
    src/slots.rs    the 132 slot names, GENERATED from the Python list
    src/layout.rs   the physical layout, written by hand, with a test
    src/effects.rs  the byte 10 catalog, from the canonical table in docs/09 §7.1
    src/assign.rs   the function table (bytes 120-263) and the log from docs/28
    src/app.rs      the single screen: matrix, color, brightness, effect, function, Apply

It builds on both targets, with no warnings:

    cargo build --release                                  # 27 MB
    cargo build --release --target x86_64-pc-windows-gnu   # 28 MB

## Three corrections to doc 29  [C]

### 1. `physical-matrix.json` is no good for drawing the keyboard

Doc 29 says to use the file "to draw the keyboard". It is the **electrical
scan matrix**, and `docs/25` itself warns: "the matrix wiring does not
follow physical adjacency". M3 comes out at `(2,6)`, right next to F3; the
WinLock/Brightness/Timer trio comes out stacked in column 4. Drawing from there would give a
scrambled keyboard.

The layout in `src/layout.rs` is written by hand, in key units, with a
test that validates each name against `slots::SLOTS`. **The position of the G,
MR, M1-M3, WinLock/Brightness/Timer and multimedia keys is an informed guess, not
measured** -- if one shows up out of place, that is where it gets fixed, by touching
numeros.

### The GUI and the CLI go through the same function, on purpose

Measured on this machine. While the process keeps interface 1 claimed,
the kernel loses it:

    3-5:1.1 -> usbfs      (app holding it)
    3-5:1.1 -> usbhid     (app closed)

And interface 1 declares Mouse, System Control, Consumer Control **and the
0x08/0x09 keyboard reports** (`docs/29`). Holding the device for the life
of the window would take the user's media keys away while the app was
open -- a relative of the incident that already happened here.

That is why `App::with_keyboard` **opens, does the operation, and closes**. It costs an
enumeration of a few milliseconds per click and gives the driver back between them.

### 3. Rule 5 of doc 29 already comes for free -- but not for the expected reason

The rule says to give the kernel driver back on `Drop`. `hidapi` 2.6.7
builds the libusb backend **without** `DETACH_KERNEL_DRIVER` (there is no crate
feature that turns it on), so the detach/attach blocks in `hid.c` are
left out of the binary entirely. The prediction was that `libusb_claim_interface` would hit
`EBUSY` and the app would not even open.

That is not what happens: **it opens and it works**, and in the end all three interfaces go back
to `usbhid`. The hid node of interface 1 is recreated on every cycle --
`.0073` -> `.0074` -> ... -- proof that the driver leaves and comes back. What does that
is Linux's usbfs, not hidapi. The app needs no code at all to
give the driver back; what it needs is to **close**, which is §2 above.

## Validado no fio  [C]

Read, with `km780 --probe`:

    open: G.SKILL RIPJAWS KM780 RGB GAMING KEYBOARD
    active mode: 2
    effect=0 brightness=4
    0x89 ok: 127 slots lit
      F1   slot  18  #00FB2F

Color write, with `km780 --paint F1,F2,F3 ff0000`:

    the re-read matches what was written, across the 144 slots of the 3 channels

Brightness and effect write: the one-byte diff from the section above.
Key function write: the remapping section.

The check is always by re-reading with `0x89`, never by eye: this
unit has LEDs with the R component stuck (`docs/27` §10).

## A build dependency that was missing

Doc 29 said the Linux build needs `libudev`. It also needs the
**libusb headers** -- only the runtime was installed:

    dnf install libusb1-devel systemd-devel

## Brightness and effect: the round-trip, proven instead of assumed  [C]

Brightness (byte 11) and effect (byte 10) go in through the **same `Apply`** as the color.
The user should not have to know that color goes through the `0x09` and brightness through the
`0x02`; that is why the app's editable state carries the 264 bytes of the mode
payload along with the 144 color slots, and `Apply` writes whatever is dirty
on either side -- the payload first, so the color arrives last when the
user turns the effect off precisely to see the static color.

Rule 3 of `docs/29` requires unknown fields to travel intact through the
round-trip. That is now **measured**, not trusted. `km780 --brightness N` and
`km780 --effect N` do the read-change-write and print the diff of the payload
payload:

    mode 2: brightness 4 -> 2    bytes that changed in the 264-byte payload: [11]
    mode 2: effect 0 -> 4    bytes that changed in the 264-byte payload: [10]
    mode 2: effect 4 -> 0    bytes that changed in the 264-byte payload: [10]

**One byte of the 264, all three times.** Byte 3 (status), which the write zeroes
on purpose, comes back on the re-read as it was -- not even it shows up in the diff.

The effect catalog in `src/effects.rs` is the one in `docs/09` §7.1, which is the
union of what the captures saw with what the decompilation confirmed. It includes the
`0x06` -- the key that lights up on press and never goes dark --, which the official UI
does not offer and the DLL has no branch for, but the firmware runs
(`docs/27` §10). The `0x05` is left out: no branch, no capture.

For the effects that take color from a stored gradient (Ripple, Sonic,
Changeling, Rain), the app only changes byte 10 -- the animation comes out with the
gradient already on the keyboard. The combo says so on each one, because
otherwise it looks as if the color picker stopped working.

## Key remapping: the two traps from docs/28, closed  [C]

The assignment table (bytes 120-263, one byte per slot) also goes in through
`Apply`. `docs/28` left two traps documented, and both are
closed in code:

**The table is per mode.** Remapping only in the active mode makes the key stop
working when the M1/M2/M3 keys switch the profile -- and they switch without
telling the host. `assign::propagate` carries **only the function bytes that changed**
to the other two modes. Only the ones that changed, never the active mode's whole
payload: that would carry its color, brightness and effect along to the other
profiles.

**Undo depends on having stored the original.** The app reads and writes the
**same** `~/.config/km780/assignments.json` as `km780assign.py`, with the
same `"mode/slot"` keys. A format of its own would make one tool's `restore` not
see what the other stored. And `store` uses
`entry().or_insert()`, which is Python's `setdefault` -- the bug that destroyed
undo when the same remapping was reapplied does not come back by this path. There is a
test for it. With no log, the restore button **refuses** instead of
guessing.

### The GUI and the CLI go through the same function, on purpose

Validating `--function` on the wire would prove nothing about the window's `Apply` if
they were two code paths. So `assign::propagate` is the single path: the GUI calls it
in `Apply`, the CLI in `--function`. Measured, with the Timer (slot 44, byte 164):

    F13 -> F14   modes 1, 2 and 3    1 slot propagated
    F14 -> F13   modes 1, 2 and 3    1 slot propagated
    log after the two rewrites:  {"1/44": 186, "2/44": 186, "3/44": 186}

The `186` = `0xBA` is the Timer's factory function code. It **survived
two rewrites**, which is exactly what the `docs/28` bug destroyed.

## What v1 does, and what it does not

It does: read the active mode, the mode payload and the three color channels; draw the
keyboard; edit a local buffer of color, brightness, effect and key function;
`Apply` writes, `Discard` re-reads, `Undo` goes back to the state before the
last `Apply`. Hovering a key shows slot, color and function.

It does not, on purpose:

- **The effect gradients** (`0x0B`/`0x0E`/`0x0F`/`0x12`/`0x15`) are not
  editable -- only the byte that turns the effect on.
- **Macro and Text** (`0xB4` and `0xB8`) are not offered in the remapping: both
  need extra data in the `0x0A` base and the content of a Text has never
  appeared on the wire (`docs/17` §7). The field accepts raw hex for whoever knows
  what they are doing, with the range announced beside it.
- **Profiles saved to a file** -- the last item of `docs/29`.
- **`NumSep` has no position in the drawing.** `docs/25` calls the key
  `NumEnter`, the slot list has `NumSep`, and it is not obvious they are the same.
  Guessing would put color on the wrong key; it stays in the "Other slots" list.
- **The I/O runs on the UI thread.** A `Reload` freezes for ~250 ms, an `Apply`
  ~160 ms. A conscious trade of robustness for less code in v1.

## Guards the code respects

Of the five rules in `docs/29` §"Rules the app cannot violate": only
`0x81`, `0x82`, `0x89` and `0x09` are sent; the `0x16` never; the mode
payload is only written by read-change-write, and that is measured; the mode is
**re-read before every `Apply`** and the write is refused if it changed (M1/M2/M3 switch the mode
inside the keyboard); and the timing is the one in `docs/09` §2, 10/20/+100 ms.

The header warns when the mode has effect != 0, because then the static
color is hidden by the animation and the user would think the app did not
work.

## Still to be verified on Windows  [?]

The `.exe` builds, and that is all. Everything `docs/29` listed as unverified
is still unverified: which device path to open among the several top-level
collections, and whether Windows lets the collection be opened with read access.
`proto::usage_page_hint` already prefers the usage page `0xFF01` collection, and
`Keyboard::open` tries each candidate with the harmless `0x81` read before
accepting it -- but that is design, not measurement.
