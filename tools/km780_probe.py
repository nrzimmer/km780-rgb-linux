#!/usr/bin/env python3
"""Read-only probing of the KM780 RGB: takes snapshots and compares them.

Idea: read the unknown commands in two different keyboard states (e.g.
maximum and minimum brightness) and compare. Whatever changes locates the field.

Subcommands:
  snap <file>         reads every 0x8x command and saves
  diff <a> <b>        compares two snapshots, byte by byte
  listen [seconds]    listens to the 8-byte event channel on EP 0x83
"""
import sys, time, json
import usb.core, usb.util
sys.path.insert(0, __file__.rsplit('/', 1)[0])
from km780_led import KB, HDR, NSLOTS, PKT

# read commands: bit 7 set. The named ones and the ones found in the DLL.
READS = {
    0x81: 'Current Profile',
    0x82: 'Profile Information',
    0x85: '(unknown)',
    0x88: 'LED Effect Database?',
    0x89: 'LED Matrix Database',
    0x8A: '(unknown)',
    0x8B: '(unknown, paged)',
    0x8D: '(predicted, not observed)',
    0x92: '(unknown, paged)',
    0x96: '(predicted, not observed)',
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
