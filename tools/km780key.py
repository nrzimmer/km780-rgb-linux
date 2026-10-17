#!/usr/bin/env python3
"""Shows what Linux receives when a KM780 key is pressed.

Opens every event node of the keyboard and decodes them. Answers the question
"does the kernel see this key?" without depending on 0x16 or the vendor
session: it reads /dev/input/event*, the normal HID path.

  km780key.py [seconds]        default 20

Needs root OR the `input` group: the nodes are `root:input rw-rw----` and have
no uaccess ACL (unlike /dev/bus/usb, which the rest of our tooling uses without
root).

What each line means:

  EV_KEY   the kernel has a keycode for the key -> it can be bound in i3,
           xbindkeys, keyd, with no process running
  MSC_SCAN raw scancode; if MSC_SCAN arrives without EV_KEY, the key reaches
           the kernel but has no keycode -> it can be mapped with hwdb/keyd,
           still with no service
  nothing  the key is internal to the keyboard and never leaves over HID ->
           only through the vendor channel, which requires 0x16 open and
           therefore requires a service
"""
import os, sys, struct, select, glob

FMT = 'llHHi'
SZ = struct.calcsize(FMT)
EV_SYN, EV_KEY, EV_MSC, EV_LED, EV_REP = 0x00, 0x01, 0x04, 0x11, 0x14
MSC_SCAN = 0x04

# Keycode names that matter; the rest comes out as a number.
TYPES = {EV_SYN: 'EV_SYN', EV_KEY: 'EV_KEY', EV_MSC: 'EV_MSC',
         EV_LED: 'EV_LED', EV_REP: 'EV_REP'}


def nodes():
    """The event nodes whose name is the KM780's, with the suffix that tells them apart."""
    found = []
    for p in sorted(glob.glob('/sys/class/input/event*/device/name')):
        try:
            name = open(p).read().strip()
        except OSError:
            continue
        if 'KM780' not in name.upper():
            continue
        ev = p.split('/')[4]
        # the name comes duplicated from the descriptor; what matters is the suffix
        suffix = name.split('GAMING KEYBOARD')[-1].strip() or 'main'
        found.append(('/dev/input/' + ev, suffix))
    return found


def main():
    secs = int(sys.argv[1]) if len(sys.argv) > 1 else 20
    targets = nodes()
    if not targets:
        sys.exit('no KM780 event node found; is the keyboard plugged in?')

    fds = {}
    for path, suffix in targets:
        try:
            fds[os.open(path, os.O_RDONLY | os.O_NONBLOCK)] = (path, suffix)
        except PermissionError:
            sys.exit('no permission on %s.\n'
                     'run with sudo, or join the input group '
                     '(`usermod -aG input %s`, requires logging back in).'
                     % (path, os.environ.get('USER', 'you')))

    print('listening on %d node(s) for %ds:' % (len(fds), secs))
    for path, suffix in targets:
        print('  %-20s %s' % (path, suffix))
    print('\npress the key you want to investigate. Ctrl+C to quit.\n')

    import time
    t0 = time.time()
    seen = 0
    try:
        while time.time() - t0 < secs:
            r, _, _ = select.select(list(fds), [], [], 0.4)
            for fd in r:
                path, suffix = fds[fd]
                try:
                    data = os.read(fd, SZ * 64)
                except BlockingIOError:
                    continue
                for i in range(0, len(data) - SZ + 1, SZ):
                    _, _, kind, code, val = struct.unpack(FMT, data[i:i + SZ])
                    if kind == EV_SYN:
                        continue
                    seen += 1
                    name = TYPES.get(kind, 'type 0x%02x' % kind)
                    if kind == EV_MSC and code == MSC_SCAN:
                        print('  %-12s MSC_SCAN  scancode 0x%x  (%s)'
                              % (name, val, suffix))
                    elif kind == EV_KEY:
                        print('  %-12s code %-4d %s  (%s)'
                              % (name, code,
                                 {0: 'release', 1: 'press', 2: 'repeat'}.get(val, val),
                                 suffix))
                    else:
                        print('  %-12s code %-4d val %-4d  (%s)'
                              % (name, code, val, suffix))
    except KeyboardInterrupt:
        print('\n(interrupted)')
    finally:
        for fd in fds:
            os.close(fd)
    print('\n%d event(s). If the key did not show up, it does not leave over HID: '
          'only through the vendor channel, which requires 0x16 and therefore a service.'
          % seen)


if __name__ == '__main__':
    main()
