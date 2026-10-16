# The protocol as seen by the firmware

Firmware FW 77 (`Skill_1_FW_2017_10_11_77.hex`), NXP LPC11U35, ARM Cortex-M0.
Loaded at `0x00002000`, 34,448 bytes. Ghidra project: `ghidra-proj/KM780`,
program `fw77.bin`, language `ARM:LE:32:Cortex`.

Queries via `tools/ghidra/*.java` (GhidraScript in Java — `analyzeHeadless`
does **not** initialize PyGhidra, and Ghidra 12.1.4 does not ship the
`pyghidra` wheel).

    analyzeHeadless ghidra-proj KM780 -process fw77.bin -noanalysis \
        -scriptPath tools/ghidra -postScript FwDecomp.java 0x81b8

| Script | What it is for |
|---|---|
| `FwProbe.java` | inventory of functions, xrefs, dispatcher candidates |
| `FwFindConst.java` | finds instructions with specific immediates |
| `FwDecomp.java` | decompiles functions by address |

Coverage: auto-analysis recognised **175 functions**, which is incomplete for
34 KB — the vector table was not typed, so part of the code was not reached by
flow. Absent xrefs do not prove an absent reference.

## Color channel, confirmed by the device  [C]

`FUN_000081b8`:

    void FUN_000081b8(uint channel, void *data) {
      if (channel > 3 || channel == 0) channel = 1;     // clamp to 1..3
      FUN_0000847c(channel * 0x1000 - 0xf90, data, 0x90);
    }

This confirms, from the firmware side, what the DLL already indicated:
**channel ∈ {1,2,3}**, **144 bytes (`0x90`) per channel**.

## There is an external SPI flash  [C]

`FUN_0000847c` is an SPI NOR flash access:

    param_1 += 0x1000;
    buf = [0x03, addr>>16, addr>>8, addr];   // 0x03 = standard SPI NOR READ opcode
    FUN_00002198(0, 0x17, 0);                // chip select low (GPIO 0x17)
    FUN_00002b90(1, buf, 4);                 // opcode + 24-bit address
    FUN_00002bf8(1, param_2, param_3);       // data transfer
    FUN_00002198(0, 0x17, 1);                // chip select high

Since the opcode is `0x03` (READ), `FUN_000081b8` **reads** the channel's 144
bytes from the flash — it does not write.

Effective addresses (`channel * 0x1000 - 0xf90 + 0x1000`):

| Channel | Address in the flash |
|---|---|
| 1 | `0x1070` |
| 2 | `0x2070` |
| 3 | `0x3070` |

4 KB banks per channel. This is the memory that the official `KM780RGB_Reset`
utility formats ("This action will format the onboard flash memory").

## Device → host notification channel  [C]

`FUN_0000358c` builds an 8-byte message in RAM and raises a transmit flag:

| Offset | Value |
|---|---|
| +0 | `0x02` |
| +1 | `0x04` |
| +2 | `param_1` |
| +3 | `param_2` — event code |
| +4 | `param_3` — value |
| +5 | `param_4` |
| +6 | `param_5` |

Eight bytes corresponds to the **Input report of interface 2** (EP `0x83`),
whose report descriptor is at `0x0000a412`. So: it is the **device → host**
event channel, through which the keyboard tells the app about changes made on
the physical controls.

**A distinct namespace from the Feature report commands.** Codes may coincide,
but these are different directions and different channels.  [I]

### Brightness event  [I]

`FUN_000027f0` keeps a clamped counter, driven by the codes `0xAA` and `0xAB`
(the physical brightness keys), and emits:

    FUN_0000358c(handle, 0x0b, level, 0, 0);

That is, **event `0x0B` = "brightness changed to `level`"**. The counter is
limited to `0x30` going up and to `0` going down (with a test against 100 on
the way down), which suggests a brightness range of 0–48.  [I]

This does **not** establish that `0x0B` is the host→device brightness command,
even though `0x0B` is exactly the write counterpart predicted for the `0x8B`
observed in the DLL.

## Discarded leads

Two attempts that went nowhere, recorded so nobody repeats them:

- **`0x0000a2d4` is not code.** It showed up as a "Thumb pointer" in a literal
  block only because the address was odd. It is the **mouse report descriptor**
  (the 252-byte one from interface 1), unaligned data.
- **`FUN_00003ffe` is not the dispatcher.** It was the strongest candidate
  because it compares against 29 distinct immediates, including `0x85`,
  `0x88`–`0x8B`. But those are **HID usage codes**, not command bytes: they are
  precisely the international slots 35, 80, 91 and 129 of the key map. The
  function is the key and macro scanning loop. Hunting for the dispatcher by
  `cmp #imm` fails because the command bytes collide numerically with HID
  usages.

## Open

- the Feature report dispatcher (host → device) has not been located yet
- the channel 1/2/3 → R/G/B correspondence remains undetermined: the firmware
  treats the three channels symmetrically, so probably **only a hardware test
  or a USB capture settles it**
- who **writes** to the SPI flash (opcode `0x02` PAGE PROGRAM, `0x06` WREN)
- improving coverage: type the vector table and re-analyse
