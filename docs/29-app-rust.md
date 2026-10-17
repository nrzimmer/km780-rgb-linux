# The Rust app — decisions taken and starting point

> Written on 2026-10-17 as a **handoff to a fresh context**. Everything the
> next piece of work needs to know is here or pointed at from here. No code
> written yet.

## What the user asked for

- **A GUI, not a CLI.** The tools in `tools/` are for reverse engineering and
  stay that way; the app is another thing.
- **Rust**, not Python.
- **Buildable for Windows**, with the declared intention of replacing
  G.SKILL's official app.
- **v1 = read + static color.**
- **Batch editing with an explicit `Apply`.** The literal requirement: *"I want
  to be able to change several keys, and I need to apply for it to happen, not
  have it send on its own at every key or change"* — that is, the opposite of
  the official app, which writes on every touch.

## Decisions already taken

| | | |
|---|---|---|
| GUI | **egui / eframe 0.37.0** | chosen by the user |
| HID | **hidapi 2.6.7** | the only right choice, see below |
| Linux backend | **`linux-static-libusb`** | the feature is confirmed to exist |
| Windows backend | `windows-native` | the crate's default |
| Windows target | `x86_64-pc-windows-gnu` | **already installed**, mingw present |

### Why `hidapi`, and not `rusb` directly

The protocol is **Feature report ID 7** (`docs/01`, `docs/09` §1). `hidapi`
exposes exactly `hid_send_feature_report` / `hid_get_feature_report`, which are
the same `SetFeature`/`GetFeature` (ordinals 7 and 8) the official app calls in
`FUN_100d8fc0`. One body of code serves Linux and Windows, and on Windows it
needs no driver at all — it uses the system's HID stack.

### Why the libusb backend on Linux, and not hidraw  [C]

**Measured on this machine, 2026-10-17:**

    /dev/bus/usb/003/002   crw-rw----+ root root   ACL: user:zimmer:rw-
    /dev/hidraw0..9        crw-------  root root   no ACL

systemd's `uaccess` gives the session's user access to the **USB node**, but
**not** to the `hidraw` nodes. So:

- the **libusb** backend → works **without root and without configuring
  anything**, same as today's Python tools;
- the **hidraw** backend → would need a udev rule and one `sudo`.

libusb was chosen to preserve the "no root needed" property, which is a real
advantage over the official app. The udev rule stays as a documented
alternative if libusb ever becomes a nuisance.

A known pitfall: the libusb backend detaches the kernel driver from the
interface it uses. Today `km780lib.py` does that and **gives the driver back**
in `close()` (`attach_kernel_driver`). The app has to do the same in `Drop`, or
the keyboard may stop typing — which is what happened once in this session.

## Architecture of v1

The `Apply` requirement defines the design:

    keyboard state (read)         edit buffer (local)
    ---------------------------   -------------------------
    colors[3][144]  from 0x89 -->  editable copy
    mode payload from 0x82    -->  editable copy
                                   |
                              [Apply]    writes only what differs
                              [Discard]  throws the copy away and re-reads

- **Nothing is written until `Apply`.** Clicking a key and choosing a color
  only touches the buffer.
- The UI shows **how many keys are dirty**, and marks the dirty ones in the
  matrix.
- `Apply` compares the buffer against what was read and writes. Since color
  goes in 3 packets (one per channel, `docs/09` §6), any color change means
  resending all three — there is no partial channel write.
- `Discard` re-reads from the keyboard, which is the source of truth.
- An `Undo` after `Apply` comes for free: keeping the state read before writing
  is the same thing `km780ctl.py`'s `backup` already does.

### v1's screens

Just one: a header with device/FW/active mode, the keyboard matrix drawn, a
color picker, and the `Apply` / `Discard` bar with the pending counter.
Brightness and effects are for later (but brightness is one byte and it is
tempting — see "what comes after").

## What to port from `tools/km780lib.py`

It is the reference; the Rust should mirror it and not reinvent it.

| What | Where |
|---|---|
| Envelope constants | `PKT=264`, `HDR=8`, `REPORT_ID=7`, `NSLOTS=144` |
| Commands | `MODE_GET/SET 0x81/0x01`, `INFO_GET/SET 0x82/0x02`, `LED_GET/SET 0x89/0x09` |
| Mode offsets | `OFF_STATUS=3`, `OFF_POLLING=8`, `OFF_EFFECT=10`, `OFF_BRIGHTNESS=11`, `OFF_DIRECTION=12`, `OFF_KEYS=120` |
| Channels | `CHANNELS=(1,2,3)` = R, G, B, **in `buf[3]`** |
| Names of the 132 slots | the `SLOTS` list |
| Physical matrix 15×9 | `physical-matrix.json`, for drawing the keyboard |

**Mind `buf[3]` vs `buf[4]`:** `0x09`'s channel goes in `buf[3]`; the effect
helpers (`0x0B`/`0x12`/`0x15`) use `buf[4]`. Confusing the two has already
caused a bug here, and that is why `km780lib.py`'s packet builder takes explicit
`b3`/`b4`/`b5`, with no parameter called "channel".

## Rules the app must not break

1. **Never probe an unobserved `0x8x` command.** A sweep between `0x96` and
   `0xAF` put this keyboard into bootloader mode. The confirmed read space ends
   at `0x95`. The app only needs `0x81`, `0x82`, `0x89`.
2. **Do not send `0x16`.** The app does not need it (it is what turns on the
   `f6` key reporting), and the session it opens **cannot be closed** except by
   a USB reset.
3. **Read-change-write on the mode payload.** Never build a payload from
   scratch: the unknown fields travel intact through the round-trip, and that
   is what makes it possible to touch the known without understanding the rest.
   It holds for brightness and effect.
4. **Re-read the active mode with `0x81`** instead of trusting the last value:
   the M1/M2/M3 keys switch the mode **inside the keyboard**, without telling
   the host.
5. **Give the kernel driver back on exit** (the `attach_kernel_driver`
   equivalent in `Drop`).
6. **Respect the timing** in `docs/09` §2: 10 ms on simple transactions, 20 ms
   between `0x09`'s channels and +100 ms after the third.

## A caveat when testing color on this keyboard

This unit has **some LEDs with the R component stuck** (only the brightness
varies). It shows in static color — exactly v1's case — and not in animation.
Healthy ones checked: `F1 F2 F3`, `1 2 3 4`. Suspect: `P`, `[`. Check color by
**re-reading the payload** with `0x89`, never by eye alone. See `docs/27` §10.

## Build

Verified on this machine:

    cargo build --release                               # Linux
    cargo build --release --target x86_64-pc-windows-gnu # Windows

The target and mingw are already installed. For `hidapi` with
`linux-static-libusb`, the Linux build needs `libudev` (present, 259).

## What comes after v1

In order of return, all already documented and none needing more reverse
engineering:

1. **Brightness** — one byte (11) via read-change-write. Almost free.
2. **Effects** — byte 10, with the catalog in `docs/27` §7. It includes `6`,
   which the official app does not offer (and which is useless, but it is ours).
3. **Key remapping** — bytes 120–263, `docs/28`. It is the feature the official
   app has and handles badly, and ours has already been validated on the wire.
4. **Saved profiles** — in a file of ours, not on the keyboard.

## Reading map, by subject

    01   USB transport, what was known before
    09   canonical specification -- the app's reference
    10   key -> LED slot map (132 slots)
    25   physical matrix 15x9, for drawing the keyboard
    27   0x0E/0x0F, effect catalog, effect 6, the LED caveat
    28   remapping a key in the keyboard's memory
    protocol.html   the visual view, useful for checking packet layout
