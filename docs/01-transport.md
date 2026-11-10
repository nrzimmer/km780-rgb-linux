# USB transport layer (known)

Source: [`legonigel/KM780-RGB-Linux-Driver`](https://github.com/legonigel/KM780-RGB-Linux-Driver)
(GPL-3.0), `src/keyboard.cpp`. The project never worked as an application, but
the transport is correct and confirmed against the hardware (see below). Its
code is not copied here — see [`00-references.md`](00-references.md).

## Device

- VID `0x28DA` / PID `0x1101`
- `product` = `G.SKILL RIPJAWS KM780 RGB GAMING KEYBOARD`

Three HID interfaces, confirmed with `lsusb -v` on this machine:

| Interface | bInterfaceProtocol | Endpoint | Descriptor | Role |
|---|---|---|---|---|
| 0 | 1 (Keyboard) | `0x81` IN | 65 B | HID keyboard — **do not touch**, this is typing |
| 1 | 2 (Mouse) | `0x82` IN | 252 B | command output |
| 2 | 0 | `0x83` IN | 26 B | answer input |

All endpoints are Interrupt, `wMaxPacketSize` 64.

Note: interface 1 declares itself as "Mouse" but is used as the command
channel. Since the control only uses interfaces 1 and 2, the keyboard can be
probed without interrupting typing.

## Command format

Control transfer OUT:

| Field | Value |
|---|---|
| `bmRequestType` | `0x21` (CLASS \| INTERFACE \| OUT) |
| `bRequest` | `0x09` (SET_REPORT) |
| `wValue` | `0x0307` |
| `wIndex` | `0x0001` |
| `wLength` | `0x0108` (264 bytes) |

264-byte payload:

    byte 0   = 0x07   (magic start)
    byte 1   = command
    byte 2.. = data (up to 262 bytes)

## Answer

1. **ACK**: 8 bytes on endpoint `0x83` IN (interrupt). Valid when
   `r[0] == 0 && r[1] == 0 && r[2] == command`.
2. **Data** (only for some commands): a second control transfer IN —
   `bmRequestType` `0xA1`, `bRequest` `0x01` (GET_REPORT), same
   `wValue`/`wIndex`, 264 bytes.

## Commands

| Function | Set | Get | Get's answer |
|---|---|---|---|
| Current Profile | `0x01` | `0x81` | interrupt |
| Profile Information | `0x02` | `0x82` | control IN |
| LED Effect Database | `0x08` | `0x88` | control IN |
| LED Matrix Database | `0x09` | `0x89` | control IN |

The names are symbolic — they came from inside the official app, not from hex
analysis. That is a hint that the app is decompilable (see
`03-decompilation.md`).

## What is missing

1. Layout of the 264 bytes of `0x09` (SetLEDMatrixDatabase): where the RGB
   values sit and in what order (R/G/B vs B/G/R).
2. **Key → offset map.** The KM780 has ~104 keys plus side lighting. At
   3 bytes/key that does not fit in 264 bytes, so there must be several
   packets per change, with a page/index field. Identifying that paging field
   is probably the hardest part.
3. The init/handshake sequence when the software opens.
4. Whether there is a commit/apply command after sending the matrix.
5. The format of `0x08` (LED Effect Database) for animated effects.

## Permissions on Linux

The keyboard's `/dev/bus/usb/...` is `root:root 0664`. To access it without
root:

    # /etc/udev/rules.d/40-km780.rules
    SUBSYSTEM=="usb", ATTRS{idVendor}=="28da", ATTRS{idProduct}=="1101", MODE="0660", TAG+="uaccess"
