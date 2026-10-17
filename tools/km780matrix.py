#!/usr/bin/env python3
"""Maps the KM780's physical matrix, key by key.

    tools/km780matrix.py [file.json]

The keyboard emits `f6 01 <column> <row>` on EP 0x83 for every key pressed
(docs/18 §3). This asks for one key at a time and records the coordinate that
arrives. READ ONLY -- no write command is sent.

How to use: press the key it asks for. If the key does not exist on your
keyboard, wait a few seconds and it skips on. Ctrl+C saves what it already has.

During collection the terminal echo is turned off, otherwise what you type
shows on screen and clogs the buffer.
"""
import sys, os, json, time, termios, tty
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K
import usb.core

WAIT = 6.0            # seconds before considering "the key does not exist"
DEBOUNCE = 0.35       # drains repeated events from the same press

# Order of the prompts: by physical row, so the user does not get lost.
ROWS = [
    ('G keys (left column)', ['G1', 'G2', 'G3', 'G4', 'G5', 'G6']),
    ('function row', ['Esc', 'F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8',
                      'F9', 'F10', 'F11', 'F12', 'PrtSc', 'ScrLk', 'Pause']),
    ('number row', ['`', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
                    '-', '=', 'Backspace']),
    ('Tab row', ['Tab', 'Q', 'W', 'E', 'R', 'T', 'Y', 'U', 'I', 'O', 'P',
                 '[', ']', '\\']),
    ('Caps row', ['CapsLock', 'A', 'S', 'D', 'F', 'G', 'H', 'J', 'K', 'L',
                  ';', "'", 'Enter']),
    ('Shift row', ['LShift', 'ISO\\| (the extra key next to LShift)',
                   'Z', 'X', 'C', 'V', 'B', 'N', 'M', ',', '.', '/', 'RShift']),
    ('bottom row', ['LCtrl', 'LWin', 'LAlt', 'Space', 'RAlt', 'RWin', 'Menu',
                    'RCtrl']),
    ('navigation', ['Insert', 'Home', 'PgUp', 'Delete', 'End', 'PgDn',
                    'Up', 'Left', 'Down', 'Right']),
    ('numeric keypad', ['NumLock', 'Num/', 'Num*', 'Num-', 'Num7', 'Num8', 'Num9',
                        'Num+', 'Num4', 'Num5', 'Num6', 'Num1', 'Num2', 'Num3',
                        'NumSep (keypad Enter)', 'Num0', 'Num.']),
    ('multimedia and extras', ['media / volume key 1', 'media / volume key 2',
                        'media / volume key 3', 'MR', 'M1', 'M2', 'M3',
                        'Timer', 'any key I have not asked for yet']),
]

class EchoOff:
    def __enter__(self):
        self.fd = sys.stdin.fileno()
        try:
            self.old = termios.tcgetattr(self.fd)
            new = termios.tcgetattr(self.fd)
            new[3] &= ~termios.ECHO          # lflag: only removes the echo
            termios.tcsetattr(self.fd, termios.TCSADRAIN, new)
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

def wait_key(kb, limit):
    """Returns (column, row) of the first f6, or None if the time runs out."""
    t0 = time.time()
    found = None
    while time.time() - t0 < limit:
        try:
            r = bytes(kb.dev.read(K.EP_EVENTS, 8, 200))
        except usb.core.USBError:
            continue
        if r[0] == 0xf6 and r[1] == 0x01:
            found = (r[2], r[3])
            break
    if found:
        end = time.time() + DEBOUNCE      # drains the rest of the same press
        while time.time() < end:
            try:
                kb.dev.read(K.EP_EVENTS, 8, 50)
            except usb.core.USBError:
                pass
    return found

def grid(mapping):
    if not mapping:
        return
    cols = sorted({c for c, _ in mapping})
    rows = sorted({l for _, l in mapping})
    width = max(len(v) for v in mapping.values()) + 1
    print('\n      ' + ''.join(('c%d' % c).rjust(width) for c in cols))
    for l in rows:
        print('  r%-3d' % l + ''.join((mapping.get((c, l), '.')).rjust(width) for c in cols))

def main():
    target = sys.argv[1] if len(sys.argv) > 1 else 'physical-matrix.json'
    mapping, skipped = {}, []
    print(__doc__)
    print('Saving to %s. Starting in 3s...\n' % target)
    time.sleep(3)
    with K.Keyboard() as kb, EchoOff():
        try:
            for title, keys in ROWS:
                print('\n=== %s ===' % title)
                for name in keys:
                    short = name.split(' (')[0]
                    print('  press  %-14s ' % short, end='', flush=True)
                    co = wait_key(kb, WAIT)
                    if co is None:
                        print('-- skipped (no event)')
                        skipped.append(short)
                        continue
                    if co in mapping and mapping[co] != short:
                        print('column %2d row %d   WAS ALREADY %s (?)' % (co[0], co[1], mapping[co]))
                    else:
                        print('column %2d row %d' % co)
                    mapping[co] = short
        except KeyboardInterrupt:
            print('\n\ninterrupted -- saving what we have')
    json.dump({'map': {'%d,%d' % k: v for k, v in mapping.items()}, 'skipped': skipped},
              open(target, 'w'), indent=1, ensure_ascii=False)
    print('\n%d coordinates, %d skipped, in %s' % (len(mapping), len(skipped), target))
    grid(mapping)
    if skipped:
        print('\nskipped: ' + ', '.join(skipped))

main()
