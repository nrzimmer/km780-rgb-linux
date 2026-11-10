# References

Third-party material used in this project. **Nothing here is redistributed in
the repository** — each item is pointed at its source, with what it solved and
what it did not.

## `legonigel/KM780-RGB-Linux-Driver`

<https://github.com/legonigel/KM780-RGB-Linux-Driver> — GPL-3.0, 3 commits,
the last on 2017-02-25.

The only open-source project aimed specifically at this keyboard. It never
worked as an application, but **its USB transport layer is correct**: VID/PID,
the three HID interfaces, which one accepts the Feature report, and the report
size. It is the basis of [`01-transport.md`](01-transport.md), where every
claim was reconfirmed against the hardware on this machine.

That code is GPL-3.0 and this repository is MIT, so the copy that used to live
in `external/repo-legonigel/` was removed, history included. To consult it,
clone the original — the relevant file is `src/keyboard.cpp`.

## G.SKILL's official software

The installers for the *RIPJAWS KM780 RGB Software* (V2.03 and V2.04) and for
the KM780 MX (v0.59), which is where `App.exe`, `Setting.dll`, `FWUpdate.exe`
and the firmware images came from.

They are not redistributed. [`06-binaries.md`](06-binaries.md) records where
they were obtained and how to extract them, so the analysis is reproducible
from the manufacturer's own download.

## OpenRGB

<https://openrgb.org> — supports G.SKILL only on the memory line (Trident Z /
Royal, over SMBus). No G.SKILL keyboard in its device list. Checked 2026-10.

## `sidewinderd`

<https://github.com/tolga9009/sidewinderd> — Microsoft keyboards. Useful as a
reference for how a keyboard daemon is structured on Linux, not for this
device.
