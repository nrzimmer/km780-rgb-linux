#!/usr/bin/env python3
"""Per-channel color test on the G.SKILL KM780 RGB.

Protocol as per docs/09-protocol.md:
  264-byte packet = HID Feature report, ID 7, on interface 1
  buf[0]=0x07  buf[1]=command  buf[2]=profile+1  buf[3]=channel(1..3)  buf[8..]=144 slots

Subcommands:
  info                   read the current profile (0x81) and the matrix (0x89), no writing
  backup <file>          save the 3 channels read via 0x89
  test                   light F5/F6/F7, one channel each (WRITES)
  restore <file>         write the 3 channels of a backup back (WRITES)
"""
import sys, time, json
import usb.core, usb.util

VID, PID = 0x28DA, 0x1101
OUT_IFACE, IN_IFACE = 1, 2
PKT = 0x108
WVALUE, WINDEX = 0x0307, 0x0001
REPORT_ID = 0x07
HDR = 8
NSLOTS = 144
TIMEOUT = 1000

REQ_OUT = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_OUT
REQ_IN = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_IN
SET_REPORT, GET_REPORT = 0x09, 0x01

SLOT = {'F5': 45, 'F6': 54, 'F7': 59}


class KB:
    def __init__(self):
        self.dev = usb.core.find(idVendor=VID, idProduct=PID)
        if self.dev is None:
            sys.exit('keyboard 28da:1101 not found')
        for i in (OUT_IFACE, IN_IFACE):
            try:
                self.dev.detach_kernel_driver(i)
            except (usb.core.USBError, NotImplementedError):
                pass
            usb.util.claim_interface(self.dev, i)

    def close(self):
        for i in (OUT_IFACE, IN_IFACE):
            try:
                usb.util.release_interface(self.dev, i)
                self.dev.attach_kernel_driver(i)
            except Exception:
                pass

    def _pkt(self, cmd, profile=0, channel=0, payload=None):
        b = bytearray(PKT)
        b[0] = REPORT_ID
        b[1] = cmd
        b[2] = profile + 1
        b[3] = channel
        if payload:
            b[HDR:HDR+len(payload)] = payload
        return bytes(b)

    def send(self, cmd, profile=0, channel=0, payload=None, wait=0.02):
        self.dev.ctrl_transfer(REQ_OUT, SET_REPORT, WVALUE, WINDEX,
                               self._pkt(cmd, profile, channel, payload), TIMEOUT)
        time.sleep(wait)

    def query(self, cmd, profile=0, channel=0, wait=0.02):
        self.send(cmd, profile, channel, wait=wait)
        d = bytes(self.dev.ctrl_transfer(REQ_IN, GET_REPORT, WVALUE, WINDEX, PKT, TIMEOUT))
        time.sleep(wait)
        return d


def hexdump(b, n=48):
    return ' '.join('%02x' % c for c in b[:n])


def do_info(kb):
    print('--- 0x81 Current Profile:')
    d = kb.query(0x81, wait=0.01)
    print('    %s ...' % hexdump(d))
    print('--- 0x89 LED Matrix, per channel:')
    for ch in (1, 2, 3):
        d = kb.query(0x89, channel=ch, wait=0.05)
        sl = d[HDR:HDR+NSLOTS]
        nz = [(i, v) for i, v in enumerate(sl) if v]
        print('    channel %d: hdr=%s' % (ch, hexdump(d, 8)))
        print('             %d non-zero slots %s' % (len(nz), nz[:12]))


def do_backup(kb, path):
    out = {}
    for ch in (1, 2, 3):
        d = kb.query(0x89, channel=ch, wait=0.05)
        out[str(ch)] = list(d[HDR:HDR+NSLOTS])
    json.dump(out, open(path, 'w'))
    print('backup of the 3 channels saved to %s' % path)


def do_test(kb):
    plan = [(1, 'F5'), (2, 'F6'), (3, 'F7')]
    print('--- writing 3 packets (one per channel):')
    for ch, key in plan:
        pay = bytearray(NSLOTS)
        pay[SLOT[key]] = 0xFF
        kb.send(0x09, channel=ch, payload=bytes(pay), wait=0.02)
        print('    channel %d -> slot %d (%s) = 0xFF, rest 0' % (ch, SLOT[key], key))
    time.sleep(0.1)          # the extra wait the official app does after the 3rd channel
    print('--- done. F5 shows channel 1, F6 channel 2, F7 channel 3.')


def do_restore(kb, path):
    b = json.load(open(path))
    for ch in (1, 2, 3):
        kb.send(0x09, channel=ch, payload=bytes(b[str(ch)]), wait=0.02)
        print('    channel %d restored' % ch)
    time.sleep(0.1)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    kb = KB()
    try:
        if cmd == 'info':
            do_info(kb)
        elif cmd == 'backup':
            do_backup(kb, sys.argv[2])
        elif cmd == 'test':
            do_test(kb)
        elif cmd == 'restore':
            do_restore(kb, sys.argv[2])
        else:
            sys.exit('unknown subcommand: %s' % cmd)
    finally:
        kb.close()
