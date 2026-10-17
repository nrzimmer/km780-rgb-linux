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
| **Per-key effect data (`0x0B`/`0x12`/`0x15`)** | **solved** — `docs/22` §1 |
| Event channel `0x83` (ACK, key, brightness) | **mapped** — `docs/22` §2 |
| Visual view of the protocol | `docs/protocol.html` |

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

**`0x8x` read commands are NOT safe merely for being `0x8x`.** This was the
project's premise and it is wrong: sweeping bytes with bit 7 set is what put
the keyboard into bootloader mode on 2026-10-16. Some value between `0x96`
and `0xAF` is not a read, despite bit 7.

The "bit 7 means read" convention holds for the **confirmed** write/read
pairs — `0x01/0x81`, `0x02/0x82`, `0x09/0x89`, `0x0A/0x8A` — not for
arbitrary bytes. Across the 11 captures of the vendor software only those
four `0x8x` commands ever answer.

Do not add a command to `tools/km780_probe.py` without having seen that
command answer in a real capture. Recovery exists and is documented in
`firmware/recovery/`, but it depends on a Windows machine.

## Layout

    docs/      documentation, one per line of work
    tools/     our code (km780.py: transport + GET commands, read-only;
               extract.sh/view.sh: decode the USBPcap captures)
    captures/  the 11 USB captures of the vendor software: .pcapng (raw) and
               .txt (decoded payloads). Versioned and **to keep until the
               protocol is fully documented** — they are the evidence, and
               they cannot be recaptured without the Windows machine.
    screenshots/ the vendor UI: what the app was showing when each capture
               was made. Also to keep — it is what ties a packet on the wire
               to the control that produced it.
    firmware/  recovery/: the three .hex files (FW 75/76/77), the MCU and the
               procedure that got the keyboard out of the bootloader
    sessions/  transcript of the Windows session (not versioned, 22 MB)
    external/  third-party material: installers, the abandoned repo

Nothing leaves `captures/` or `screenshots/` while there is still a gap in
the specification. Reopening a question about a byte means going back to the
packet and to the screen that produced it, and the Windows machine is not a
resource one can assume will be available again.

`START-HERE.txt` is the state summary brought over from the Windows machine:
what Phase 1 closed, what is left in Phase 2, and the warning not to sweep
unknown commands on the keyboard.
