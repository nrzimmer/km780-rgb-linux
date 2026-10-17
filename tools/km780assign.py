#!/usr/bin/env python3
"""Reassigns a key's function, in the keyboard's own memory.

Bytes 120-263 of the mode payload are the assignment table: 1 byte per LED
slot, holding that key's function code (docs/09 §5.1 and §5.2). The table
lives in the keyboard, so a key remapped here **stays remapped with no process
running** -- which is why this solves "make a key useful without a service".

  km780assign.py show [KEY...]             read only
  km780assign.py set KEY=CODE [...]        WRITES
  km780assign.py restore KEY [...]         back to the previous value

CODE accepts a name (f13..f24) or raw hex (e.g. 68). The byte ranges, from
docs/09 §5.2:

  00         no function       b4         macro
  04-e7      HID usage         b8         text
  a5-ab      multimedia        c0-c8      Windows shortcut
                               ce-d5      mouse function

F13..F24 (0x68-0x73) are the good choice for a "free key": they exist in HID,
Linux maps them to KEY_F13..KEY_F24, and no software uses them out of the box.

Uses read-change-write on 0x82/0x02, like the rest. Before writing, it saves
the previous value in ~/.config/km780/assignments.json; `restore` goes back to
exactly that value. With no saved record it **refuses** instead of guessing --
the factory table is in docs/10, and guessing a function code is worse than
doing nothing.
"""
import sys, os, json
sys.path.insert(0, __file__.rsplit('/', 1)[0])
import km780lib as K

OFF_FUNCTIONS = 120
STORE = os.path.expanduser('~/.config/km780/assignments.json')

NAMES = {}
for i in range(12):                       # f13..f24 -> 0x68..0x73
    NAMES['f%d' % (13 + i)] = 0x68 + i


def code_of(txt):
    t = txt.strip().lower()
    if t in NAMES:
        return NAMES[t]
    try:
        v = int(t, 16)
    except ValueError:
        raise SystemExit('code "%s" not understood: use f13..f24 or hex '
                         '(e.g. 68)' % txt)
    if not 0 <= v <= 255:
        raise SystemExit('code 0x%x out of 0-255' % v)
    return v


def store_read():
    try:
        return json.load(open(STORE))
    except Exception:
        return {}


def store_write(d):
    os.makedirs(os.path.dirname(STORE), exist_ok=True)
    json.dump(d, open(STORE, 'w'), indent=1)


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd = sys.argv[1]
    args = sys.argv[2:]
    modes = (1, 2, 3)
    if '--mode' in args:
        i = args.index('--mode')
        modes = (int(args[i + 1]),)
        del args[i:i + 2]

    with K.Keyboard() as kb:
        if cmd == 'show':
            g = store_read()
            if args:
                targets = [(K.slot_of(a), a) for a in args]
            else:
                # no list: only what this tool has already touched
                targets = sorted({(int(k.split('/')[1]),
                                   K.SLOTS[int(k.split('/')[1])]) for k in g})
                if not targets:
                    print('no assignment recorded by this tool.')
                    print('pass a key name to look at one, e.g.: show Timer')
                    return
            for m in modes:
                d = kb.read_mode(m)
                print('--- mode %d' % m)
                for slot, name in targets:
                    v = d[OFF_FUNCTIONS + slot]
                    prev = g.get('%d/%d' % (m, slot))
                    print('  slot %3d  %-10s byte %3d = 0x%02x%s'
                          % (slot, name, OFF_FUNCTIONS + slot, v,
                             '   (previous 0x%02x)' % prev if prev is not None else ''))
            return

        if cmd not in ('set', 'restore'):
            sys.exit('unknown subcommand: %s' % cmd)
        if not args:
            sys.exit('%s needs at least one key' % cmd)

        requests = []
        for a in args:
            if cmd == 'set':
                if '=' not in a:
                    sys.exit('use KEY=CODE, not "%s"' % a)
                t, c = a.split('=', 1)
                requests.append((K.slot_of(t), t, code_of(c)))
            else:
                requests.append((K.slot_of(a), a, None))

        g = store_read()
        for m in modes:
            d = bytearray(kb.read_mode(m))
            for slot, name, code in requests:
                off = OFF_FUNCTIONS + slot
                before = d[off]
                if code is None:
                    key = '%d/%d' % (m, slot)
                    if key not in g:
                        sys.exit('no saved value for %s in mode %d. '
                                 'The factory code is in docs/10 -- '
                                 'pass it with set %s=<hex>.' % (name, m, name))
                    code = g[key]
                d[off] = code
                # DO NOT overwrite an original that is already saved: reapplying
                # the same remap would record the new value as "previous" and
                # destroy the undo.
                g.setdefault('%d/%d' % (m, slot), before)
                print('  mode %d  %-10s byte %3d: 0x%02x -> 0x%02x'
                      % (m, name, off, before, code))
            kb.write_mode(m, bytes(d))
            back = kb.read_mode(m)
            for slot, name, _ in requests:
                off = OFF_FUNCTIONS + slot
                if back[off] != d[off]:
                    print('     WARNING: re-read returned 0x%02x, it did not take'
                          % back[off])
        store_write(g)
        print('\nprevious values saved in %s' % STORE)
        print('to undo:  km780assign.py restore %s'
              % ' '.join(n for _, n, _ in requests))


if __name__ == '__main__':
    main()
