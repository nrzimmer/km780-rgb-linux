#!/usr/bin/env python3
"""Experiment log for closing the gaps in the mode payload.

The gaps left in 0x02 (byte 108 vs 119, bytes 72-107) do not come out of a
capture: a capture only shows what the official UI does. They come out of here
-- read the mode, change ONE byte, write, and look at the keyboard.

  km780exp.py events [sec]         listen and decode EP 0x83        (read only)
  km780exp.py map                  show the mode payload annotated  (read only)
  km780exp.py watch [sec]          re-read in a loop; says which byte
                                   changed when you touch the keyboard (read only)
  km780exp.py byte <N> <value>     change one byte and confirm      WRITES
  km780exp.py sweep <start>-<end>  walk a range, one byte at a
                                   time, asking what changed        WRITES

Options: --mode N        act on mode N instead of the active one
         --backup <file> use this file (default: backup-exp.json)
         --yes           confirm the writes without asking

Every write makes a backup first, and `restore` puts it back.
  km780exp.py restore              rewrite the backup               WRITES

What this path does NOT do: send an unknown command. It only uses 0x81, 0x82
and 0x02, all confirmed. See the README warning.
"""
import sys, os, json, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import km780lib as K

NOTES = {
    3: 'status (0 write / 3 answer)', 8: 'polling rate, ms',
    10: 'active effect', 11: 'brightness 0-4', 12: 'direction', 13: 'direction',
    14: 'direction', 108: 'toggle A -- all lighting OR volume display?',
    119: 'toggle B -- the other of the two', 75: 'no. of color stops',
    76: 'no. of color stops', 77: 'no. of color stops',
}
for _i in range(15, 72):
    NOTES.setdefault(_i, '4-byte record; the 4th is duration' if _i % 4 == 3 else '')
for _i in range(120, 264):
    NOTES[_i] = 'key %s' % (K.SLOTS[_i - 120] if _i - 120 < len(K.SLOTS) else '?')


def decode(r):
    """The three formats of EP 0x83 (docs/22 §2)."""
    if r[0] == 0x00:
        extra = '  mode=%d' % r[4] if r[4] else ''
        return 'ACK   cmd=0x%02x%s' % (r[2], extra)
    if r[0] == 0xf6:
        return 'KEY   column=%2d row=%d' % (r[2], r[3])
    if r[0] == 0x02 and r[1] == 0x04:
        kind = {0x08: 'mode/presence', 0x09: 'BRIGHTNESS level=%d' % r[4],
                0x04: 'unknown'}.get(r[3], 'type 0x%02x' % r[3])
        return 'STATE mode=%d  %s' % (r[2], kind)
    return 'UNKNOWN'


def cmd_events(kb, a):
    sec = int(a['rest'][0]) if a['rest'] else 30
    print('listening on EP 0x83 for %ds -- press keys, change the brightness, '
          'switch modes M1/M2/M3\n' % sec)
    seen = {}
    def show(t, r):
        h = r.hex(' ')
        seen[h] = seen.get(h, 0) + 1
        print('  %6.2fs  %s   %s' % (t, h, decode(r)))
    kb.events(sec, show)
    print('\n%d distinct messages:' % len(seen))
    for h, n in sorted(seen.items(), key=lambda x: -x[1]):
        print('  %5d x  %s   %s' % (n, h, decode(bytes.fromhex(h))))


def cmd_map(kb, a):
    m = a['mode'] or kb.active_mode()
    d = kb.read_mode(m)
    print('mode %d, 264 bytes. "." = zero.\n' % m)
    for off in range(0, 264, 16):
        line = ' '.join('%02x' % v if v else ' .' for v in d[off:off + 16])
        notes = {i: NOTES[i] for i in range(off, min(off + 16, 264))
                 if NOTES.get(i) and i < 120}
        print('  %3d: %s' % (off, line))
        for i, t in notes.items():
            print('        byte %3d = %-3d  %s' % (i, d[i], t))


def cmd_watch(kb, a):
    m = a['mode'] or kb.active_mode()
    sec = int(a['rest'][0]) if a['rest'] else 60
    base = kb.read_mode(m)
    print('watching mode %d for %ds. Touch the keyboard (MR/M1/M2/M3 keys,\n'
          'brightness, anything) -- I will say which byte changed.\n' % (m, sec))
    t0 = time.time()
    while time.time() - t0 < sec:
        time.sleep(1.0)
        new = kb.read_mode(m)
        diff = [i for i in range(K.PKT) if base[i] != new[i] and i != K.OFF_STATUS]
        if diff:
            for i in diff:
                print('  %6.1fs  byte %3d: %d -> %d   %s'
                      % (time.time() - t0, i, base[i], new[i], NOTES.get(i, '')))
            base = new
    print('done.')


