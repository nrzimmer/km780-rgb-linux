#!/usr/bin/env python3
"""Hardware-free tests of the KM780 tools.

    tools/test_km780.py

Swaps the real Keyboard for a simulator that records the packets and answers
read commands the way the keyboard does. Checks what can be checked without the
device: packet layout, read-change-write, and command-line parsing. It does NOT
replace running on the keyboard -- it only guarantees that, when it does run, it
will not fall over on a silly mistake.
"""
import sys, os, json, tempfile
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K

failures = []

def ok(cond, msg):
    print(('  ok   ' if cond else '  FAIL ') + msg)
    if not cond:
        failures.append(msg)


class Simulator:
    """Fake keyboard. Keeps state per mode and records what it received."""
    def __init__(self):
        self.sent = []
        self.active = 2
        self.modes = {}
        self.colors = {}
        for m in (1, 2, 3):
            d = bytearray(K.PKT)
            d[0], d[1], d[2], d[3] = 0x07, K.INFO_GET, m, 0x03
            d[K.OFF_POLLING], d[K.OFF_EFFECT], d[K.OFF_BRIGHTNESS] = 1, 0, 3
            for i in range(60, 108):          # "unknown" fields, marked
                d[i] = (i * 7) & 0xff
            self.modes[m] = d
            self.colors[m] = [bytearray(K.NSLOTS) for _ in range(3)]

    def write(self, packet, wait=0):
        self.sent.append(bytes(packet))
        b = packet
        if b[1] == K.INFO_SET:
            d = bytearray(b); d[1] = K.INFO_GET; d[3] = 0x03
            self.modes[b[2]] = d
        elif b[1] == K.LED_SET:
            self.colors[b[2]][b[3] - 1] = bytearray(b[K.HDR:K.HDR + K.NSLOTS])

    def read(self, packet, wait=0):
        self.sent.append(bytes(packet))
        c, m = packet[1], packet[2]
        if c == K.MODE_GET:
            r = bytearray(K.PKT); r[0], r[1], r[2] = 0x07, K.MODE_GET, self.active
            return bytes(r)
        if c == K.INFO_GET:
            return bytes(self.modes[m])
        if c == K.LED_GET:
            ch = packet[3]
            r = bytearray(K.PKT); r[0], r[1], r[2], r[3] = 0x07, K.LED_GET, m, ch
            r[K.HDR:K.HDR + K.NSLOTS] = self.colors[m][ch - 1]
            return bytes(r)
        raise AssertionError('command not expected in the simulator: %02x' % c)

    # the interface km780ctl uses
    build = K.Keyboard.build
    active_mode = K.Keyboard.active_mode
    read_mode = K.Keyboard.read_mode
    write_mode = K.Keyboard.write_mode
    read_colors = K.Keyboard.read_colors
    write_colors = K.Keyboard.write_colors
    def close(self): pass
    def __enter__(self): return self
    def __exit__(self, *a): pass


def run(sim, argv):
    """Runs km780ctl with the simulator in place of the Keyboard."""
    import importlib
    K.Keyboard_real, K.Keyboard = K.Keyboard, lambda: sim
    old = sys.argv[:]
    sys.argv = ['km780ctl.py'] + argv
    try:
        for mod in list(sys.modules):
            if mod == 'km780ctl':
                del sys.modules[mod]
        importlib.import_module('km780ctl')
    finally:
        sys.argv = old
        K.Keyboard = K.Keyboard_real


