## Key remapping: docs/28's two traps, closed  [C]

> 2026-10-17. Written after building, running and validating on hardware.
> The code is in `app/`. Doc 29 was the plan; this one says what the plan
> got wrong.

## O que existe

`app/`, 1000 lines, four modules:

    src/proto.rs    HID Feature report 7 transport, mirroring km780lib.py
    src/slots.rs    the 132 slot names, GENERATED from the Python list
    src/layout.rs   the physical layout, written by hand, with a test
    src/app.rs      the single screen: clickable matrix, color, Apply/Discard

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

Write, with `km780 --paint F1,F2,F3 ff0000`:

    the re-read matches what was written, across the 144 slots of the 3 channels

The check is always by re-reading with `0x89`, never by eye: this
unit has LEDs with the R component stuck (`docs/27` §10).

## A build dependency that was missing

Doc 29 said the Linux build needs `libudev`. It also needs the
**libusb headers** -- only the runtime was installed:

    dnf install libusb1-devel systemd-devel

## What v1 does, and what it does not

It does: read the active mode, the mode payload and the three color channels; draw the
keyboard; edit a local buffer; `Apply` writes, `Discard` re-reads,
`Undo` goes back to the state before the last `Apply`.

It does not, on purpose:

- **Brightness and effect** are only displayed, not edited. They are the next step, and
  `proto::write_mode` is already there and tested from the read side.
- **`NumSep` has no position in the drawing.** `docs/25` calls the key
  `NumEnter`, the slot list has `NumSep`, and it is not obvious they are the same.
  Guessing would put color on the wrong key; it stays in the "Other slots" list.
- **The I/O runs on the UI thread.** A `Reload` freezes for ~250 ms, an `Apply`
  ~160 ms. A conscious trade of robustness for less code in v1.

## Guards the code respects

Of the five rules in `docs/29` §"Rules the app cannot violate": only
`0x81`, `0x82`, `0x89` and `0x09` are sent; the `0x16` never; the mode
mode would only be written by read-change-write; the mode is **re-read before
every `Apply`** and the write is refused if it changed (M1/M2/M3 switch the mode
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
