#!/usr/bin/env python3
"""Toggles a mode toggle-byte on and off, at your pace, and puts it back.

    tools/km780toggle.py            works on 107 and then on 108
    tools/km780toggle.py 107 108 119    on the bytes you list
    tools/km780toggle.py --mode 2 107   on a specific mode

For telling `LightDisable` from `BacklightDisable`, which are bytes 107 and 108
in some order (docs/23 §9). Each step waits for Enter, so you can look at the
keyboard calmly before moving on.

WRITES -- but only through the safe path of docs/09 §5.3: read the mode with
0x82, change ONE byte, write it back with 0x02. The unknown fields travel
intact. At the end it restores and checks byte for byte against the initial
state, including on an exception or Ctrl+C.

If the change has no immediate effect, switch mode on the keyboard itself
(M1/M2/M3) and come back -- these toggles are reported to take only that way.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K

NAMES = {107: 'LightDisable or BacklightDisable',
         108: 'the other of the two',
         119: 'LightVolumeDisable (volume display)',
         109: 'Windows key lock'}


def wait(msg):
    try:
        input('      %s  [Enter] ' % msg)
    except EOFError:
        print()


def main():
    av = [a for a in sys.argv[1:] if not a.startswith('--')]
    mode = None
    if '--mode' in sys.argv:
        mode = int(sys.argv[sys.argv.index('--mode') + 1])
    targets = [int(a) for a in av] or [107, 108]
    for o in targets:
        if not 0 <= o < K.PKT:
            sys.exit('offset %d out of 0..263' % o)

    print(__doc__)
    with K.Keyboard() as kb:
        m = mode or kb.active_mode()
        initial = bytes(kb.read_mode(m))
        print('mode %d.  initial state of the targets:' % m)
        for o in targets:
            print('   byte %3d = %d   %s' % (o, initial[o], NAMES.get(o, '')))
        wait('ready to start?')
        try:
            for o in targets:
                orig = initial[o]
                other = 0 if orig else 1
                for value in (other, orig):
                    d = kb.read_mode(m)
                    d[o] = value
                    kb.write_mode(m, bytes(d))
                    got = kb.read_mode(m)[o]
                    note = '' if got == value else '   (re-read returned %d!)' % got
                    print('\n  byte %3d = %d%s' % (o, value, note))
                    wait('look at the keyboard. did anything change?')
        finally:
            print('\nrestoring...')
            d = kb.read_mode(m)
            for o in targets:
                d[o] = initial[o]
            kb.write_mode(m, bytes(d))
            final = bytes(kb.read_mode(m))
            diff = [i for i in range(K.PKT)
                    if initial[i] != final[i] and i != K.OFF_STATUS]
            if diff:
                print('WARNING: still different from the initial state at bytes %s' % diff)
            else:
                print('checked: mode %d came back identical to the initial state' % m)


main()
