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

# Order of asking: by physical row, so the user does not get lost.
# The 19 "synthetic" keys (G, media, MR/M1/M2/M3, brightness, timer and the bag
# one) come last, because they are the ones that may emit an event of a type
# other than f6 -- see docs/10 and the HID usage analysis.
ROWS = [
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
                        'NumEnter', 'Num0', 'Num.']),
    ('G keys (left column)', ['G1', 'G2', 'G3', 'G4', 'G5', 'G6']),
    ('MR and the modes -- CAREFUL: these really do switch the mode',
     ['MR (record macro)', 'M1', 'M2', 'M3']),
    ('multimedia (5 keys)', ['media 1 (leftmost)', 'media 2', 'media 3',
                             'media 4', 'media 5 (rightmost)']),
    ('the other three', ['brightness', 'timer', 'the bag one (symbol nobody knows)']),
    ('the volume wheel', ['roll the volume UP', 'roll the volume DOWN']),
    ('any left over?', ['any key I did not ask for', 'another one I did not ask for']),
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
    """Returns (kind, detail, bytes) of the first event, or None on timeout.

    Accepts ANY 8-byte event, not only f6: the MR/M1/M2/M3 keys switch the mode
    and emit `02 04 <mode> 08 01`, and the brightness one emits
    `02 04 <mode> 09 <level>` (docs/22 §2). Filtering by f6 would lose exactly
    the most interesting keys.
    """
    t0 = time.time()
    found = None
    while time.time() - t0 < limit:
        try:
            r = bytes(kb.dev.read(K.EP_EVENTS, 8, 200))
        except usb.core.USBError:
            continue
        if r[0] == 0xf6 and r[1] == 0x01:
            found = ('matrix', (r[2], r[3]), r)
        elif r[0] == 0x02 and r[1] == 0x04:
            kind = {0x08: 'mode', 0x09: 'brightness'}.get(r[3], 'state%02x' % r[3])
            found = (kind, (r[2], r[4]), r)
        elif r[0] == 0x00:
            continue                      # ACK of a command of ours, ignore
        else:
            found = ('other', None, r)
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
    mapping, skipped, log = {}, [], []
    print(__doc__)
    print('Saving to %s. Starting in 3s...\n' % target)
    time.sleep(3)
    with K.Keyboard() as kb, EchoOff():
        try:
            for title, keys in ROWS:
                print('\n=== %s ===' % title)
                for name in keys:
                    short = name.split(' (')[0]
                    print('  press  %-16s ' % short, end='', flush=True)
                    ev = wait_key(kb, WAIT)
                    if ev is None:
                        print('-- skipped (no event)')
                        skipped.append(short)
                        continue
                    kind, det, raw = ev
                    log.append({'key': short, 'kind': kind,
                                'detail': list(det) if det else None,
                                'raw': raw.hex(' ')})
                    if kind == 'matrix':
                        if det in mapping and mapping[det] != short:
                            print('column %2d row %d   WAS ALREADY %s (?)' % (det[0], det[1], mapping[det]))
                        else:
                            print('column %2d row %d' % det)
                        mapping[det] = short
                    elif kind == 'mode':
                        print('NOT f6 -- MODE event, mode=%d   [%s]' % (det[0], raw.hex(' ')))
                    elif kind == 'brightness':
                        print('NOT f6 -- BRIGHTNESS event, level=%d   [%s]' % (det[1], raw.hex(' ')))
                    else:
                        print('event %s   [%s]' % (kind, raw.hex(' ')))
        except KeyboardInterrupt:
            print('\n\ninterrupted -- saving what we have')
    json.dump({'map': {'%d,%d' % k: v for k, v in mapping.items()},
               'skipped': skipped, 'events': log},
              open(target, 'w'), indent=1, ensure_ascii=False)
    notf6 = [r for r in log if r['kind'] != 'matrix']
    print('\n%d matrix coordinates, %d events of another kind, %d skipped -- in %s'
          % (len(mapping), len(notf6), len(skipped), target))
    for r in notf6:
        print('  %-18s %-8s %s' % (r['key'], r['kind'], r['raw']))
    grid(mapping)
    if skipped:
        print('\nskipped: ' + ', '.join(skipped))

main()
