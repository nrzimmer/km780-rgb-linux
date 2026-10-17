#!/usr/bin/env python3
"""Shows the keys that reach X11, without root and without the input group.

Complements `km780key.py`, which reads /dev/input and therefore needs root or
the `input` group. This one needs nothing: the X RECORD extension hands a
passive copy of the events to any client.

This is how `0xC3` and `0xC4` came out of the "[?] hole" and became `Super`+`Tab`
and `Alt`+`Tab` (`docs/17` §4.1): with the key remapped in firmware, what the
host receives is the combination, and here it shows up as both events in the
same millisecond.

It uses the RECORD extension, which hands over a passive copy of the events --
it steals no focus and intercepts nothing. It is for seeing what the keyboard
EMITS when a key is remapped in firmware: the host receives the key
combination, and that is what we want to read.

  listen_x11.py [seconds]
"""
import sys, time
from Xlib import X, display, XK
from Xlib.ext import record
from Xlib.protocol import rq

seconds = float(sys.argv[1]) if len(sys.argv) > 1 else 15.0
d = display.Display()
d2 = display.Display()
t0 = time.time()
seen = []

# keysym -> name, built from Xlib's own XK_* constants: keysym_to_string only
# resolves Latin-1 and returns empty for Ctrl, F13, etc.
NAMES = {v: k[3:] for k, v in vars(XK).items() if k.startswith("XK_")}

def name_of(ks):
    return NAMES.get(ks) or XK.keysym_to_string(ks) or f"keysym 0x{ks:x}"

def handle(reply):
    if reply.category != record.FromServer or reply.client_swapped:
        return
    data = reply.data
    while len(data):
        ev, data = rq.EventField(None).parse_binary_value(data, d2.display, None, None)
        if ev.type in (X.KeyPress, X.KeyRelease):
            kind = "press  " if ev.type == X.KeyPress else "release"
            ks = d2.keycode_to_keysym(ev.detail, 0)
            n = name_of(ks)
            line = f"  t={time.time()-t0:5.2f}  {kind}  keycode {ev.detail:3d}  {n}"
            print(line, flush=True)
            seen.append((ev.type, ev.detail, n))

ctx = d.record_create_context(
    0, [record.AllClients],
    [{'core_requests': (0, 0), 'core_replies': (0, 0),
      'ext_requests': (0, 0, 0, 0), 'ext_replies': (0, 0, 0, 0),
      'delivered_events': (0, 0), 'device_events': (X.KeyPress, X.KeyRelease),
      'errors': (0, 0), 'client_started': False, 'client_died': False}])

import threading
def stop():
    time.sleep(seconds)
    d.record_disable_context(ctx)
    d.flush()
threading.Thread(target=stop, daemon=True).start()

print(f"listening {seconds:.0f}s...", flush=True)
d.record_enable_context(ctx, handle)
d.record_free_context(ctx)
print(f"end: {len(seen)} events")
