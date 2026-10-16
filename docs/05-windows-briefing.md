# KM780 RGB — briefing for the Windows session

## Goal
Work out the USB LED-control protocol of the G.SKILL Ripjaws KM780 RGB
keyboard (USB `28da:1101`) by capturing G.SKILL's official software on Windows
with USBPcap/Wireshark, and then write a color utility on Linux.

## What we ALREADY know (transport — no need to rediscover)

Taken from `legonigel/KM780-RGB-Linux-Driver` (GPL-3, abandoned, 3 commits,
the last on 2017-02-25), file `src/keyboard.cpp`. The repo never worked as an
application, but the transport layer is correct and documented:

- Device: VID `0x28DA`, PID `0x1101`
- Interface 0 = HID keyboard (DO NOT TOUCH — this is typing)
- Interface 1 = command output
- Interface 2 = answer input, endpoint `0x83` IN
- Command = control transfer:
  - `bmRequestType` = CLASS | INTERFACE | OUT  (`0x21`)
  - `bRequest` = `0x09` (SET_REPORT)
  - `wValue` = `0x0307`
  - `wIndex` = `0x0001`
  - `wLength` = `0x0108` (264 bytes)
  - payload[0] = `0x07` (magic start), payload[1] = command byte, rest = data
- ACK: 8 bytes on EP `0x83` IN; valid when `[0]==0 && [1]==0 && [2]==command`
- Read: a second control transfer IN (`0xA1`, `bRequest=0x01` GET_REPORT, same
  wValue/wIndex, 264 bytes)

Known commands (set / get):
| Function | Set | Get |
|---|---|---|
| Current Profile | `0x01` | `0x81` |
| Profile Information | `0x02` | `0x82` |
| LED Effect Database | `0x08` | `0x88` |
| LED Matrix Database | `0x09` | `0x89` |

Answers via control transfer IN: `0x82`, `0x88`, `0x89`. Via interrupt: the
rest.

## What is MISSING (the point of the capture)

1. Layout of the 264 payload bytes of `0x09` (SetLEDMatrixDatabase): where the
   RGB bytes sit, and in what order (R/G/B vs B/G/R).
2. Key -> offset map. The KM780 has ~104 keys plus side lighting. At
   3 bytes/key that does NOT fit in 264 bytes, so expect several packets per
   color change, with some page/index field. **Identifying that paging field is
   probably the hardest part.**
3. The init/handshake sequence the software performs on opening.
4. Whether there is a "commit"/apply command after sending the matrix.
5. The format of `0x08` (LED Effect Database) for the animated effects.

## Capture plan (one atomic change per .pcapng file)

Install Wireshark with **USBPcap** ticked in the installer (it is not there by
default), reboot, run as admin, pick the keyboard's root hub in USBPcapCMD.
Filter in Wireshark to trim it down: `usb.idVendor == 0x28da`, or by the device
address.

Save each capture under a clear name:

| File | Action in the G.SKILL software | Reveals |
|---|---|---|
| `01-open.pcapng` | open the software, touch nothing | init/handshake, reads |
| `02-all-red.pcapng` | everything `#FF0000` | color bytes, channel order |
| `03-all-green.pcapng` | everything `#00FF00` | confirms channel order |
| `04-all-off.pcapng` | everything `#000000` | baseline for the diff |
| `05-only-A-red.pcapng` | only the A key red, rest dark | offset of one key |
| `06-only-S-red.pcapng` | only the S key (A's neighbour) | stride between keys |
| `07-only-esc-red.pcapng` | only Esc | is the order physical or by scancode? |
| `08-effect.pcapng` | turn an animated effect on | `0x08` commands |
| `09-profile.pcapng` | switch profile | `0x01`/`0x02` commands |

Tip: between starting the capture and acting, wait ~2 s without touching
anything, so the action's traffic is isolated in time.

## Extraction with tshark

Control transfers OUT with the payload, in hex, one per line:

    tshark -r 02-all-red.pcapng -Y "usb.bmRequestType == 0x21" \
           -T fields -e frame.number -e usb.setup.wValue -e usb.data_fragment

If `usb.data_fragment` comes back empty, try `-e usb.capdata`. The fields vary
with the USBPcap/Wireshark version — check one packet with `-V` first.

## State on Linux (the Fedora machine, already set up)

- a venv with pyusb installed
- `km780.py`: implements the transport above and emits only GET commands
  (read-only, it does not write to the keyboard). Useful for validating the
  protocol afterwards.
- `tshark` was NOT installed (`sudo dnf install wireshark-cli` if needed)
- the keyboard's `/dev/bus/usb/...` is `root:root 0664` -> needs a udev rule
  (`SUBSYSTEM=="usb", ATTRS{idVendor}=="28da", ATTRS{idProduct}=="1101", MODE="0660", TAG+="uaccess"`)
  or running as root.

## Caution

Only write to the keyboard after understanding the structure. There is a report
on G.SKILL's forum of a KM780 needing a firmware reset via safety-pin + a .bin
file — that is, the keyboard can be left in a bad state. Reads (`0x8x`) are
safe.
