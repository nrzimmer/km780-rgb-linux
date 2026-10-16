#!/usr/bin/env python3
"""Read-only probing of the G.SKILL Ripjaws KM780 RGB (28da:1101).

Transport derived from legonigel/KM780-RGB-Linux-Driver (GPL-3), src/keyboard.cpp.
Writes nothing to the keyboard: it only emits GET commands (0x8x).
"""
import sys, time
import usb.core, usb.util

VID, PID = 0x28DA, 0x1101
OUT_IFACE, IN_IFACE, EP_IN = 1, 2, 0x83
START = 0x07
PKT = 0x108          # 264 bytes
WVALUE, WINDEX = 0x0307, 0x0001
TIMEOUT = 500

REQ_OUT = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_OUT
REQ_IN  = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_IN
SET_REPORT, GET_REPORT = 0x09, 0x01

CMDS = {
    "profile":     0x81,   # GetCurrentProfile       (answer: interrupt)
    "profinfo":    0x82,   # GetProfileInformation   (answer: control IN)
    "ledeffect":   0x88,   # GetLEDEffectDatabase    (answer: control IN)
    "ledmatrix":   0x89,   # GetLEDMatrixDatabase    (answer: control IN)
}
CONTROL_RESP = {0x82, 0x88, 0x89}


class KM780:
    def __init__(self):
        self.dev = usb.core.find(idVendor=VID, idProduct=PID)
        if self.dev is None:
            sys.exit("keyboard 28da:1101 not found")
        for i in (OUT_IFACE, IN_IFACE):
            try:
                self.dev.detach_kernel_driver(i)
            except (usb.core.USBError, NotImplementedError):
                pass   # it was already detached
            usb.util.claim_interface(self.dev, i)

    def close(self):
        for i in (OUT_IFACE, IN_IFACE):
            try:
                usb.util.release_interface(self.dev, i)
                self.dev.attach_kernel_driver(i)
            except Exception:
                pass

    def cmd(self, command, payload=b""):
        buf = bytearray(PKT)
        buf[0], buf[1] = START, command
        buf[2:2+len(payload)] = payload
        self.dev.ctrl_transfer(REQ_OUT, SET_REPORT, WVALUE, WINDEX, bytes(buf), TIMEOUT)

        ack = None
        for _ in range(12):                      # wait for the 8-byte ACK on EP3 IN
            try:
                r = bytes(self.dev.read(EP_IN, 8, TIMEOUT))
            except usb.core.USBError:
                continue
            if len(r) >= 3 and r[0] == 0 and r[1] == 0 and r[2] == command:
                ack = r
                break
        data = None
        if command in CONTROL_RESP:
            data = bytes(self.dev.ctrl_transfer(REQ_IN, GET_REPORT, WVALUE, WINDEX, PKT, TIMEOUT))
        return ack, data


def hexdump(b, width=16):
    out = []
    for o in range(0, len(b), width):
        chunk = b[o:o+width]
        out.append("%04x  %-*s |%s|" % (
            o, width*3-1, " ".join("%02x" % c for c in chunk),
            "".join(chr(c) if 32 <= c < 127 else "." for c in chunk)))
    return "\n".join(out)


if __name__ == "__main__":
    which = sys.argv[1:] or list(CMDS)
    kb = KM780()
    try:
        for name in which:
            if name not in CMDS:
                print("unknown command: %s" % name); continue
            c = CMDS[name]
            print("\n########## %s (0x%02x)" % (name, c))
            try:
                ack, data = kb.cmd(c)
            except usb.core.USBError as e:
                print("  ERROR: %s" % e); continue
            print("  ACK : %s" % (ack.hex(" ") if ack else "<none>"))
            if data is not None:
                nz = sum(1 for x in data if x)
                print("  DATA: %d bytes, %d non-zero" % (len(data), nz))
                print(hexdump(data))
            time.sleep(0.1)
    finally:
        kb.close()
