# Firmware recovery kit

Material brought from `Desktop/KM780-Recuperacao` on the Windows machine, used
on 2026-10-16 to get the keyboard out of bootloader mode. Only the text part:
`FWUpdate.exe`, `HidDevice.dll`, `GetOSVer.dll`, `devcon_*.exe` and `skins/`
were left out — they are third-party binaries, re-extractable from the
official installer (see `docs/06-binaries.md`).

| File | What it is |
|---|---|
| `Skill_1_FW_2017_10_11_77.hex` | FW 77 — what is on the keyboard today |
| `Skill_1_FW_2017_09_30_76.hex` | FW 76 |
| `Skill_1_FW_2017_09_05_75.hex` | FW 75 |
| `info.ini` | points at the firmware and the MCU: `IC_NAME=11U35` (LPC11U35) |
| `RECOVERY.txt` | the recovery procedure, as it was carried out |
| `Information.txt` | log from `FWUpdate.exe` of the recovery that worked |

## The three .hex files

They are Intel HEX, loading at `0x2000`–`0xa68f` (34448 bytes) — above the
bootloader, which occupies the first 8 KB and is what survives a bad firmware.

`firmware/fw77.bin` (not versioned) is exactly FW 77 converted from that
range; reproducible with any Intel HEX → binary converter, filling gaps with
`0xff`.

Having all three versions makes it possible to **diff FW 75 → 76 → 77**: the
changes between them point at the code that touches the protocol, which is
easier to find in a diff than by sweeping 34 KB of Thumb-2.
