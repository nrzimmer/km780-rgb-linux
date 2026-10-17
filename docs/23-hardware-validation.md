# First contact with the hardware

> 2026-10-16, keyboard plugged into Fedora, **read commands only**.
> The first time this repository's code touched the device.
>
> **[C]** confirmed · **[I]** inferred · **[?]** open

## 0. Summary

The tooling worked on the first run, without `sudo`. Five claims from the
specification were confirmed on hardware, one open item was closed, and
**one claim marked `[C]` fell** — the inference behind it is not valid.

## 1. Access: no root needed  [C]

The node shows up as `crw-rw----+ root root`, but the `+` is a `uaccess` ACL
granting `rw` to the graphical session's user:

    user:zimmer:rw-

So nothing in this project needs privilege, nor a `udev` rule. Worth recording
because the documentation of similar projects tends to tell you to create a
rule in `/etc/udev/rules.d` with no need.

## 2. What `lsusb` confirmed, without sending a byte  [C]

| Claim | Where | Checks out |
|---|---|---|
| `bcdDevice` = firmware version | §1.0 | `bcdDevice 0.77` = FW 77 |
| 3 interfaces, protocols 1 / 2 / 0 | §1 | exact |
| endpoints `0x81` / `0x82` / `0x83` | §1 | exact |
| `bInterval` is the polling rate | §1.0.1 | `bInterval 1`, and byte 8 of the mode read over the protocol says 1000 Hz |

That last line is a **real cross-check**: the USB descriptor and a field of the
mode payload, read by independent paths, agree.

## 3. Byte 4 of the ACK is the mode — now on hardware  [C]

`22-gaps-solved.md` §2.1 deduced this from capture statistics. Listening to EP
`0x83` while sending commands:

    sent 0x81  ->  00 00 81 00 02 00 00 00     byte 4 = 2
    sent 0x82  ->  00 00 82 00 00 00 00 00     byte 4 = 0
    sent 0x89  ->  00 00 89 00 00 00 00 00     byte 4 = 0

The active mode read through `0x81`'s control IN is **2**, the same as byte 4
of the ACK. Confirmed both ways.

## 4. Page `0x09` of `0x8A` can be read  [C]

It had been open since the Windows session: *"the app never reads it; safe to
test on Linux"*. **Tested: it works.** The header echoes correctly
(`07 8a 09 00`) and the payload comes back zeroed — this keyboard has no macro
or text stored, which is consistent.

As a bonus, the other `0x8A` pages decoded exactly as `§7c` describes:

    page 0x00 sub 0x00  ->  "Profile2"       profile name
    page 0x00 sub 0x01  ->  "Mode1"          mode name
    page 0x1b sub 0x00  ->  "Default Wave"   effect name

> **A note on the init.** `12-capture-01-open.md` records that the init reads
> `0x8A` and gets "all `0xFF`". Today the same pages return real data. The
> `0xFF` was a **state** — probably right after the firmware was reflashed —,
> not the command's normal answer.

## 5. The three effect slots per mode exist  [C]

Reading subpages `00`/`03`/`06` (name) and `02`/`05`/`08` (parameters) of pages
`0x1b`/`0x1c`/`0x1d`, all three slots answer in all three modes, and the format
in `§7.4` decodes byte for byte:

    modes 1 and 2, slots 0/1/2:  "Default Wave"  dur=3 dir=3  4 stops
        ff0000 int 100 pos   0
        ffffff int 100 pos  23
        00ff00 int  22 pos  47
        0000ff int 100 pos 100

    mode 3, slots 0/1/2:         "Default Wave"  dur=9 dir=3  3 stops
        ffffff int 100 pos   0
        c6010b int 100 pos  83
        0000ff int 100 pos 100

The four stops of mode 1 are **exactly** the factory example documented in
`§7.4`, including the green with intensity 22. And mode 3's `dur=9` matches the
observation from the Windows session ("Wave 9").

This **corroborates the `0x0E`/`0x0F` model** from `22-gaps-solved.md` §4: that
model requires three effect slots per mode to exist, and they do, addressable
and with content.

One new detail: **the three slots hold identical data** within a mode. They are
not three different effects. Why there are three remains `[?]`.

