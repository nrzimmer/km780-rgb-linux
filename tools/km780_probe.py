#!/usr/bin/env python3
"""Probing the KM780 RGB: takes snapshots of the read commands and compares.

Reads the same command in two different keyboard states (e.g. maximum and
minimum brightness) and compares them. Whatever changes locates the field.

Subcommands:
  snap <file>         read the CONFIRMED read commands and save
  diff <a> <b>        compare two snapshots, byte by byte
  listen [seconds]    listen to the 8-byte event channel on EP 0x83

WARNING -- READ BEFORE TOUCHING THE READS LIST
----------------------------------------------
An earlier version of this file swept 0x85, 0x88, 0x8B, 0x8D, 0x92 and 0x96
"because they have bit 7 set, so they are reads". **That sweep is what put the
keyboard into bootloader mode.** Some byte between 0x96 and 0xAF is not a read,
despite bit 7.

The "bit 7 means read" convention holds for the confirmed write/read pairs, NOT
for arbitrary bytes. READS below only contains commands that were seen
answering as a read in a real USB capture of the official software. Do not add
anything there without that evidence: recovery exists, but it depends on a
Windows machine with FWUpdate.exe (see firmware/recovery/).
"""
import sys, time, json
import usb.core, usb.util
sys.path.insert(0, __file__.rsplit('/', 1)[0])
from km780_led import KB, HDR, NSLOTS, PKT

# Only read commands CONFIRMED on the wire -- see the warning at the top.
# These four are the only ones that appear as GET in the 11 captures
# (captures/*.txt, check with: awk -F'\t' '$3=="GET"{print $4}' | sort -u).
READS = {
    0x81: 'Current Mode',
    0x82: 'Mode Information',
    0x89: 'LED Matrix Database',
    0x8A: 'page-indexed database',
}

# Never observed answering as a read. Kept here only to record that they were
# tried and why they are not in READS.
DO_NOT_SWEEP = {
    0x85: 'the write 0x05 exists, but 0x85 never answered in a capture',
    0x88: 'the abandoned repo calls it "LED Effect Database"; does not exist in v2.04',
    0x8B: 'cited in the app timing, never seen on the wire',
    0x8D: 'predicted by the bit-7 rule, never observed',
    0x92: 'cited in the app timing, never seen on the wire',
    0x96: 'predicted by the bit-7 rule, never observed -- a suspect range',
}
EP_IN = 0x83


def snap(kb, path, label):
    out = {'label': label, 'data': {}}
    for cmd in sorted(READS):
        for ch in (0, 1, 2, 3):
            try:
                d = kb.query(cmd, channel=ch, wait=0.05)
            except usb.core.USBError as e:
                out['data']['%02x/%d' % (cmd, ch)] = 'ERROR: %s' % e
                continue
            out['data']['%02x/%d' % (cmd, ch)] = list(d)
        print('  0x%02X read (%s)' % (cmd, READS[cmd]))
    json.dump(out, open(path, 'w'))
    print('snapshot "%s" saved to %s' % (label, path))


def diff(pa, pb):
    a = json.load(open(pa)); b = json.load(open(pb))
    print('### diff  A="%s"  B="%s"\n' % (a['label'], b['label']))
    found = False
    for k in sorted(a['data']):
        va, vb = a['data'][k], b['data'].get(k)
        if isinstance(va, str) or isinstance(vb, str) or vb is None:
            continue
        d = [(i, x, y) for i, (x, y) in enumerate(zip(va, vb)) if x != y]
        if not d:
            continue
        found = True
        cmd = int(k.split('/')[0], 16)
        print('  0x%02X channel %s  -> %d differing byte(s)  %s' % (
            cmd, k.split('/')[1], len(d),
            ' '.join('[%d] %02x->%02x' % t for t in d[:10])))
    if not found:
        print('  no difference in any command.')


def listen(kb, secs):
    print('### listening on EP 0x%02x for %ds -- change the keyboard brightness now' % (EP_IN, secs))
    t0 = time.time()
    seen = 0
    while time.time() - t0 < secs:
        try:
            r = bytes(kb.dev.read(EP_IN, 8, 300))
        except usb.core.USBError:
            continue
        seen += 1
        print('  %6.2fs  %s' % (time.time() - t0, ' '.join('%02x' % c for c in r)))
    print('### %d report(s) received' % seen)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    c = sys.argv[1]
    if c == 'diff':
        diff(sys.argv[2], sys.argv[3]); sys.exit()
    kb = KB()
    try:
        if c == 'snap':
            snap(kb, sys.argv[2], sys.argv[3] if len(sys.argv) > 3 else sys.argv[2])
        elif c == 'listen':
            listen(kb, int(sys.argv[2]) if len(sys.argv) > 2 else 15)
        else:
            sys.exit('unknown subcommand: %s' % c)
    finally:
        kb.close()
