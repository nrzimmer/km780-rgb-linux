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
    src/profiles.rs profiles in a file of our own, item 4 of docs/29
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

The right layout was **inside the repository all along** -- see the section
"The layout comes from the vendor" below.

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

## The layout comes from the vendor, not from a guess  [C]

An earlier version of this document said that the position of the G keys, of
MR/M1-M3, of the WinLock/Brightness/Timer trio and of the multimedia ones was an "informed
guess, not measured". **It does not have to be**, and it no longer is: the source was in the
repository itself, in `external/gds/`.

`payload/app/KeyboardButtons.xml` is the file the **official app** uses to
draw its own keyboard. Each key has a `<POS>` with x, y, width and
height in pixels. There are **110 keys** -- the 104 the directory declares in
`<KEY_NUM>` plus the six G keys. `src/layout.rs` is generated from it.

The 12 that are missing -- MR, M1-M3, WinLock/Brightness/Timer and the five
multimedia ones -- **are not in the XML because the official app does not let you
reassign them**. Those came from `Keyboard Wireframe_01232015v3.pdf`, from the
same package: a photo of the keyboard, from which the positions were measured and
converted to the XML's coordinate system using the neighbouring keys
as a reference.

And the guess was wrong on two points, which justifies the work:

- **MR, M1, M2 and M3 sit in a strip at the top left**, not in a column
  next to the G keys. And `WinLock`, `Brightness` and `Timer` sit **next to
  them, also at the top left** -- not in the right corner, where the guess
  had put them.
- The five multimedia ones sit at the top right, in the order **Stop, Prev,
  Play, Next, Mute**, above the numeric keypad region.

### The check that closes it: 122

110 from the XML + 12 from the wireframe = **122 keys**. And 122 is exactly how many
`docs/25` counted by scanning the electrical matrix on the hardware, by a path that
has nothing to do with this one. Two sources that do not talk to each other arriving at the same
number. There is a test pinning the 122: if the layout and the matrix ever disagree,
ele cai.

### As a bonus, `NumSep` stopped being a doubt  [C]

Doc 25 calls the key `NumEnter`, the slot list has `NumSep`, and this
document said that "it is not obvious they are the same". The vendor's XML
settles it: the entry is called **`NUM_ENTER  VK_SEPARATOR`**. They are the same
key, and `NumSep` is now drawn as the numeric keypad's Enter, two rows
tall, where it really is.

## Brightness and effect: the round-trip, proven instead of assumed  [C]

Brightness (byte 11) and effect (byte 10) go in through the **same `Apply`** as the color.
The user should not have to know that color goes through the `0x09` and brightness through the
`0x02`; that is why the app's editable state carries the 264 bytes of the mode
payload along with the 144 color slots, and `Apply` writes whatever is dirty
on either side -- the payload first, so the color arrives last when the
user turns the effect off precisely to see the static color.

Rule 3 of `docs/29` requires unknown fields to travel intact through the
round-trip. That is now **measured**, not trusted. `km780 --brightness N` and
`km780 --effect N` do the read-change-write and print the diff of the whole
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

### What can be assigned  [C]

The combo offers the ranges from `docs/09` §5.2, in two steps -- range, then
item --, because the keyboard one alone has 106 entries and in a single list the popup
ends up bigger than the window:

| Range | What it is |
|---|---|
| Off | `0x00`, no function |
| Free keys | `0x68`-`0x73` = F13..F24, the free key space from `docs/28` |
| Keyboard | each key's HID usage -- makes the key **act as another one** |
| Multimedia | `0xA5`-`0xAB`, do `docs/17` §3 |
| Mouse | `0xCE`-`0xD5`, do `docs/17` §2 |
| Windows | `0xC0`-`0xC8`, os nove, do `docs/17` §4 |

The Keyboard table is **generated** from the same slot list that
`docs/protocol.html` publishes, filtered down to real keyboard HID
(`0x04`-`0x65` and the modifiers `0xE0`-`0xE7`); the high codes of the special
keys are not HID and are left out.

**Macro (`0xB4`) and Text (`0xB8`) are not in the combo**, each for a different
reason. The macro needs content, which lives in the `0x05` and in the `0x0A` page
`0x09`, and the app manages neither -- assigning `B4` would point at a slot
nobody maintains. Text **never** works without G.SKILL's app running:
the content does not reach the keyboard, only the name. A hex field beside it reaches
both, and any other byte, for whoever knows what they are doing; the range the
code belongs to appears next to it as you type.

### "Copy from": copying another key's function

