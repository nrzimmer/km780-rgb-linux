#!/usr/bin/env python3
"""Finds out which command enables key reporting (f6) on EP 0x83.

    tools/km780f6.py

On Windows f6 was already on from an earlier run of the software, so the
captures never showed who turns it on. Here the keyboard was power-cycled and
receives no f6 at all. This script tests the three init candidates, ONE AT A
TIME, from the cheapest to the most expensive.

The attribution is clean: if f6 shows up in phase N, the culprit is phase N's
command, because the ones from earlier phases did not enable it.

WRITES -- but only the three commands below, all with a fixed payload copied
from a real capture of the official software, and none of them touches user
data:

    07 07 00 15      the unexplained command, last message of the init
    07 16 01         opens the session, first message of the app
    07 0d 00 / 01    enters and leaves configuration mode

Before and after, it reads the 3 modes and the 3 color channels and compares
byte by byte, to say what the commands changed -- if they changed anything.
"""
import sys, os, time, termios
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K
import usb.core

LISTEN = 10.0      # seconds of listening per phase
KEY = 'G1'

PHASES = [
    ('control', None, 'no command yet -- the expectation is that NOTHING shows up'),
    ('0x07', (0x07, 0x00, 0x15), 'the unexplained command at the end of the init'),
    ('0x16', (0x16, 0x01, 0x00), 'opens the session'),
    ('0x0D enter', (0x0D, 0x00, 0x00), 'enters configuration mode'),
    ('0x0D leave', (0x0D, 0x01, 0x00), 'leaves configuration mode -- turns f6 off?'),
]


class EchoOff:
    def __enter__(self):
        self.fd = sys.stdin.fileno()
        try:
            self.old = termios.tcgetattr(self.fd)
            n = termios.tcgetattr(self.fd)
            n[3] &= ~termios.ECHO
            termios.tcsetattr(self.fd, termios.TCSADRAIN, n)
        except termios.error:
            self.old = None
        return self

    def __exit__(self, *a):
        if self.old:
            termios.tcsetattr(self.fd, termios.TCSADRAIN, self.old)
        try:
            termios.tcflush(self.fd, termios.TCIFLUSH)
        except termios.error:
            pass


def classify(r):
    if r[0] == 0xf6 and r[1] == 0x01:
        return 'F6 KEY', 'column=%d row=%d' % (r[2], r[3])
    if r[0] == 0x00:
        return 'ack', 'cmd=0x%02x byte4=%d' % (r[2], r[4])
    if r[0] == 0x02 and r[1] == 0x04:
        t = {0x08: 'mode/presence', 0x09: 'brightness level=%d' % r[4]}.get(r[3], 'type 0x%02x' % r[3])
        return 'state', 'mode=%d %s' % (r[2], t)
    return 'OTHER', ''


def listen(kb, seconds):
    """Returns the list of events (ignores the ACKs of our own commands)."""
    out = []
    t0 = time.time()
    while time.time() - t0 < seconds:
        try:
            r = bytes(kb.dev.read(K.EP_EVENTS, 64, 250))
        except usb.core.USBError:
            continue
        kind, det = classify(r)
        if kind == 'ack':
            continue
        out.append((time.time() - t0, r, kind, det))
    return out


def portrait(kb):
    """Full state, for comparing before and after."""
    return {'modes': {m: bytes(kb.read_mode(m)) for m in (1, 2, 3)},
            'colors': {m: [bytes(c) for c in kb.read_colors(m)] for m in (1, 2, 3)}}


def compare(a, b):
    changed = []
    for m in (1, 2, 3):
        d = [i for i in range(K.PKT)
             if a['modes'][m][i] != b['modes'][m][i] and i != K.OFF_STATUS]
        if d:
            changed.append('mode %d: bytes %s' % (m, d[:12]))
        for ci in range(3):
            d = [i for i in range(K.NSLOTS) if a['colors'][m][ci][i] != b['colors'][m][ci][i]]
            if d:
                changed.append('color mode %d channel %d: slots %s' % (m, ci + 1, d[:12]))
    return changed


def main():
    print(__doc__)
    input('Enter to start (Ctrl+C to give up). ')
    with K.Keyboard() as kb:
        print('\nreading the before state...')
        before = portrait(kb)
        print('  active mode: %d' % kb.active_mode())

        found_in = None
        with EchoOff():
            for name, cmd, desc in PHASES:
                print('\n' + '=' * 62)
                print('PHASE  %s  --  %s' % (name, desc))
                if cmd:
                    c, b2, b3 = cmd
                    packet = kb.build(c, mode=b2, b3=b3)
                    print('  sending: %s' % packet[:6].hex(' '))
                    kb.write(packet, wait=0.2)
                else:
                    print('  (nothing sent)')
                print('  >>> PRESS %s SEVERAL TIMES, for the next %d seconds <<<'
                      % (KEY, int(LISTEN)))
                evs = listen(kb, LISTEN)
                f6 = [e for e in evs if e[2] == 'F6 KEY']
                for t, r, kind, det in evs[:10]:
                    print('    %5.1fs  %-9s %-26s %s' % (t, kind, det, r[:8].hex(' ')))
                if len(evs) > 10:
                    print('    ... %d more events' % (len(evs) - 10))
                if not evs:
                    print('    (no events)')
                print('  --> %d event(s), %d of them f6' % (len(evs), len(f6)))
                if f6 and found_in is None:
                    found_in = name
                    print('  *** f6 SHOWED UP HERE. The culprit is this phase\'s command. ***')

        print('\n' + '=' * 62)
        print('reading the after state...')
        after = portrait(kb)
        changed = compare(before, after)
        print('\nRESULT')
        if found_in:
            print('  f6 started showing up in phase: %s' % found_in)
        else:
            print('  f6 did NOT show up in any phase -- none of the three enables it,')
            print('  or it depends on something else not yet identified')
        if changed:
            print('  WARNING: the commands changed keyboard state:')
            for m in changed:
                print('     %s' % m)
        else:
            print('  none of the commands altered the 3 modes or the color matrix')
            print('  (compared byte by byte, before and after)')


main()
