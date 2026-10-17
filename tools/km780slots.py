#!/usr/bin/env python3
"""Finds out which LED slot each special key is, by painting and looking.

    tools/km780slots.py

The 19 synthetic slots are grouped by HID usage (docs/10), but which slot is
which key WITHIN the group was never determined. This lights one group at a
time, each slot in a different color, and you read the legend on the keyboard.

Four rounds, one per group. Each one waits for Enter.

WRITES -- only 0x09, the color path confirmed on hardware. It backs the colors
up first and restores them in a finally, which also runs on an exception and on
Ctrl+C. Checks byte for byte at the end.
"""
import sys, os
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K

# well-separated colors, easy to name
PALETTE = [('RED', (255, 0, 0)), ('GREEN', (0, 255, 0)), ('BLUE', (0, 0, 255)),
           ('YELLOW', (255, 255, 0)), ('MAGENTA', (255, 0, 255)), ('CYAN', (0, 255, 255))]

GROUPS = [
    ('the 6 G keys', [0, 1, 2, 3, 4, 5]),
    ('MR / M1 / M2 / M3', [6, 7, 8, 24]),
    ('brightness / timer / bag', [42, 43, 44]),
    ('the 5 multimedia ones', [105, 106, 107, 115, 116]),
]


def main():
    print(__doc__)
    with K.Keyboard() as kb:
        m = kb.active_mode()
        d = kb.read_mode(m)
        if d[K.OFF_EFFECT] != 0:
            sys.exit('mode %d has effect %d active; the animation overwrites the\n'
                     'matrix. Switch to a mode with no effect (2 is static).'
                     % (m, d[K.OFF_EFFECT]))
        if d[K.OFF_BRIGHTNESS] == 0:
            sys.exit('brightness is 0; raise it first, otherwise nothing shows.')
        print('mode %d, effect 0, brightness %d. Saving the current colors...'
              % (m, d[K.OFF_BRIGHTNESS]))
        orig = kb.read_colors(m)
        try:
            for title, slots in GROUPS:
                channels = [bytearray(K.NSLOTS) for _ in range(3)]
                print('\n=== %s ===' % title)
                for i, s in enumerate(slots):
                    name, (r, g, b) = PALETTE[i]
                    channels[0][s], channels[1][s], channels[2][s] = r, g, b
                    print('   slot %3d  ->  %s' % (s, name))
                kb.write_colors(m, channels)
                print('   (the rest of the keyboard stays dark)')
                try:
                    input('   look at the keyboard and write it down. [Enter] for the next ')
                except EOFError:
                    print()
        finally:
            print('\nrestoring the colors...')
            kb.write_colors(m, orig)
            after = kb.read_colors(m)
            diff = [(c, i) for c in range(3) for i in range(K.NSLOTS)
                    if orig[c][i] != after[c][i]]
            print('checked: the colors of mode %d came back identical' % m if not diff
                  else 'WARNING: %d slot(s) differ: %s' % (len(diff), diff[:10]))


main()
