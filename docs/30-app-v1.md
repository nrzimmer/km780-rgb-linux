## Key remapping: docs/28's two traps, closed  [C]

The assignment table (bytes 120-263, one byte per slot) also goes through
`Apply`. `docs/28` left two documented traps, and both are closed in code:

**The table is per mode.** Remapping only in the active mode makes the key stop
working when the M1/M2/M3 keys switch the profile -- and they switch without
telling the host. `assign::propagate` carries **only the function bytes that
changed** to the other two modes. Only the ones that changed, never the active
mode's whole payload: that would drag its color, brightness and effect over to
the other profiles.

**The undo depends on having saved the original.** The app reads and writes the
**same** `~/.config/km780/assignments.json` as `km780assign.py`, with the same
`"mode/slot"` keys. A format of its own would make one tool's `restore` blind to
what the other stored. And the record uses `entry().or_insert()`, which is
Python's `setdefault` -- the bug that destroyed the undo when reapplying the
same remap does not come back by this route. There is a test for it. With no
record, the restore button **refuses** instead of guessing.

### What can be assigned  [C]

The combo box offers the ranges from `docs/09` §5.2, in two steps -- range,
then item --, because the keyboard one alone has 106 entries and in a single
list the popup ends up bigger than the window:

| Range | What it is |
|---|---|
| Off | `0x00`, no function |
| Free keys | `0x68`-`0x73` = F13..F24, `docs/28`'s free-key space |
| Keyboard | each key's HID usage -- makes the key **act as another** |
| Multimedia | `0xA5`-`0xAB`, from `docs/17` §3 |
| Mouse | `0xCE`-`0xD5`, from `docs/17` §2 |
| Windows | `0xC0`-`0xC8`, all nine, from `docs/17` §4 |

The Keyboard table is **generated** from the same slot list `docs/protocol.html`
publishes, filtered to real keyboard HID (`0x04`-`0x65` and the modifiers
`0xE0`-`0xE7`); the high codes of the special keys are not HID and stay out.

**Macro (`0xB4`) and Text (`0xB8`) are not in the combo box**, each for a
different reason. A macro needs content, which lives in `0x05` and in `0x0A`
page `0x09`, and the app manages neither -- assigning `B4` would point at a slot
nobody maintains. Text **never** works without G.SKILL's app running: the
content does not reach the keyboard, only the name. A hex field alongside
reaches both, and any other byte, for whoever knows what they are doing; the
range the code belongs to shows up beside it as you type.

### "Copy from": copying another key's function

Choosing a source key copies the function it has **now, in the buffer** -- not
its factory HID. If the source has already been remapped, it is the remapped
value that travels, which is what "copy this key" means.

### The GUI and the CLI go through the same function, on purpose

Validating `--function` on the wire would prove nothing about the window's
`Apply` if they were two bodies of code. So `assign::propagate` is the single
path: the GUI calls it on `Apply`, the CLI on `--function`. Measured, with Timer
(slot 44, byte 164):

    F13 -> F14   modes 1, 2 and 3    1 slot propagated
    F14 -> F13   modes 1, 2 and 3    1 slot propagated
    record after both rewrites:  {"1/44": 186, "2/44": 186, "3/44": 186}

`186` = `0xBA` is Timer's factory function code. It **survived two rewrites**,
which is exactly what `docs/28`'s bug destroyed.

## What v1 does, and what it does not

It does: reads the active mode, the mode payload and the three color channels;
draws the keyboard; edits a local buffer of color, brightness, effect and key
function; `Apply` writes, `Discard` re-reads, `Undo` goes back to the state
before the last `Apply`. Hovering a key shows its slot, color and function.

It does not, on purpose:

- **The effect gradients** (`0x0B`/`0x0E`/`0x0F`/`0x12`/`0x15`) are not
  editable -- only the byte that turns the effect on.
- **Macro and Text** (`0xB4` and `0xB8`) are not offered in the remapping: both
  need extra data in the `0x0A` database and a Text's content never appeared on
  the wire (`docs/17` §7). The field accepts raw hex for whoever knows what they
  are doing, with the range announced alongside.
- **The I/O runs on the UI thread.** A `Reload` freezes for ~250 ms, an `Apply`
  for ~160 ms. A conscious trade of robustness for less code in v1.

With that, **all four items of `docs/29`'s "what comes after" are done**:
brightness, effects, key remapping and saved profiles. What is left of that
document is only the Windows `[?]` block, which needs a Windows machine.

## The 22 slots the drawing does not show  [C]

The color command addresses **144 slots**. Of those, 132 have a name in the
factory table (`docs/10`), and **122** are physical keys on this keyboard --
the ones the drawing shows. That leaves 22, and it is worth knowing which:

**10 named, with no key on the US/ABNT layout:**

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
number** in place of a name:

    km780 --paint 85,F4 00ff00
      slot  85  #000000   (no physical key on the US/ABNT layout)
      slot  36  #00FB2F

The warning is a warning, not a refusal: painting a slot with no key is
legitimate, and several are lit. Whoever asked by number simply deserves to know
they will see nothing on a US/ABNT layout.

