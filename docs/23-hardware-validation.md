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