def main():
    print('--- library ---')
    ok(len(K.SLOTS) == 132, '132 slots in the table')
    ok(len(set(K.SLOTS)) == 132, 'key names are unique')
    ok(K.slot_of('F5') == 45 and K.slot_of('f5') == 45, 'F5 is slot 45, case-insensitive')
    ok(K.slot_of('45') == 45, 'the slot number works too')
    ok(K.color_of('f00') == (255, 0, 0) == K.color_of('FF0000') == K.color_of('red'),
       'color accepts RGB, RRGGBB and a name')
    for bad in ('DoesNotExist', 'gg0000', '12345'):
        try:
            K.slot_of(bad) if bad == 'DoesNotExist' else K.color_of(bad)
            ok(False, 'invalid input %r should have raised' % bad)
        except ValueError:
            ok(True, 'invalid input %r refused' % bad)

    print('\n--- packet layout ---')
    sim = Simulator()
    p = sim.build(K.LED_SET, mode=2, b3=3, payload=bytes([0xff] * K.NSLOTS))
    ok(len(p) == 264, 'the packet has 264 bytes')
    ok(p[0] == 0x07, 'buf[0] is Report ID 0x07')
    ok(p[1] == 0x09 and p[2] == 2 and p[3] == 3, 'cmd/mode/channel in buf[1..3]')
    ok(p[8] == 0xff and p[8 + 143] == 0xff and p[8 + 144] == 0x00,
       'the 144 slots occupy exactly bytes 8..151')

    print('\n--- set: paints one key and preserves the others ---')
    sim = Simulator()
    sim.colors[2][0][21] = 0x11        # A already had red 0x11
    sim.colors[2][1][50] = 0x22        # Space already had green 0x22
    run(sim, ['set', 'F5=ff0000'])
    ok(sim.colors[2][0][45] == 0xff and sim.colors[2][1][45] == 0
       and sim.colors[2][2][45] == 0, 'F5 came out pure red')
    ok(sim.colors[2][0][21] == 0x11 and sim.colors[2][1][50] == 0x22,
       'the other keys were preserved')
    leds = [b for b in sim.sent if b[1] == K.LED_SET]
    ok(len(leds) == 3, 'one color update is 3 packets, one per channel')
    ok([b[3] for b in leds] == [1, 2, 3], 'the channels go out in the order 1, 2, 3')
    ok(all(b[2] == 2 for b in leds), 'wrote to the active mode (2), not to 1')

    print('\n--- brightness: read, change one byte, write ---')
    sim = Simulator()
    before = bytes(sim.modes[2])
    run(sim, ['brightness', '4'])
    after = sim.modes[2]
    ok(after[K.OFF_BRIGHTNESS] == 4, 'byte 11 became 4')
    diff = [i for i in range(K.PKT) if before[i] != after[i]]
    ok(diff == [K.OFF_BRIGHTNESS], 'ONLY byte 11 changed; the unknown ones travelled intact (changed: %s)' % diff)
    sets = [b for b in sim.sent if b[1] == K.INFO_SET]
    ok(len(sets) == 1 and sets[0][K.OFF_STATUS] == 0,
       'wrote one 0x02 with the status byte zeroed')

    print('\n--- -n writes nothing ---')
    sim = Simulator()
    run(sim, ['-n', 'all', 'blue'])
    ok(not [b for b in sim.sent if b[1] in (K.LED_SET, K.INFO_SET)],
       'with -n no write packet was sent')

    print('\n--- backup and restore ---')
    sim = Simulator()
    run(sim, ['all', 'ff8800'])
    path = os.path.join(tempfile.mkdtemp(), 'b.json')
    run(sim, ['backup', path])
    saved = json.load(open(path))
    ok(saved['colors']['2'][0][45] == 0xff and saved['colors']['2'][1][45] == 0x88,
       'the backup saved the colors of all 3 channels')
    run(sim, ['off'])
    ok(sim.colors[2][0][45] == 0, 'off zeroed it')
    run(sim, ['restore', path])
    ok(sim.colors[2][0][45] == 0xff and sim.colors[2][1][45] == 0x88,
       'restore brought the colors back')

    print('\n--- --mode acts on the requested mode ---')
    sim = Simulator()
    run(sim, ['--mode', '1', 'brightness', '2'])
    ok(sim.modes[1][K.OFF_BRIGHTNESS] == 2, 'mode 1 changed')
    ok(sim.modes[2][K.OFF_BRIGHTNESS] == 3, 'mode 2 (the active one) stayed intact')

    print('\n--- limits refused ---')
    for argv in (['brightness', '5'], ['effect', '6'], ['set', 'F5']):
        sim = Simulator()
        try:
            run(sim, argv)
            ok(False, '%s should have been refused' % ' '.join(argv))
        except SystemExit:
            ok(True, '%s refused' % ' '.join(argv))

    print('\n%d test(s) failed' % len(failures) if failures else '\nALL OK')
    return 1 if failures else 0


sys.exit(main())
