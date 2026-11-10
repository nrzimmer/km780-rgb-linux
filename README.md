# KM780 RGB on Linux

Reverse engineering of the lighting protocol of the **G.SKILL Ripjaws KM780 RGB**
keyboard (USB `28da:1101`), plus a working controller for Linux.

The keyboard is a standard HID device, so typing, G keys and media keys already
work on Linux out of the box. The only thing missing is **RGB control** — the
vendor software is Windows-only. That is what this project provides.

> The protocol specification is [`docs/09-protocol.md`](docs/09-protocol.md),
> and [`docs/00-references.md`](docs/00-references.md) lists every third-party
> source this work builds on.

## Why this exists

No working open-source solution covers this keyboard:

- **OpenRGB** supports G.SKILL only on the memory line (Trident Z / Royal, over
  SMBus). No G.SKILL keyboard appears in its device list.
- **[`legonigel/KM780-RGB-Linux-Driver`](https://github.com/legonigel/KM780-RGB-Linux-Driver)**
  is the only project aimed at this keyboard. It was abandoned in February 2017
  after 3 commits and never worked as an application — but it does describe the
  USB transport layer correctly, which is credited and expanded in
  [`docs/01-transport.md`](docs/01-transport.md).
- **`sidewinderd`** targets Microsoft keyboards.
- Running the vendor software in a VM with USB passthrough is reported not to
  detect the keyboard at all.

## Status

The colour path is **validated against real hardware** — writes and reads
confirmed, not inferred from captures alone.

| Area | State |
|---|---|
| USB transport (framing, endpoints, commands) | known |
| LED payload layout | known |
| Key → LED slot map (132 slots) | confirmed 4× |
| Colour protocol | **validated on hardware**, read/write symmetric |
| Brightness | solved (byte 11 of command `0x02`) |
| Animated effect catalogue | solved — all 8 exposed by the vendor UI |
| Per-key effect data (`0x0B` / `0x12` / `0x15`) | solved |
| Macro format and key assignment | solved |
| Event channel `0x83` (ACK, key, brightness) | mapped |
| Physical matrix map (column, row) | method known |
| Firmware and MCU | identified |
| Decompilation of the vendor app (`Setting.dll`) | in progress |

Evidence: 11 USB captures of the vendor software, each paired with a screenshot
of what its UI was showing at the time, plus 30 documents tracing how every
byte was established.

## What is here

    app/        GUI controller in Rust (egui) — Linux and Windows
    tools/      Python CLI and analysis tools; km780lib.py is the shared transport
    docs/       one document per line of work; docs/09 is the canonical spec
    captures/   the 11 USB captures: .pcapng (raw) and .txt (decoded payloads)
    screenshots/ vendor UI screenshots matching each capture
    firmware/   MCU identification and the bootloader recovery procedure

### The Rust application

A single-window GUI (`app/`): key matrix, colour, brightness, effect and key
function, with an Apply button. It builds for both targets with no warnings:

```sh
cd app
cargo build --release                                  # Linux
cargo build --release --target x86_64-pc-windows-gnu   # Windows
```

The `hidapi` backend is selected per target deliberately, not by preference:
on Linux the libusb backend gets access without root (systemd's `uaccess` tags
the USB node, not `hidraw`), while on Windows the native backend is required —
libusb there would mean swapping the driver for WinUSB, which would stop the
device being a keyboard.

### The Python tools

```sh
python3 -m venv venv && venv/bin/pip install pyusb

venv/bin/python tools/km780ctl.py info              # current state, read-only
venv/bin/python tools/km780ctl.py set F5=ff0000     # paint keys
venv/bin/python tools/km780ctl.py brightness 4      # brightness, 0-4
venv/bin/python tools/km780ctl.py backup mine.json  # save everything first
```

`-n` performs a dry run. `tools/test_km780.py` validates the logic against a
simulated keyboard — it does not replace testing on the device, but it does
guarantee the packet layout is right and that read-modify-write preserves
unknown fields.

## Warning: read commands are not inherently safe

**`0x8x` commands are NOT safe merely because they are `0x8x`.** This was the
project's founding assumption and it is wrong: sweeping bytes with bit 7 set is
what put the keyboard into bootloader mode on 2026-10-16. Some value between
`0x96` and `0xAF` is not a read, despite bit 7.

The "bit 7 means read" convention holds for the **confirmed** write/read pairs —
`0x01/0x81`, `0x02/0x82`, `0x09/0x89`, `0x0A/0x8A` — not for arbitrary bytes.
Across all 11 captures of the vendor software, only those four `0x8x` commands
ever answer.

Do not add a command to `tools/km780_probe.py` without having seen that command
answer in a real capture. Recovery from bootloader mode is documented in
`firmware/`, but it requires a Windows machine.

## Scope

This project documents the protocol by **observing** the device and the traffic
the vendor software produces. It redistributes no vendor software, firmware or
installer; [`docs/06-binaries.md`](docs/06-binaries.md) records where those were
obtained and how they were extracted, so the work can be reproduced from the
vendor's own downloads. Every third-party source is listed in
[`docs/00-references.md`](docs/00-references.md), by link, never by copy.

## License

MIT — see [`LICENSE.md`](LICENSE.md).