The GUI's `All` button selects **only the drawn keys**, on purpose: the button
says "All" and the user expects the keyboard they are looking at.

## Profiles in a file of ours  [C]

Item 4 of `docs/29`, and the "ours" is the point: the keyboard already has three
modes, they are its own, and there are three. A profile from here lives in
`~/.config/km780/profiles/<name>.json`, and the user can have as many as they
want.

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

The app is the product -- the declared idea is to replace G.SKILL's --, so the
screen, the messages, the command-line flags and the **crate's identifiers** are
in English. So are the comments, and so are `docs/` and `tools/`: the reasoning
and the references to `docs/NN` live in those comments, and keeping them in the
same language as the documentation that explains them is what keeps the two
together.

The `SLOTS` table is **canonical** and shared with `docs/10` and with the Python
tools, so renaming an entry there would break both ends. Of the 132 entries only
one is shortened for the screen, the `Brightness` key, which becomes `Bright` at
drawing time via `slots::display_name` -- a one-line exception, not a second
table.

## Guards the code respects

Of the five rules in `docs/29` §"Rules the app must not break": only `0x81`,
`0x82`, `0x89` and `0x09` are emitted; `0x16` never; the mode payload is only
written by read-change-write, and that is measured; the mode is **re-read before
every `Apply`** and the write is refused if it changed (M1/M2/M3 switch the mode
inside the keyboard); and the timing is `docs/09` §2's, 10/20/+100 ms.
The header warns when the mode has effect != 0, because then the static color
is hidden by the animation and the user would think the app had not worked.

## Still to be verified on Windows  [?]

The `.exe` builds, and that is all. Everything `docs/29` listed as unverified
is still unverified: which device path to open among the several top-level
collections, and whether Windows allows opening the collection with read
access. `proto::usage_page_hint` already prefers the collection with usage page
`0xFF01`, and `Keyboard::open_device` tests each candidate with the harmless
`0x81` read before accepting it -- but that is design, not measurement.

## Macros  [C]

Record, edit, save, read from the keyboard and write to the keyboard. What is
**not** done is the link to the key -- see the end of this section.

### Recording happens in the window, not by listening to the system

`tools/listen_x11.py` listens to the whole keyboard, from any window. That is
right for measuring the keyboard and wrong for recording a macro: it would catch
what the user types elsewhere, and it only exists on Linux with X11. Through
`egui`'s events the recording is the same on Windows, which is the declared
target.

The price is that the window has to be focused, and that keys `egui` itself
consumes -- Tab moves the focus, Space presses a button -- have to be taken away
from it while recording. `macro_controls` clears the event queue and releases
the text focus on every recording frame.

Three things only showed up when recording was tested on the real screen:

1. **The `C` of Ctrl+C disappeared.** `egui` turns Ctrl+C, Ctrl+X and Ctrl+V
   into `Event::Copy`/`Cut`/`Paste` **before** delivering them, and none of
   those is a key event. The macro came out with the Ctrl and without the
   letter.
2. **A release of `LWin` appeared that nobody pressed.** The `Modifiers::command`
   field is an alias of `ctrl` on Linux and Windows; only on macOS does it mean
   Super. Using it for Super generated a phantom on every Ctrl that came up. The
   honest consequence: **the Win key is not recordable** -- `egui` delivers it
   neither as an event nor as a field.
3. **Shift+Z recorded the Shift coming up before the Z went down**, which would
   type a lowercase `z` on playback. The modifiers were being synchronized by
   the state at the **end of the frame**; each event carries the modifiers **of
   its own moment**, and the synchronization has to follow those.

After those three, a Ctrl+C recorded here comes out in the same shape the
official app sent on the wire in `docs/21` §2: press Ctrl, press C, release C,
release Ctrl.

### The delay is stored in units of 10 ms

Which is the wire's unit. Storing it in milliseconds would force rounding on
recording and again on writing, and two roundings over the same number do not
commute: a macro recorded, saved and reloaded could drift on its own. The screen
converts to seconds only for display and editing.

### What the screen refuses

A macro with a key **never released** is not written, and the button is disabled
with the reason beside it. A macro that presses Ctrl and never releases it
leaves Ctrl stuck **in the system**, and the symptom -- everything stops
responding -- does not point at the macro. Of the limits, the 84 events is the
packet's: 264 - 8 - 2, divided by 3.

### `Apply` does not write macros, and that is deliberate

A macro goes by another command (`0x05`), to another address, and is **not**
read-change-write: the packet is the entire content. Putting the two in the same
pending counter would make `Apply` promise a write it does not do. That is why
the macro section has its own button, which states the full address
(`Write to M2`), and not a generic "Apply".

### What was measured on the keyboard because of this

It is in `docs/09` §7b in detail; in short: `0x85` reads a macro (it had never
been read in this project), a virgin bank answers `0xFF`, an immediate re-read
after a write returns the old content, and **all three banks are mandatory** --
with one or two the keyboard ACKs and does not store. The third packet closes
the write, just like the third channel of `0x09`.