def _backup(kb, a):
    path = a['backup']
    if os.path.exists(path):
        return path
    out = {'modes': {str(m): list(kb.read_mode(m)) for m in (1, 2, 3)},
           'colors': {str(m): [list(c) for c in kb.read_colors(m)] for m in (1, 2, 3)}}
    json.dump(out, open(path, 'w'))
    print('backup created in %s' % path)
    return path


def cmd_restore(kb, a):
    b = json.load(open(a['backup']))
    for m in (1, 2, 3):
        kb.write_mode(m, bytes(b['modes'][str(m)]))
        kb.write_colors(m, [bytearray(c) for c in b['colors'][str(m)]])
    print('the 3 modes were rewritten from %s' % a['backup'])


def _write_byte(kb, m, off, value, a):
    d = kb.read_mode(m)
    before = d[off]
    if before == value:
        print('  byte %d is already %d' % (off, value))
        return before
    if not a['yes']:
        r = input('  byte %d: %d -> %d.  write? [y/N] ' % (off, before, value))
        if r.strip().lower() not in ('y', 'yes'):
            print('  skipped'); return None
    d[off] = value
    kb.write_mode(m, bytes(d))
    conf = kb.read_mode(m)[off]
    if conf != value:
        print('  WARNING: re-read returned %d, not %d -- the firmware refused '
              'or normalized it' % (conf, value))
    return before


def cmd_byte(kb, a):
    m = a['mode'] or kb.active_mode()
    off, value = int(a['rest'][0]), int(a['rest'][1])
    if not 0 <= off < K.PKT:
        sys.exit('offset out of 0..263')
    _backup(kb, a)
    print('mode %d, byte %d  (%s)' % (m, off, NOTES.get(off, 'no note')))
    _write_byte(kb, m, off, value, a)
    print('\nlook at the keyboard. `km780exp.py restore` puts it back.')


def cmd_sweep(kb, a):
    m = a['mode'] or kb.active_mode()
    span = a['rest'][0]
    start, end = (int(x) for x in span.split('-'))
    value = int(a['rest'][1]) if len(a['rest']) > 1 else 1
    _backup(kb, a)
    print('sweeping bytes %d to %d of mode %d, putting %d in each one, one at a\n'
          'time, and restoring the old value before moving to the next.\n'
          'Enter = nothing changed; any text = what you saw.\n' % (start, end, m, value))
    found = {}
    for off in range(start, end + 1):
        note = NOTES.get(off, '')
        print('byte %3d %s' % (off, ('(%s)' % note) if note else ''))
        before = _write_byte(kb, m, off, value, a)
        if before is None:
            continue
        obs = input('  what changed on the keyboard? ').strip()
        d = kb.read_mode(m); d[off] = before
        kb.write_mode(m, bytes(d))
        if obs:
            found[off] = obs
            print('  noted.')
    print('\n--- findings ---')
    for off, obs in found.items():
        print('  byte %3d: %s' % (off, obs))
    if not found:
        print('  no byte in the range had a visible effect.')


COMMANDS = {'events': (cmd_events, 0), 'map': (cmd_map, 0), 'watch': (cmd_watch, 0),
            'byte': (cmd_byte, 2), 'sweep': (cmd_sweep, 1), 'restore': (cmd_restore, 0)}


def main():
    av = sys.argv[1:]
    if not av or av[0] in ('-h', '--help'):
        print(__doc__); return
    a = {'mode': None, 'yes': '--yes' in av, 'backup': 'backup-exp.json', 'rest': []}
    skip = set()
    for flag, key, conv in (('--mode', 'mode', int), ('--backup', 'backup', str)):
        if flag in av:
            i = av.index(flag); a[key] = conv(av[i + 1]); skip |= {i, i + 1}
    cmd = None
    for i, x in enumerate(av):
        if i in skip or x == '--yes':
            continue
        if cmd is None:
            cmd = x
        else:
            a['rest'].append(x)
    if cmd not in COMMANDS:
        sys.exit('unknown command: %r\n\n%s' % (cmd, __doc__))
    fn, minimum = COMMANDS[cmd]
    if len(a['rest']) < minimum:
        sys.exit('%r needs %d argument(s)' % (cmd, minimum))
    with K.Keyboard() as kb:
        fn(kb, a)


main()
