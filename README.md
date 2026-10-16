# KM780 RGB no Linux

Reverse engineering of the lighting protocol of the **G.SKILL Ripjaws
KM780 RGB** keyboard (USB `28da:1101`), aiming to control the colors on Linux.

## Why

No working open-source solution exists:

- **OpenRGB** — supports G.Skill only on the memory line (Trident Z / Royal,
  over SMBus). No G.Skill keyboard in its device list.
- **`legonigel/KM780-RGB-Linux-Driver`** — the only project specific to this
  keyboard. Abandoned: 3 commits, the last on 2017-02-25, it never worked as
  an application. **But it does document the USB transport layer correctly**,
  which was extracted into `docs/01-transport.md`.
- `sidewinderd` — it is for Microsoft keyboards, no use here.
- The vendor software in a VM with USB passthrough — it is reported that the
  software **does not detect the keyboard** in that setup.

The keyboard is standard HID, so typing, the G keys and the media keys
already work on Linux. For macros and remapping the solution is on the OS
side (`keyd`, `input-remapper`). **All that is missing is RGB control** — the
goal of this project.

## Status

| Area | Status |
|---|---|
| USB transport (framing, endpoints, commands) | **known** — see `docs/01-transport.md` |
| LED payload layout / key→offset map | **unknown** — that is the target |
| Read-only probing tool | ready — `tools/km780.py` |
| USB capture of the official software (Windows) | planned — `docs/02-usb-capture.md` |
| Vendor installers obtained and extracted | **done** — `docs/06-binaries.md` |
| Decompilation of the vendor app | in progress — `docs/03`, `docs/06` |
| Firmware + MCU identified | **done** — `docs/04-firmware.md` |
| Ghidra 12.1.4 + GhidraMCP installed | **done** — `docs/07-tooling.md` |
| Analysis of `Setting.dll` in Ghidra | in progress |

## Three lines of attack

1. **Decompile the vendor app** (`docs/03`) — the highest information density:
   it gives the whole key→index table and the complete command set, not only
   what was exercised. Runs on Linux, with no reboot and no risk to the hardware.
2. **Capture the USB on Windows** (`docs/02`) — ground truth, confirms item 1.
3. **Firmware** (`docs/04`) — last resort; it is the device side and it is
   a great deal more work.

## Caution

Only write to the keyboard after understanding the structure. There is a
report on G.SKILL's forum of a KM780 needing a firmware reset via safety-pin
+ a `.bin` file, which means it is possible to leave the keyboard in a bad
state. Read commands (`0x8x`) are safe.

## Layout

    docs/      documentation by area of work
    tools/     our own code (km780.py: transport + GET commands, read-only)
    captures/  .pcapng files from the USB captures (not versioned)
    external/  third-party material: installers, the abandoned repo
