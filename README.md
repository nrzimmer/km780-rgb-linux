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

| Area | State |
|---|---|
| USB transport (framing, endpoints, commands) | **known** — see `docs/01-transport.md` |
| LED payload layout | **known** — `docs/09-protocol.md` §6 |
| Key → LED slot map (132 slots) | **confirmed 4x** — `docs/10-key-map.md` |
| Physical matrix map (column, row) | **method known** — `docs/18` §3 |
| Byte-by-byte protocol specification | Phase 1 closed — `docs/09-protocol.md` |
| **Color protocol validated on hardware** | **done** — writes and reads confirmed |
| **Intensity (brightness)** | **solved** — byte 11 of `0x02`, `docs/13` |
| Read-only probing tool | ready — `tools/km780.py` |
| USB capture of the vendor software (Windows) | **done** — `docs/12` to `docs/21`, 11 captures |
| Read confirmed symmetric to write | **done** — `docs/20` §1 |
| Animated effect catalogue | **solved** — the 8 in the UI, `docs/15`, `docs/16` |
| Macro format and key assignment | **solved** — `docs/16`, `docs/17` |
| Vendor installers obtained and extracted | **done** — `docs/06-binaries.md` |
| Decompilation of the vendor app | in progress — `docs/03`, `docs/06` |
| Firmware + MCU identified | **done** — `docs/04-firmware.md` |
| RE tooling (Ghidra 12.1.4 + REA) | **done** — `docs/07-tooling.md` |
| Analysis of `Setting.dll` in Ghidra | in progress |

## Order of work

Document the protocol, field by field, **before** writing the tool.
The canonical specification is `docs/09-protocol.md`.

1. **Phase 1** — fully document the **color and intensity** path:
   the `0x09`/`0x89` command (LED Matrix Database), its paging, the map
   key→offset map and any commit/apply.
2. **Phase 2** — work out which messages are left over and what each one does.

The gaps of each phase are in section 9 of the specification.

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

    docs/      documentation, one per line of work
    tools/     our code (km780.py: transport + GET commands, read-only;
               extract.sh/view.sh: decode the USBPcap captures)
    captures/  the 11 USB captures of the vendor software: .pcapng (raw) and
               (decoded payloads). Versioned — they are the evidence, and
               they cannot be recaptured without the Windows machine.
    screenshots/ vendor UI screens referenced by the capture docs
    sessions/  transcript of the Windows session (not versioned, 22 MB)
    external/  third-party material: installers, the abandoned repo

`START-HERE.txt` is the state summary brought over from the Windows machine:
what Phase 1 closed, what is left in Phase 2, and the warning not to sweep
unknown commands on the keyboard.
