#!/usr/bin/env python3
"""Tests values of byte 10 (effect) of a mode, one at a time, with restore.

docs/27 section 7 found a ninth effect in Setting.dll -- index 8,
"Checkpoint", profile `.chk` -- that the v2.04 UI never offered, and showed
that indices 5 and 6 have no branch in the builder. This script writes each
candidate into byte 10 and stops waiting for Enter, so the operator can look
at the keyboard.

  km780effect.py <mode> <value> [value...]

The original value is read beforehand and **written back in a finally**, even
on an exception or Ctrl+C. It opens no 0x16 session and probes no command: only
the 0x82/0x02 read-change-write pair, the same path `km780ctl.py brightness`
uses. See the operating rules in STATUS.md.
"""
import sys
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import km780lib as K

NAMES = {0: 'none (static)', 1: 'Wave', 2: 'Ripple', 3: 'Sonic',
         4: 'Breathing', 5: 'no branch in the builder', 6: 'no branch in the builder',
         7: 'Changeling', 8: 'Checkpoint  <- never seen in the UI', 9: 'Cycle',
         10: 'Rain'}


def main():
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    mode = int(sys.argv[1])
    values = [int(a) for a in sys.argv[2:]]
    if mode not in (1, 2, 3):
        sys.exit('mode %d does not exist; use 1, 2 or 3' % mode)
    for v in values:
        if not 0 <= v <= 255:
            sys.exit('value %d out of 0-255' % v)

    with K.Keyboard() as kb:
        d = bytearray(kb.read_mode(mode))
        orig = d[K.OFF_EFFECT]
        print('mode %d: byte 10 (effect) is %d (%s)'
              % (mode, orig, NAMES.get(orig, '?')))
        print('brightness %d -- if it is 0 nothing will show' % d[K.OFF_BRIGHTNESS])
        print('the original value will be written back on exit, always.\n')
        try:
            for v in values:
                d[K.OFF_EFFECT] = v
                kb.write_mode(mode, bytes(d))
                got = kb.read_mode(mode)[K.OFF_EFFECT]
                print('  byte 10 <- %-3d (%s)  | re-read: %d%s'
                      % (v, NAMES.get(v, '?'), got,
                         '' if got == v else '  <-- DID NOT TAKE'))
                input('     look at the keyboard and press Enter for the next... ')
        finally:
            d[K.OFF_EFFECT] = orig
            kb.write_mode(mode, bytes(d))
            back = kb.read_mode(mode)[K.OFF_EFFECT]
            print('\nrestored: byte 10 = %d%s'
                  % (back, '' if back == orig else '  <-- FAILED, replug'))


if __name__ == '__main__':
    main()