And the macros from the Windows captures **were still in the keyboard**: mode 1
returned the 8.65 s pause one from `docs/21`, mode 2 the `A B C` from `docs/16`.
`--macro-import` brought them to a file before any write test, and the tests
were all on mode 3, which was virgin and was returned to `0xFF` at the end with
`--macro-erase`.

### `0xB4` was solved by pressing the key

The key gets `0xB4` (`docs/16` §3) and the link to the index had been `[I]`
since the capture. Solving it required **pressing the key** and seeing what
comes out -- the only part of this project that cannot be automated.

What appeared: `0xB4` runs the macro in bank 1 of the active mode, and **only on
a G key**. G1 and G2 played the same macro; `Timer` and `LCtrl` played nothing,
and `LCtrl` also **went mute** -- it stopped emitting Ctrl, which shows the
firmware accepted the remap and simply does not play. Detail in `docs/09` §7b.2.

A bonus finding along the way: **G1 and G2 on this keyboard still had `0xB4`**,
put there by the official app during the Windows capture. `--function G1=b4`
answered "DID NOT STICK / 0 slots propagated" because there was nothing to
change. So the decisive test ran **with no write of ours at all**, in the state
G.SKILL's own app had set up -- the cleanest control that question could have
had.

On screen, the `0xB4` warning names the key: `only G1-G6 run macros: LCtrl
would go silent instead`. Naming matters because the failure is silent -- the
key simply stops working, with no error and no clue.

### And a macro of OURS plays too

The test above proved `0xB4` plays -- but it played a macro the official app had
written. This app's whole path was only validated with a macro written by us:
`F13`, 2.5 s, `F14`, 2.5 s, `F15`, with deliberately inert keys so nothing gets
typed anywhere. It came out in the listener at **2.55 s and 2.56 s** between
events -- the programmed 2.50 s plus the 0.06 s release.

With that the cycle closes end to end: record in the window, encode the `0x05`,
write to all three banks, name it in the `0x0A` database, assign `0xB4` to a G
key, play.

The name now goes **along with** the write, on both paths, because the official
app always sends both and because the name is the only thing the keyboard has to
tell one macro from another -- `0x05` stores no name. A macro with no name is
still writable: the name is the keyboard's requirement, not the user's, and the
app puts `Macro` when the field is empty.

## The `0x0A` database reader, and what it disproved

`--base-dump` reads the whole database with `0x8A`; `--base-read PAGE SUB` reads
an isolated subpage, in a process of its own, which is how an address's content
is separated from the effect of read order.

The first thing it showed was that **the name I had written was not there**.
Page `0x09` was zeroed, and the macro had played anyway. Two mistakes of mine at
once:

1. I was writing **five** subpages; the capture shows sixteen, and an incomplete
   sequence does not commit. The same shape as `0x05`'s three banks -- and I had
   already measured that shape two days earlier, in the same project, without
   carrying it over here.
2. I was writing the name into subpage `0x00`, which **discards**. The ones that
   keep are `0x02`-`0x0F`.

And the third thing undoes my original hypothesis: when `0xB4` would not play on
an ordinary key, I assumed the name was the missing piece and started writing
it. The missing piece was something else -- the key has to be a G. The name
**never** was part of what the keyboard stores: the official app writes it into
`0x00`, the discarded one, and that is why the page was empty after all the
Windows captures. The name lives on the PC, not in the keyboard. Detail in
`docs/09` §7c.1.

Writing the name stayed in the app anyway, now at `0x02 + index` and by
read-change-write across all sixteen, because it is the only place that persists
and because it gives the user a way to see through `--base-dump` what is in the
keyboard. That the name's index is the same index as `0x05`'s is **our
convention**: the only pair the capture shows falls in the discarded subpage, so
it cannot be measured. Nothing else reads the field, so getting it wrong breaks
nothing -- inventing it without saying so would.

### A slip of mine worth recording

I wrote into the name database (`0x0A` page `0x09`) **without having read**
what was there first. `0x8A` reads that database and is safe; I should have
taken the portrait first. The macro's content I had saved with `--macro-import`,
so it came back exact; the name I restored as `Macro1(1)`, which is what the
capture in `docs/16` §2 shows being written there -- **restoration by inference,
not by reading**. The rule the project already applied to the mode bytes holds
for any indexed database: read before writing, always.

With the reader ready, the circle could be closed: `write_macro_name` now
**reads all sixteen subpages, changes one and gives all sixteen back**. The
other fifteen are other macros' names, and building them from scratch would
erase them -- which is the same class of mistake, one layer up.

## A bug the macro section found in the rest of the app

`slots::slot_of` accepts a name **or a number**, and used to try the number
first. So the ten keys of the number row could not find themselves: `"1"` landed
on slot 1, which is **G2**. On screen that showed up as a widget id collision --
`egui` painting "First/Second use of widget ID" over the drawing --, and in the
CLI it was worse and silent: `--paint 1 ff0000` lit up G2.

Now the name comes first and a raw number only counts for what is not a key
name; `#N` forces the number. Two tests hold it: one says the number row finds
itself, another that the slots with no key are still reachable by number. And a
third, in `layout.rs`, fails if two keys in the drawing land on the same slot --
which is the shape the bug had on screen.
