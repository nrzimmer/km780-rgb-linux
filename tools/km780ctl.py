#!/usr/bin/env python3
"""Lighting control for the G.SKILL Ripjaws KM780 RGB on Linux.

  km780ctl.py info                    current state, without writing
  km780ctl.py colors [--all]          read the color of every key (0x89)
  km780ctl.py backup <file.json>      save colors + the 3 modes
  km780ctl.py restore <file.json>     write a backup back        WRITES

  km780ctl.py set F5=ff0000 A=blue    paint keys, keep the rest   WRITES
  km780ctl.py all <color>             paint everything            WRITES
  km780ctl.py off                     everything black            WRITES
  km780ctl.py brightness <0..4>       intensity                   WRITES
  km780ctl.py effect <id|list>        animated effect (0 = none)  WRITES

Options: --mode N  act on mode N instead of the active one.
         -n        show what it would do and write nothing.

Colors: RRGGBB, RGB, or a name (red, blue, cyan, ...).
Keys: the name from docs/10-key-map.md (F5, A, Space, Num7) or the slot.

Color writes use only 0x09, and brightness/effect writes do a
read-change-one-byte-write on 0x02, which is the safe path: the still-unknown
fields of the mode travel intact through the round-trip (docs/09 §5.3).
"""
import sys, json
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import km780lib as K

# 6 is not in the official UI and the DLL has no branch to build a profile for
# it, but firmware 77 implements it: dark at rest, a pressed key lights up.
# Confirmed on hardware -- docs/27 §10. 8 (Checkpoint) is reactive too.
# 3 (Sonic) is internally called Reactive, profile .rea -- docs/27 §7.
EFFECTS = {0: 'none (static)', 1: 'Wave', 2: 'Ripple', 3: 'Sonic',
           4: 'Breathing', 6: 'key lights up and stays lit (outside the UI; useless)',
           7: 'Changeling', 8: 'Checkpoint (reactive)', 9: 'Cycle',
           10: 'Rain'}
POLLING = {1: '1000 Hz', 2: '500 Hz', 4: '250 Hz', 8: '125 Hz'}


def colors_per_slot(channels):
    return [(channels[0][i], channels[1][i], channels[2][i]) for i in range(K.NSLOTS)]


def cmd_info(kb, args):
    active = kb.active_mode()
    print('active mode: %d' % active)
    print('%-6s %-10s %-22s %-10s' % ('mode', 'brightness', 'effect', 'polling'))
    for m in (1, 2, 3):
        d = kb.read_mode(m)
        ef = d[K.OFF_EFFECT]
        print('%-6s %-10s %-22s %-10s%s' % (
            m, d[K.OFF_BRIGHTNESS], '%d  %s' % (ef, EFFECTS.get(ef, '?')),
            POLLING.get(d[K.OFF_POLLING], '%d ms' % d[K.OFF_POLLING]),
            '   <- active' if m == active else ''))


def cmd_colors(kb, args):
    m = args['mode'] or kb.active_mode()
    colors = colors_per_slot(kb.read_colors(m))
    print('mode %d, %d slots (those with no physical key are hidden; --all shows them)\n' % (
        m, K.NSLOTS))
    n = 0
    for i, (r, g, b) in enumerate(colors):
        if i >= len(K.SLOTS):
            if not args['all']:
                continue
            name = '(slot %d)' % i
        else:
            name = K.SLOTS[i]
            if i in K.NO_KEY and not args['all']:
                continue
        print('  %-12s %3d  %02x%02x%02x' % (name, i, r, g, b), end='')
        n += 1
        print('' if n % 4 == 0 else '   ', end='')
    print()


def cmd_backup(kb, args):
    target = args['rest'][0]
    out = {'colors': {}, 'modes': {}}
    for m in (1, 2, 3):
        out['colors'][str(m)] = [list(c) for c in kb.read_colors(m)]
        out['modes'][str(m)] = list(kb.read_mode(m))
    out['active'] = kb.active_mode()
    json.dump(out, open(target, 'w'))
    print('backup of the 3 modes (color + mode payload) in %s' % target)


def cmd_restore(kb, args):
    b = json.load(open(args['rest'][0]))
    for m in (1, 2, 3):
        if args['dry']:
            print('  [dry] would rewrite mode %d' % m)
            continue
        kb.write_mode(m, bytes(b['modes'][str(m)]))
        kb.write_colors(m, [bytearray(c) for c in b['colors'][str(m)]])
        print('  mode %d rewritten' % m)


