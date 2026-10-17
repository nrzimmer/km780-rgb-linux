#!/usr/bin/env python3
"""Transport and structures for the G.SKILL Ripjaws KM780 RGB (28da:1101).

Module shared by the tools. Everything here follows docs/09-protocol.md;
no field is guessed.

  packet      264 bytes, HID Feature report ID 7, interface 1
              buf[0]=0x07  buf[1]=command  buf[2]=mode (1-based)
              buf[3..5]    depend on the command
              buf[8..]     256-byte payload

MIND buf[3] vs buf[4]:
  0x09/0x89 put the color channel in **buf[3]**.
  0x0B/0x12/0x15 put the channel in **buf[4]**, and 0x0B uses buf[5] to
  tell Ripple from Sonic. Do not generalize one from the other -- see
  docs/22 §0.1.

Only confirmed commands are emitted. See the README warning about not probing
bytes with bit 7 set that have never been observed.
"""
import sys, time
import usb.core, usb.util

VID, PID = 0x28DA, 0x1101
IFACE_OUT, IFACE_IN = 1, 2
EP_EVENTS = 0x83
PKT = 0x108                 # 264
HDR = 8
NSLOTS = 144                # slots in the payload; 132 used
WVALUE, WINDEX = 0x0307, 0x0001
REPORT_ID = 0x07
SET_REPORT, GET_REPORT = 0x09, 0x01
TIMEOUT = 1000

REQ_OUT = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_OUT
REQ_IN = usb.util.CTRL_TYPE_CLASS | usb.util.CTRL_RECIPIENT_INTERFACE | usb.util.CTRL_IN

# --- commands (docs/09 §3) ---------------------------------------------
MODE_GET, MODE_SET = 0x81, 0x01          # active mode
INFO_GET, INFO_SET = 0x82, 0x02          # mode payload, 264 bytes
LED_GET, LED_SET = 0x89, 0x09            # LED matrix, 1 packet per channel

# offsets inside the mode payload (0x02/0x82), absolute within the packet
OFF_STATUS = 3      # 0x00 on write, 0x03 in the answer
OFF_POLLING = 8     # bInterval in ms: 1/2/4/8 = 1000/500/250/125 Hz
OFF_EFFECT = 10     # 0 = static; see docs/09 §7.1
OFF_BRIGHTNESS = 11 # 0..4
OFF_DIRECTION = 12  # 3 equal bytes: 0=Up 1=Down 2=Left 3=Right
OFF_KEYS = 120      # 144 bytes, function code per slot

CHANNELS = (1, 2, 3)  # 1=R 2=G 3=B, tested on hardware

# Generated from docs/10-key-map.md -- do not edit by hand.
# index = slot in the 144-byte payload of 0x09; value = key name.
SLOTS = [
    'Z801',            'Z802',            'Z803',            'Z804',
    'Z805',            'Z806',            'Z807',            'Z808',
    'Z809',            'Esc',             '`',               'Tab',
    'CapsLock',        'LShift',          'LCtrl',           '1',
    '\\|',             'LWin',            'F1',              '2',
    'Q',               'A',               'Z',               'LAlt',
    'Z80a',            'W',               'X',               'F2',
    '3',               'E',               'S',               'C',
    'VK25',            'F3',              'D',               'INTL8b',
    'F4',              '4',               'R',               'F',
    'V',               '5',               'Z80b',            'Z80c',
    'Z80d',            'F5',              '6',               'T',
    'G',               'B',               'Space',           '7',
    'Y',               'H',               'F6',              '8',
    'U',               'J',               'N',               'F7',
    'I',               'K',               'M',               'F8',
    '9',               'O',               'L',               ',',
    'VK61',            'F9',              '0',               'VK28',
    '[',               '-',               'P',               ';',
    '.',               'RAlt',            '/',               'RWin',
    'INTL88',          'F11',             '=',               'F10',
    "'",               'Z813',            'Menu',            ']',
    'VK22',            'RCtrl',           'F12',             'INTL89',
    '\\',              'Enter',           'RShift',          'Left',
    'Backspace',       'Up',              'Down',            'PrtSc',
    'Insert',          'Delete',          'ScrLk',           'Home',
    'End',             'Z80e',            'Z80f',            'Z810',
    'Pause',           'PgUp',            'PgDn',            'Num4',
    'Num1',            'NumLock',         'Num7',            'Z811',
    'Z812',            'Num*',            'Num/',            'Num8',
    'Num5',            'Num2',            'Num0',            'Num9',
    'Num6',            'Right',           'NumSep',          'Num-',
    'Num+',            'INTL85',          'Num3',            'Num.',
]
# name -> slot, accepting lowercase
BY_NAME = {}
for _i, _n in enumerate(SLOTS):
    BY_NAME[_n.lower()] = _i