Another cross-check: mode 3's `c6010b` stop is the most frequent color in the
LED matrix read from mode 3 (18 slots). The stored gradient and the rasterized
matrix agree.

---

## 6. CORRECTION: "dark slot" does not mean "no key"  [C]

`09-protocol.md` §6.7 claims, marked **[C]**:

> besides those, **10 slots within the 132 also have no physical key** on the
> US/ABNT layout: 16, 32, 35, 68, 71, 80, 85, 88, 91, 129

The evidence was: a matrix read came back dark in those slots. **The inference
does not hold**, because the set of dark slots **changes from mode to mode**:

| Mode | Dark slots within the 132 |
|---|---|
| 1 | 35, 80, 85, 91, 129 |
| 2 | 35, 80, 85, 91, 129 |
| 3 | 6, 8, 24, 42, 43, 44, 105, 106, 107, 115, 116, 129 |

In mode 3 the dark ones are the **media keys and zones** (`Z807`…`Z812`), and
the international ones are lit — the opposite of modes 1 and 2. Only slot 129
is dark in all three.

The matrix stores **what was painted last**, not what exists. A dark slot only
says nobody wrote a color into it.

### What survives of the claim

- **The 12 slots 132–143 are dark in all three modes** and are not addressed by
  any table. Those stay `[C]`.
- The 5 dark ones in modes 1 and 2 — 35, 80, 85, 91, 129 — all have a **HID
  usage in the international range** `0x85`–`0x8b`, and four of them have
  `vk = 0`:

      slot  35  vk=0     hid=0x8b      slot  91  vk=0     hid=0x89
      slot  80  vk=0     hid=0x88      slot 129  vk=0     hid=0x85
      slot  85  vk=2067  hid=0x87

  For those, the argument **does not depend** on the matrix read: `vk = 0` with
  a JIS/ABNT-layout HID already says the key does not exist on the US layout.
  `[C]`

- The other 5 — 16, 32, 68, 71, 88 — **were lit** in modes 1 and 2, and have
  real virtual-key codes (226, 25, 61, 28, 22). For those there is no evidence
  of absence at all. `[?]`

In other words: the Windows session corrected "5 slots" to "10" on the basis of
this read, and the correction went the wrong way. **It was the original 5 that
had a foundation.**

### How to settle it for good

Write a color into a suspect slot and look at the keyboard. That is a write, so
it waits for authorization — `tools/km780exp.py` does it with a backup.

---

## 7. What was run

    tools/km780ctl.py info                     state of the 3 modes
    tools/km780ctl.py backup <file>            colors + payload of the 3 modes
    reads of 0x8A pages 0x00/0x09/0x1b/1c/1d
    listening to EP 0x83 during 0x81 / 0x82 / 0x89

No writes. The backup of the state found is in `backup-first-contact.json`
(outside the index, being machine-specific).

---

## 8. Byte 109 is the Windows key lock  [C]

Discovered by accident, and the accident is the method: while collecting the
matrix the bag-symbol key was pressed, and afterwards i3's `Win+E` stopped
working — **the letter went through**, the Win did not count as a modifier. The
signature of a *Windows key lock*.

Comparing the active mode against the backup taken before anything, **a single
byte of the 264 had changed**:

    mode 1: no difference
    mode 2: byte 109:  00 -> 01      <- the active mode
    mode 3: no difference

Pressing the same key again, the byte went back to `00`, Win worked again, and
the diff against the backup went to zero.

Two things this settles:

- **the bag-symbol key is the Windows key lock** — the third of the column-4
  trio of the matrix, along with brightness `(4,7)` and timer `(4,8)`
- **the lock is not done by zeroing the key in the assignment table**: bytes
  120–263 did not change. It is a flag of its own.

### And it corrects the three RGB triples  [C]

`09-protocol.md` §5 said:

> | 109–117 | 9 | **three RGB triples** stored in the profile (pure green from
> the factory) | [I] |

With 109 being the lock, the triples can only start at **110**. And read from
there they are:

    bytes 110-112: (255, 0, 0)
    bytes 113-115: (255, 0, 0)
    bytes 116-118: (255, 0, 0)

