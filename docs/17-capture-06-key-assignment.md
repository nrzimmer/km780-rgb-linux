# Capture `06-key-assignment` — the space of key function codes

Source: `captures/06-key-assignment.pcapng` / `.txt` (5599 packets).
Action: walk the G2 key (and G3, G4 in the first three) through every type in
the KEY ASSIGNMENT dialog, in the order the UI lists them.
UI lists in `screenshots/Screenshot 2026-10-16 1854*.png`.
Confidence: **[C]** where the capture and the UI order match; **[I]** where the
code was deduced from its position in the list without being exercised.

## 1. Method

Each assignment changes **a single byte** of the mode payload (`0x02`): byte
`120 + slot`. G2 = slot 1 = byte 121. Since the UI was walked in order and each
step produced a new value, the pairing of list item <-> code is direct.

The sequence observed in byte 121, in time order:

    ce  d3 d4 d5 d2 d1   ab aa a9 a7 a8 a6 a5   c7   b8   00

(plus `cf` on G3, `d0` on G4, in the first two mouse actions)

## 2. Mouse Function  [C]

The UI list, in this order: Left Click, Right Click, Scroll Click, Backward,
Forward, Double Click, Scroll Up, Scroll Down.

| Code | Function |
|---|---|
| `0xCE` | Left Click |
| `0xCF` | Right Click |
| `0xD0` | Scroll Click |
| `0xD1` | Scroll Down |
| `0xD2` | Scroll Up |
| `0xD3` | Backward |
| `0xD4` | Forward |
| `0xD5` | Double Click |

The order of application (`ce cf d0`, then `d3 d4 d5 d2 d1`) reproduces exactly
the list's order when read against this table — that is, the codes are
internally ordered by category (clicks, scroll, navigation, double) and not by
display order. That is the confirmation: it is not a guess, it is the only
attribution consistent with the sequence.

## 3. Multimedia  [C]

The UI list: Volume Up, Volume Down, Mute Volume, Play \ Pause, Next Track,
Previous Track, Stop.

| Code | Function |
|---|---|
| `0xA5` | Stop |
| `0xA6` | Previous Track |
| `0xA7` | Play / Pause |
| `0xA8` | Next Track |
| `0xA9` | Mute Volume |
| `0xAA` | Volume Down |
| `0xAB` | Volume Up |

**Independent check:** `10-key-map.md` shows that the slots of the keyboard's
physical media keys already carried `a5`, `a6`, `a7` (slots 105-107) and `a9`,
`a8` (slots 115, 116). That is, from the factory the KM780's media keys get
exactly these codes — Stop, Previous, Play, Next, Mute. It matches. `aa`/`ab`
do not appear in the factory table because volume on the KM780 is a wheel, not
a key.

## 4. Windows Shortcuts  [C]

> **Corrected in `19-capture-08-closing.md` §2.** The first version of this doc
> predicted a contiguous `C2`-`C8` range from the single measured item
> (Copy = `0xC7`). Capture `08` walked all seven and showed that the **app does
> not offer** `C3` and `C4`.
>
> **Corrected again on 2026-10-17**: the hole was the UI's, not the firmware's.
> See §4.1.

| Code | Function |
|---|---|
| `0xC0` | Launch Task Manager |
| `0xC1` | Open Start Menu |
| `0xC2` | Show Desktop |
| `0xC3` | **Task View** (`Super`+`Tab`) — outside the official UI |
| `0xC4` | **Switch window** (`Alt`+`Tab`) — outside the official UI |
| `0xC5` | Close Active Window |
| `0xC6` | Cut |
| `0xC7` | **Copy** |
| `0xC8` | Paste |

### 4.1 The hole at `C3`/`C4` was the UI's, not the firmware's  [C]

> Measured on 2026-10-17, assigning each code to byte 164 (the Timer key) with
> the app in `app/` and reading what X11 receives.

    0xC3  ->  Super_L + Tab   both in the same millisecond
    0xC4  ->  Alt_L   + Tab   likewise

On Windows they are **Task View** and **switch window**. The earlier diagnosis
-- "the range is not contiguous" -- was right about the capture and wrong about
the device: capture `08` walked the **seven items the dialog offers**, and the
dialog simply does not list these two. The range `0xC0`-`0xC8` is **contiguous
and complete**, nine codes.

It is the second known case of a feature the firmware has and the official app
does not expose -- the first is effect `0x06` (`docs/27` §10). It is worth a
general warning: **absence in a capture proves absence in the UI, not absence
in the device.**

Since both are combinations with `Tab`, on Linux the effect depends on the
window manager; in i3 with no binding for `Super`+`Tab`, only the `Tab` reaches
the application.

## 5. Macro, Text, Default, Disable

| Code | Type | Confidence |
|---|---|---|
| `0xB4` | Macro | [C] — `05-macros` |
| `0xB8` | Text | [C] |
| `0x00` | Default / Disable on a G key | [C] |

On a G key, Default and Disable both write `0x00`, because a G key's native
function is already "nothing". **To tell the two apart you would have to
disable an ordinary key** (e.g. the A key, slot 21): Default should restore
`0x04` and Disable should write some code of its own.  [?]

## 6. Map of the code space

| Range | Meaning |
|---|---|
| `0x00` | no function |
| `0x04`–`0xE7` (low) | HID usage — "Keyboard" type (e.g. `0xE0` = LeftCtrl) |
| `0xA5`–`0xAB` | Multimedia |
| `0xB4` | Macro |
| `0xB8` | Text |
| `0xC0`–`0xC8` | Windows Shortcuts (nine, contiguous — see §4.1) |
| `0xCE`–`0xD5` | Mouse Function |

The high ranges do not collide with the HID usages the keyboard uses
(`0x04`–`0x65` and `0xE0`–`0xE7`), so **a single byte is enough** to address
any function.

## 7. Text is not autonomous  [I]

Assigning the Text `oi` to G2 produced:

- `0x0A` page `0x09` sub `01` = `Text1` (the entry's **name**, in ASCII)
- `0x05` with `buf[3]=0x01` and the payload `01 00 00 ...`
- byte 121 = `0xB8`

The **content** `oi` never appeared on the wire — neither as ASCII (`6f 69`)
nor as HID usages (`0x12`, `0x0c`). Either the text stays on the PC and the app
replays it, or it only goes to the keyboard via the "Save to Device Memory"
button, which was not pressed.

Note too that page `0x09` of `0x0A` stores macros **and** texts in the same
indexed database: sub `00` = `Macro1(1)`, sub `01` = `Text1`. And `buf[3]` of
`0x05` is the index into that database.
