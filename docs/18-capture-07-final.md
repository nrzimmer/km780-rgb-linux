# Capture `07-final` — slots per key, Disable vs Default, physical matrix

Source: `captures/07-final.pcapng` / `.txt` (1485 packets).
Actions: Disable and then Default on the A key; painting individual keys;
"Save to Device Memory"; and a typing test with isolated keys.
Confidence: **[C]**.

## 1. Disable vs Default  [C]

On the A key (slot 21, byte 141 of the mode payload):

| Action | Byte |
|---|---|
| Disable | `0x00` |
| Default | `0x04` — the HID usage of the A key itself |

So **Disable writes `0x00`**, the same code as "no function", and Default
restores the native HID usage. On a G key both gave `0x00` precisely because
its native function is already nothing. That closes the doubt in `17` §5.

## 2. Color per key: the slot map, in real data  [C]

Painting individual keys, each one wrote exactly the slot predicted by
`10-key-map.md`:

| Key | Slot | R G B |
|---|---|---|
| S | **30** | `FF 82 10` |
| D | **34** | `FF FB 2F` |
| A | **21** | `FF 00 01` |

That was the original goal of the `05-only-A-red` / `06-only-S-red` files in
the plan in `02-usb-capture.md`. **A fourth independent confirmation** of the
slot order, now with real writes from the official software, per key.

Channel R came out with 127 non-zero slots: the 132 keys minus the 5
international ones that do not exist on the US layout.

## 3. `f6 01 XX YY` = the matrix's physical coordinate  [C]

Isolated keys, three repetitions each, with a pause between groups:

| Key | `XX YY` |
|---|---|
| Esc | `01 00` |
| A | `02 03` |
| S | `03 03` |
| Enter | `0a 03` |
| Num1 | `0c 04` |

Each key gave **the same pair on all three repetitions**. And the geometry
closes:

- A and S are neighbours -> same row (`03`), consecutive columns (`02`, `03`)
- Enter is on the same row as A and S -> `YY=03`, column well to the right (`0a`)
- Esc is on the top row of everything -> `YY=00`
- Num1 is on the numpad -> column further right still (`0c`) and one row down

So **`XX` = column and `YY` = row** of the physical matrix, emitted by the
keyboard on EP `0x83` with no software running at all. That gives a **second
addressing system**, independent of the LED slot: pressing each key once maps
the whole physical layout, with no binary and no app.

Ranges seen so far: `XX` from 1 to 12, `YY` from 0 to 8.

## 4. Pages `0x1b`, `0x1c`, `0x1d` are one per mode  [C]

Identical structure in all three, which corrects the reading in `09-protocol.md`
§7c (which separated `1b` as "config" and `1c` as "effects"):

| Subpage | Content |
|---|---|
| `00`, `03`, `06` | effect name in ASCII (`Default Wave`) |
| `01`, `04`, `07` | zeroed |
| `02`, `05`, `08` | effect parameters (duration, direction, gradient) |
| `0a` | (id, value) setting pairs — e.g. repeat delay |

Three groups of (name, empty, parameters) per mode.  [I] as to why there are
three.

Page map of `0x0A`:

| Page | Content |
|---|---|
| `0x00` | names: `Profile2`, `Mode1`, `Mode2`, `Mode3` |
| `0x09` | macro and text database (names in ASCII) |
| `0x1b` | database of **mode 1** |
| `0x1c` | database of **mode 2** |
| `0x1d` | database of **mode 3** |

## 5. "Save to Device Memory"  [C]

It fired `0x08` for all three modes, with the usual payload
(`fc fc fc 7c 7c 7c 7c`), and rewrote pages `0x00`, `0x1b`, `0x1c` and `0x1d`
of `0x0A`. No new command.

## 6. A Text's content does not go to the keyboard  [C]

The text `oi` does not appear anywhere in this capture — **not even after
"Save to Device Memory"**. Neither in ASCII (`6f 69`) nor as a HID usage. Only
the entry's name (`Text1`, page `0x09`) and the code `0xB8` in the key's slot
go to the keyboard.

Conclusion: **a key with a Text function only works with G.SKILL's software
running.** A macro, it seems, is autonomous; Text is not.

## 7. "Enable Static Base Color" is not a separate path

The user's report: the box can only be ticked when an effect is active, and
ticking it disables the effect. It is the same pair of states as "Enable
Lighting Effect", with another button. There is no third lighting mode to
discover.