def _paint(kb, args, change):
    """Read the colors, apply change(list_of_colors), write them back."""
    m = args['mode'] or kb.active_mode()
    channels = kb.read_colors(m)
    colors = colors_per_slot(channels)
    change(colors)
    for c in range(3):
        for i in range(K.NSLOTS):
            channels[c][i] = colors[i][c]
    if args['dry']:
        print('[dry] nothing was written')
        return
    kb.write_colors(m, channels)
    print('mode %d updated' % m)


def cmd_set(kb, args):
    pairs = []
    for a in args['rest']:
        if '=' not in a:
            sys.exit('expected KEY=COLOR, got %r' % a)
        t, c = a.split('=', 1)
        pairs.append((K.slot_of(t), K.color_of(c)))
    for s, c in pairs:
        print('  %-12s slot %3d  -> %02x%02x%02x' % (K.SLOTS[s] if s < len(K.SLOTS) else '?', s, *c))

    def change(colors):
        for s, c in pairs:
            colors[s] = c
    _paint(kb, args, change)


def cmd_all(kb, args):
    c = K.color_of(args['rest'][0])
    print('  all %d slots -> %02x%02x%02x' % (K.NSLOTS, *c))
    _paint(kb, args, lambda colors: colors.__setitem__(slice(None), [c] * K.NSLOTS))


def cmd_off(kb, args):
    args['rest'] = ['000000']
    cmd_all(kb, args)


def _change_byte(kb, args, off, value, label):
    m = args['mode'] or kb.active_mode()
    d = kb.read_mode(m)
    before = d[off]
    print('  mode %d, byte %d (%s): %d -> %d' % (m, off, label, before, value))
    if before == value:
        print('  already at that value, nothing to do')
        return
    d[off] = value
    if args['dry']:
        print('[dry] nothing was written')
        return
    kb.write_mode(m, bytes(d))
    conf = kb.read_mode(m)[off]
    print('  confirmed on re-read: %d' % conf if conf == value
          else '  WARNING: re-read returned %d, not %d' % (conf, value))


def cmd_brightness(kb, args):
    v = int(args['rest'][0])
    if not 0 <= v <= 4:
        sys.exit('brightness goes from 0 to 4 (docs/09 §6.6)')
    _change_byte(kb, args, K.OFF_BRIGHTNESS, v, 'brightness')


def cmd_effect(kb, args):
    a = args['rest'][0]
    if a == 'list':
        for k, v in sorted(EFFECTS.items()):
            print('  %2d  %s' % (k, v))
        return
    v = int(a)
    if v not in EFFECTS:
        sys.exit('effect %d does not exist in the official UI; known: %s'
                 % (v, ' '.join(str(x) for x in sorted(EFFECTS))))
    _change_byte(kb, args, K.OFF_EFFECT, v, 'effect')


COMMANDS = {'info': (cmd_info, 0), 'colors': (cmd_colors, 0),
            'backup': (cmd_backup, 1), 'restore': (cmd_restore, 1),
            'set': (cmd_set, 1), 'all': (cmd_all, 1), 'off': (cmd_off, 0),
            'brightness': (cmd_brightness, 1), 'effect': (cmd_effect, 1)}


def main():
    av = sys.argv[1:]
    if not av or av[0] in ('-h', '--help'):
        print(__doc__); return
    args = {'mode': None, 'dry': '-n' in av, 'all': '--all' in av, 'rest': []}
    if '--mode' in av:
        args['mode'] = int(av[av.index('--mode') + 1])
    skip = set()
    if '--mode' in av:
        i = av.index('--mode'); skip |= {i, i + 1}
    cmd = None
    for i, a in enumerate(av):
        if i in skip or a in ('-n', '--all'):
            continue
        if cmd is None:
            cmd = a
        else:
            args['rest'].append(a)
    if cmd not in COMMANDS:
        sys.exit('unknown command: %r\n\n%s' % (cmd, __doc__))
    fn, minimum = COMMANDS[cmd]
    if len(args['rest']) < minimum:
        sys.exit('%r needs at least %d argument(s)' % (cmd, minimum))
    with K.Keyboard() as kb:
        fn(kb, args)


main()