**Pure red, three times** — not green. The one-byte error was what shifted
`(255,0,0)` into `(0,255,0)`.

The independent confirmation: mode 1's LED matrix has **127 slots at exactly
`ff0000`**. The stored triple and the painted matrix agree.

### The method, which is worth more than the finding

The byte was located in a diff, not in a sweep: there was a **complete portrait
taken before touching anything**, and an observable change in behaviour.
Comparing the two gave the exact byte, first time, without writing anything.

It stands as the procedure for the fields still missing (108 vs 119, 72–107):
take the portrait, work the control **on the keyboard itself**, and compare. No
writing needed at all.

---

## 9. 108 vs 119: there are **three** toggles, not two  [C]

### What brightness at 0 proved

Pressing the brightness key until the keyboard went dark (four presses, 4→0),
the diff against the portrait showed **a single byte**:

    byte 11: 4 -> 0

Not 108, not 119, not the color matrix. That is: **"going dark via brightness"
is just byte 11**, and it confirms once more that the firmware applies the
intensity without the host pre-multiplying the color. The 108 and 119 toggles
are **app-side only** settings — the keyboard has no key for them.

### There is a third toggle, byte 107  [C]

Separating, across the 11 captures, the host's writes (`0x02`) from the
keyboard's answers (`0x82`), three bytes behave like toggles, with a clear
pattern and rare inversions:

| Byte | Pattern | Off-pattern | Captures where it flips |
|---|---|---|---|
| 107 | 1 | `0` in 2 writes | `08-closing` |
| 108 | 1 | `0` in 6 writes | `05-macros`, `08-closing` |
| 119 | 0 | `1` in 8 writes | `03-config`, `08-closing` |

Each inversion appears **in pairs**, 0.2 to 1 s apart — it is the two-writes-
per-UI-action pattern that byte 106 alternates with. So that is 1, 3 and 4
*actions*, not 2, 6 and 8.

`19-capture-08-closing.md` §4 spoke of "the two toggles". **There are three.**

### And the app has exactly three  [C]

In `Setting.dll`'s block of configuration keys (offset `0x24d6e0` onwards,
UTF-16), the three appear in sequence:

    LightBrightness
    LightDisable
    BacklightDisable
    LightVolumeDisable
    KeyboardMode

Three `Disable` keys for three toggle bytes. The counts match exactly.

### `119 = LightVolumeDisable`  [C]

By elimination within `03-config`, whose exercised controls are — by the
document's own title — **polling rate, repeat delay and volume display**.
Polling is byte 8, repeat delay goes in `0x0A`, and the **only toggle byte that
moves in that capture is 119**. Neither 107 nor 108 moves there.

### `107` and `108` are `LightDisable` and `BacklightDisable`  [I]

In some order. Both candidates are named and the set is closed, but **which is
which did not come out**. The key order in the INI suggests
`107 = LightDisable` and `108 = BacklightDisable`, since 107 comes before 108
just as `LightDisable` comes before `BacklightDisable` — but that is
configuration-file key order, not proof of packet layout.

It is settled by writing one of the two and looking at the keyboard, or by
decompiling the stretch that builds the payload.

## 10. The key block also describes LED Sleep  [C]

`09-protocol.md` §9 item 15 says LED Sleep "has not been captured yet". The
configuration block shows **how it is parameterized**:

    SleepMode        SleepMin        SleepEffectKind
    SleepEffectName  SleepEffectID

That is: a mode, a **time in minutes** and an **effect** (kind, name and ID) —
the same `Kind/Name/ID` triple the ordinary effects use. The neighbouring
feature, the Timer key, has the same shape:

    AlertEffect   AlertOnScreen   AlertMin
    AlertEffectKind   AlertEffectName   AlertEffectID

`AlertOnScreen` confirms, from the binary side, what `19` §6 already said: the
OSD is drawn by the PC.

Whether those parameters ever reach the keyboard or stay in the app, as happens
with a **Text**'s content, remains unanswered.  `[?]`

## 11. Note: there is no Windows key lock key  [C]

No key containing `Win` or `Lock` exists in the block. Consistent with byte 109
(§8): the lock is a **keyboard** function, toggled by the bag-symbol key, and
the app does not configure it.