Picking a source key copies the function it has **now, in the
buffer** -- not its factory HID. If the source was already remapped, it is the
remapped value that travels, which is what "copy this key" means.

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
- **The I/O runs on the UI thread.** A `Reload` freezes for ~250 ms, an `Apply`
  ~160 ms. A conscious trade of robustness for less code in v1.

With that, **all four items in the "what comes next" of `docs/29` are
done**: brightness, effects, key remapping and saved profiles. What is
left of that document is only the Windows `[?]` block, which needs a
Windows machine.

## The 22 slots the drawing does not show  [C]

The color command addresses **144 slots**. Of those, 132 have a name in the factory
table (`docs/10`), and **122** are physical keys on this keyboard -- the ones the
drawing shows. That leaves 22, and it is worth knowing which:

**10 named, with no key in the US/ABNT layout:**

    35 INTL8b   80 INTL88   85 INTL87   91 INTL89   129 INTL85
        ^ the five with HID in the international range 0x85-0x8b

    16 \|   32 VK25   68 VK61   71 VK28   88 VK22
        ^ the extra ISO key and four slots with a virtual-key but no key here

**12 with no name at all:** the payload's `132`-`143` tail. They came back
zeroed in every read; probably padding.

**They are not inert.** A profile's portrait caught `16`, `32`, `68`, `71` and
`88` at `FF0000` while the whole keyboard was green -- they are precisely the
five `km780lib.py` notes as "once on the no-key list", lit ever since the
slot-by-slot identification sessions of `docs/26`. "Dark" never meant "no key"
(`docs/23` §6), and this is the proof from the other side: there is a slot with
no key that is lit.

### Why they do not appear in the GUI

An earlier version listed them in a column below the keyboard. It went: it took
a third of the usable area for something that, in normal use, is of no use. The
documentation is here; access is through the CLI, which accepts **the slot
lugar do nome:

    km780 --paint 85,F4 00ff00
      slot  85  #000000   (no physical key on the US/ABNT layout)
      slot  36  #00FB2F

The warning is a warning, not a refusal: painting a slot with no key is
legitimate, and several are lit. Whoever asked by number simply deserves to know
layout US/ABNT.

The GUI's `All` button selects **only the drawn keys**, on purpose: the button
says "All" and the user expects the keyboard they are looking at.

## Profiles in a file of ours  [C]

Item 4 of `docs/29`, and the "our own" is the point: the keyboard already has three modes,
they are its own, and they are three. A profile from here lives in
`~/.config/km780/profiles/<name>.json`, and the user can have as many as they want.

Saving takes a portrait of the **buffer**, not of the keyboard -- what is on
screen is what goes into the file. Loading does the reverse and **writes
nothing**: it lands in the buffer and becomes pending, like every edit here.

A profile stores color, brightness and effect. **It does not store the key
assignment table**, and that is a decision, not an oversight: a remap goes to
all three modes and survives a reboot (`docs/28`). Loading a profile called
"night" and finding out it also changed the function of eight keys in every
profile on the keyboard would be exactly the surprise this app exists to avoid.
The `format` field leaves the door open for a version that asks for this
explicitly, and a file with a newer format is **refused** rather than read
halfway.

### Exercised through the interface itself

With `xdotool`, not only through the API:

    click on the F5 key         -> selection border, "1 selected"
    "Clear selected"            -> F5 black, "1 key(s) pending"
    name "night" + "Save"       -> night.json with 144 colors, brightness 4, effect 0
    "Load"                      -> F5 green again, "nothing pending"

The saved file also showed something about the keyboard, not the app: slots 16,
32, 68, 71 and 88 -- the five `km780lib.py` notes as "once on the no-key list"
-- are at `FF0000` while the rest of the keyboard is green. It is not the app:
they are a leftover from the slot-by-slot identification sessions of `docs/26`.
It reinforces the decision to keep those slots reachable in the "Other slots"
list instead of making them disappear.

## The whole repository is in English  [C]

The app is the product -- the stated idea is to replace G.SKILL's --, so the
screen, the messages, the command-line flags and the **crate
identifiers** are in English. The comments carry the reasoning and the references to
`docs/NN`, alongside `docs/` and `tools/`, and moving them away from that would only pull the
code away from the documentation that explains it.

The `SLOTS` table is **canonical** and shared with `docs/10` and with the Python
tools, so renaming an entry there would break both ends. Of the 132 entries only
one is shortened for the screen, the `Brightness` key, which becomes `Bright` at
drawing time via `slots::display_name` -- a one-line exception, not a second
table.

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