# Slots the matrix addresses but that have no physical key in the US/ABNT
# layout. Only the five with a HID code in the international range 0x85-0x8b.
# Slots 16, 32, 68, 71 and 88 were once on this list for coming back dark in a
# matrix read -- but dark only means "nobody painted it", and the set of dark
# slots changes with the mode. See docs/23 §6.
NO_KEY = (35, 80, 85, 91, 129)


def slot_of(name):
    """Accepts the key name or the slot number. Clear error if neither exists."""
    s = str(name).strip()
    if s.isdigit():
        n = int(s)
        if not 0 <= n < NSLOTS:
            raise ValueError('slot %d out of 0..%d' % (n, NSLOTS - 1))
        return n
    k = BY_NAME.get(s.lower())
    if k is None:
        raise ValueError('unknown key: %r (use the name as in '
                         'docs/10-key-map.md, or the slot number)' % name)
    return k


def color_of(txt):
    """'ff0000' | 'f00' | 'red' -> (r, g, b)."""
    names = {'black': (0, 0, 0), 'white': (255, 255, 255),
             'red': (255, 0, 0), 'green': (0, 255, 0), 'blue': (0, 0, 255),
             'yellow': (255, 255, 0), 'cyan': (0, 255, 255),
             'magenta': (255, 0, 255), 'orange': (255, 96, 0),
             'purple': (128, 0, 255), 'pink': (255, 64, 128)}
    t = str(txt).strip().lower().lstrip('#')
    if t in names:
        return names[t]
    if len(t) == 3:
        t = ''.join(c * 2 for c in t)
    if len(t) != 6:
        raise ValueError('invalid color: %r (use RRGGBB, RGB or a name)' % txt)
    return tuple(int(t[i:i + 2], 16) for i in (0, 2, 4))


