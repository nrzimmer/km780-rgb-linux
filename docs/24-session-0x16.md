# `0x16` and the configuration session

> 2026-10-16, on the keyboard. **The project's first write.**
> Includes an incident and how to get out of it.

## 1. `0x16` is what turns key reporting on  [C]

The `f6 01 <column> <row>` event on EP `0x83` **does not come by default**. On
a freshly power-cycled keyboard, no key produces any event at all.

Testing the three init candidates, one at a time, with a control phase first
(`tools/km780f6.py`): `f6` started showing up **in the `0x16` phase**. The
earlier phases — control and `07 07 00 15` — produced nothing.

    07 16 01      ->  from here on the keyboard reports keys on EP 0x83

This explains why **none of the 11 captures shows who turns it on**: on Windows
the keyboard already came in turned on from an earlier run of the software, and
the state survives the app closing. In `01-open` the first `f6` events arrive
at t=0.16 s, nineteen seconds before the app sends its `0x16`.

Corollary: `07 07 00 15`, which was the prime suspect, is **not** what turns it
on. It still has no known function.  `[?]`

## 2. The incident: the keyboard stopped typing  [C]

After running the phases, **the keyboard stopped typing**. It was healthy in
every other respect: enumerated, answering commands, with the five *input
devices* registered and the kernel drivers bound on all three interfaces. It
was not bootloader mode — it was an operating state.

### What did not fix it

- `07 0d 01` (leave configuration mode), sent on its own afterwards
- rebinding the kernel drivers; they were already bound

### What did fix it  [C]

**A USB reset.** In Python, `dev.reset()`; it is the equivalent of replugging,
without touching the cable. After it the keyboard re-enumerated and typed
again.

    import usb.core
    usb.core.find(idVendor=0x28DA, idProduct=0x1101).reset()

### Which command caused it — not isolated  [I]

By the time the problem was noticed, `0x16`, `0x0D 00` and probably `0x0D 01`
had already been sent. Suspicion falls on the **session** (`0x16`), because it
is the command that changes the path of key events, but **this was not
isolated** and must not be written down as if it had been.

To isolate it safely: open only `0x16`, ask for some typing, and reset.
`K.Session` below makes that safe to try.

## 3. No command closes the session  [C]

Searching all 11 captures: `0x16` always appears **as `07 16 01`**, four times,
and **never** with another argument. The official software never closes the
session — not even when it exits. In `08-closing` the last messages are
`0x0D 01` and a series of `0x01` (mode switch), and no `0x16`.

So, on the protocol side, **the session has no end; what gives the keyboard
back is the USB reset**.

## 4. Consequence for any Linux tooling

Anyone sending `0x16` has to guarantee the reset on the way out, **including on
an exception, Ctrl+C and SIGTERM** — and cannot count on Ctrl+C, because it
comes from the keyboard that may not be typing.

`km780lib.py` now has this ready:

    with K.Keyboard() as kb, K.Session(kb):
        ...                 # f6 flows here
    # on leaving this block, the USB reset is guaranteed

## 5. The design mistake, so it is not repeated

The first version of `km780f6.py` chained the five phases automatically,
**with nothing to undo what it had done if it were interrupted**. I had checked
that the commands do not change persistent state — and they do not, the
byte-for-byte comparison of the 3 modes and the 3 color matrices showed no
difference — but **I did not consider that they might change the keyboard's
live behaviour**. A state backup does not cover that.

What was missing, and now exists:

- a guaranteed undo (`K.Session`), which runs on any exit
- an absolute time ceiling in the script, so it ends by itself
- not depending on Ctrl+C, which comes from the device the test may knock out
