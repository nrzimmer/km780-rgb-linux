# Line of work: firmware analysis

## Priority: low

This is the lowest return-per-effort line of work. The firmware is the MCU's
code — it gives the *device* side, not the key→offset map on the *host* side,
which is what is missing. And analysing an MCU binary (probably 8051 or ARM
Cortex-M, from Holtek/Nuvoton/SinoWealth) is far more laborious than reading
decompiled .NET.

Worth it as later confirmation, or if the other two lines stall.

## Where to get it

**There is no standalone firmware** on G.SKILL's download page. The versions
(FW 76, FW 77) ship **embedded in the installers** — so downloading the
installer for the decompilation line (`03-decompilation.md`) already covers
this one.

Look inside the extracted installer for:

    find . -iname '*.bin' -o -iname '*.hex' -o -iname '*.fw' -o -iname '*.rom'

Then: `file`, `binwalk`, `strings`, and entropy to detect compression or
encryption.

## Lead from the forum

There is a report on the G.SKILL forum (2016) of recovering a KM780 via a
safety-pin reset + a `.bin` file. It confirms the firmware is flashable from
the host — and it is also the reason for caution: **it is possible to leave
the keyboard in a bad state.**

## Findings

(nothing yet)