class Keyboard:
    def __init__(self):
        self.dev = usb.core.find(idVendor=VID, idProduct=PID)
        if self.dev is None:
            sys.exit('keyboard 28da:1101 not found '
                     '(plugged in? need permission on /dev/bus/usb?)')
        self._claimed = []
        for i in (IFACE_OUT, IFACE_IN):
            try:
                self.dev.detach_kernel_driver(i)
            except (usb.core.USBError, NotImplementedError):
                pass
            usb.util.claim_interface(self.dev, i)
            self._claimed.append(i)

    def close(self):
        for i in self._claimed:
            try:
                usb.util.release_interface(self.dev, i)
                self.dev.attach_kernel_driver(i)
            except Exception:
                pass

    def __enter__(self):
        return self

    def __exit__(self, *a):
        self.close()

    # --- packet layer ----------------------------------------------------
    def build(self, cmd, mode=1, b3=0, b4=0, b5=0, payload=None):
        b = bytearray(PKT)
        b[0] = REPORT_ID
        b[1] = cmd
        b[2] = mode
        b[3], b[4], b[5] = b3, b4, b5
        if payload:
            if len(payload) > PKT - HDR:
                raise ValueError('payload of %d bytes does not fit' % len(payload))
            b[HDR:HDR + len(payload)] = payload
        return bytes(b)

    def write(self, packet, wait=0.02):
        self.dev.ctrl_transfer(REQ_OUT, SET_REPORT, WVALUE, WINDEX, packet, TIMEOUT)
        time.sleep(wait)

    def read(self, packet, wait=0.02):
        """Sends the read command and collects the answer on the control IN."""
        self.write(packet, wait)
        d = bytes(self.dev.ctrl_transfer(REQ_IN, GET_REPORT, WVALUE, WINDEX, PKT, TIMEOUT))
        time.sleep(wait)
        return d

    def reset(self):
        """USB reset -- same as replugging. It is the universal undo: when the
        keyboard gets into a strange state, this brings it back. See docs/24."""
        self.dev.reset()
        time.sleep(2.5)

    def events(self, seconds, callback):
        """Listens on EP 0x83. Mixes ACKs with the keyboard's spontaneous events."""
        t0 = time.time()
        while time.time() - t0 < seconds:
            try:
                r = bytes(self.dev.read(EP_EVENTS, 8, 300))
            except usb.core.USBError:
                continue
            callback(time.time() - t0, r)

    # --- mode ------------------------------------------------------------
    def active_mode(self):
        """Number of the active mode, 1..3. 0x81 answers on the control IN and
        also in byte 4 of the ACK -- here we use the control IN."""
        return self.read(self.build(MODE_GET, mode=0))[2]

    def read_mode(self, mode):
        """The 264 bytes of the mode. Returns a bytearray so it can be edited."""
        d = bytearray(self.read(self.build(INFO_GET, mode=mode), wait=0.05))
        if d[1] != INFO_GET or d[2] != mode:
            raise IOError('unexpected answer from 0x82: cmd=%02x mode=%d'
                          % (d[1], d[2]))
        return d

    def write_mode(self, mode, d):
        """Writes back the 264 bytes read with read_mode(), with the status
        byte zeroed the way the official app does."""
        if len(d) != PKT:
            raise ValueError('the mode payload has %d bytes, expected %d' % (len(d), PKT))
        b = bytearray(d)
        b[0], b[1], b[2] = REPORT_ID, INFO_SET, mode
        b[OFF_STATUS] = 0x00
        self.write(bytes(b), wait=0.05)

    # --- color -------------------------------------------------------------
    def read_colors(self, mode):
        """Returns [r, g, b], each with NSLOTS bytes."""
        out = []
        for ch in CHANNELS:
            d = self.read(self.build(LED_GET, mode=mode, b3=ch), wait=0.05)
            if d[1] != LED_GET or d[3] != ch:
                raise IOError('unexpected answer from 0x89: cmd=%02x channel=%d'
                              % (d[1], d[3]))
            out.append(bytearray(d[HDR:HDR + NSLOTS]))
        return out

    def write_colors(self, mode, channels):
        """Three packets, one per channel. There is no commit command: the
        third packet closes the update (docs/09 §6.4)."""
        if len(channels) != 3 or any(len(c) != NSLOTS for c in channels):
            raise ValueError('expected 3 channels of %d bytes' % NSLOTS)
        for ch, data in zip(CHANNELS, channels):
            self.write(self.build(LED_SET, mode=mode, b3=ch, payload=bytes(data)),
                       wait=0.02)
        time.sleep(0.1)      # the official app's extra wait after the 3rd channel


class Session:
    """Opens the configuration session with 0x16 and GUARANTEES the way back.

        with K.Keyboard() as kb, K.Session(kb):
            ...                       # here the keyboard emits f6 on EP 0x83

    0x16 turns on key reporting (f6). **While the session is open the keyboard
    stops typing** -- observed on 2026-10-16, see docs/24. No observed command
    closes the session: the official software never sends one. What brings it
    back is the USB reset, and that is why this context manager exists.

    The reset runs in __exit__, so it also runs on an exception, Ctrl+C or
    SIGTERM -- as long as the process is not killed with SIGKILL.
    """
    def __init__(self, kb, do_reset=True):
        self.kb = kb
        self.do_reset = do_reset

    def __enter__(self):
        self.kb.write(self.kb.build(0x16, mode=0x01), wait=0.3)
        return self

    def __exit__(self, *a):
        if self.do_reset:
            try:
                self.kb.reset()
            except Exception as e:
                print('WARNING: the USB reset failed (%s). Replug the keyboard.' % e,
                      file=sys.stderr)
        return False
