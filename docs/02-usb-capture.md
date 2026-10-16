# Line of work: USB capture on Windows

## Priority: medium — it became validation, not discovery

This line was the main one until the binary analysis (`06-binaries.md`) showed
that `Setting.dll` preserves RTTI and class names. Now the capture serves
better as **ground truth** to confirm what the decompilation reveals, and to
settle specific ambiguities.

Inherent limit: a capture only shows what was exercised in the interface. The
binary has the whole table.

## Where to run it

The official software is Windows-only. There is a report on the forum that it
**does not detect the keyboard in a VM** with USB passthrough, so dual boot /
bare metal is the safe path. A VM is worth a try (it is cheap), but do not
count on it.

**Run Claude Code on the Windows machine itself**, rather than capturing there
and bringing it over to Fedora: with dual boot, every gap in the capture would
cost a reboot cycle, and something will always be missing. With Claude in the
Windows session the capture → analysis → recapture loop closes with no reboot.
The briefing to paste into that session is in `05-windows-briefing.md`.

## Tools

On Windows: Wireshark with **USBPcap** ticked in the installer (it is not there
by default), reboot, run as administrator, pick the keyboard's root hub in
`USBPcapCMD.exe`. Filter: `usb.idVendor == 0x28da`, or by the device address.

On Linux: `tshark` is already installed (`wireshark-cli`).

## Methodology: one atomic change per file

The diff is only readable if each capture isolates a single change. Wait ~2 s
between starting the capture and acting, so the traffic is isolated in time.

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

Save the `.pcapng` files in `captures/` (not versioned, see `.gitignore`).

## Extraction

Control transfers OUT with the payload, in hex, one per line:

    tshark -r captures/02-all-red.pcapng -Y "usb.bmRequestType == 0x21" \
           -T fields -e frame.number -e usb.setup.wValue -e usb.data_fragment

If `usb.data_fragment` comes back empty, try `-e usb.capdata`. The field names
vary with the USBPcap/Wireshark version — inspect a packet with `-V` first to
confirm.
