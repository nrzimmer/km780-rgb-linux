# Analysis of the official binaries

Installers extracted into `external/gds/<version>/payload/` (not versioned).
They are all **Inno Setup 5.5.0**; extracted with `innoextract -e -s`.

    innoextract -e -s -d <destination> <Setup.exe>

Note: G.SKILL's site returns **HTTP 403 to a non-browser client** (curl and
wget, with and without a Referer). Download it in the browser.

## Versions in hand

| Package | App.exe | Source |
|---|---|---|
| KM780 **MX** v0.59 (2015-08) | **managed .NET** (CLR v2.0.50727), 282 KB | Internet Archive |
| KM780 **RGB** v2.03 (2017-10, FW 76) | native, 1.7 MB | gskill.com |
| KM780 **RGB** v2.04 (2019-08, FW 77) | native, 1.7 MB | gskill.com |

This confirms the hypothesis: the old era was .NET, v2.x is native. **But the
only managed binary is the MX's (non-RGB).** The RGB app is native in both
available versions — so it is still worth hunting for the RGB build **v0.61**
or **V0.105**, probably .NET.

## Architecture of the software

    App.exe          main UI
    Setting.dll      ~2.9 MB — lighting/profile logic            <-- THE TARGET
    HidDevice.dll    ~83 KB  — generic HID wrapper
    KBHook.dll       keyboard hook (macros)
    GetOSVer.dll     Windows version detection
    table.xml        virtual-key -> display name map (NOT the LED map)
    FWUpdate/        updater + firmware in Intel HEX
    KM780RGB_Reset/  official reset utility

### HidDevice.dll — exports

A generic HID wrapper; it **does not contain the protocol logic**:

    HidDevice_Open            HidDevice_Close
    HidDevice_SetFeature      HidDevice_GetFeature
    HidDevice_SetOutputReport HidDevice_GetInputReport
    HidDevice_Write           HidDevice_SetReadCallBack
    HidDevice_GETCAPS         HidDevice_GetDevVersion
    HidDevice_SetCaptureDevice HidDevice_PowerSuspend
    HidDevice_Version

`SetFeature`/`GetFeature` correspond to the pair of control transfers described
in `01-transport.md`. The protocol is assembled by the caller, in
`Setting.dll`.

### Setting.dll — it has RTTI with class names

A finding that changes the game: MSVC left the RTTI in, so the C++ classes are
named. A decompiler will reconstruct this with real names:

    CkeyboardLighting      <-- probable owner of the key->LED map
    CLightingEffect
    CEffectSetting
    CSelEffectDlg
    CProfileManage

### Locating the packet builders

A byte sweep (`push 0x108` = pushing 264, the packet size):

| File | `push 0x108` | `0x0307` as a dword |
|---|---|---|
| rgb-v2.04/Setting.dll | **37** | 2 |
| rgb-v2.03/Setting.dll | 36 | 2 |
| rgb-v2.04/App.exe | 3 | 1 |

Those 37 occurrences in `Setting.dll` are the call sites that build the
264-byte packets. That is the way in to find the LED payload layout.

## Firmware

**Format: Intel HEX**, not raw binary. Three versions in hand:

    rgb-v2.03/payload/app/FWUpdate/Skill_1_FW_2017_09_05_75.hex
    rgb-v2.03/payload/app/FWUpdate/Skill_1_FW_2017_09_30_76.hex
    rgb-v2.04/payload/app/FWUpdate/Skill_1_FW_2017_10_11_77.hex   (94.7 KiB)
    mx-v0.59/payload/app/FWUpdate/G-Skill2_V0018.bin              (64 KiB, from the MX)

### The microcontroller

`FWUpdate/info.ini` from the RGB package:

    [Firmware]
    Path=Skill_1_FW_2017_10_11_77
    RequireVer=77
    IC_NAME=11U35

**`11U35` = NXP LPC11U35**, an ARM Cortex-M0 with USB device support. That sets
the disassembly target: ARM Cortex-M0 (Thumb), and it explains the keyboard's
USB architecture.

Having **three consecutive firmware versions** is useful: the diff between them
tends to isolate what changed, and each release's changelog (it ships in the
.zip as a PDF) says what was fixed.

## Safety net

There is an **official** reset utility: `app/KM780RGB_Reset/`. Its `Info.xml`:

> "You have chosen to force a keyboard setting reset. This action will format
> the onboard flash memory. All current settings will be lost."

It formats the onboard configuration memory. That covers *configuration*
corruption — it is not firmware recovery (that is the safety-pin method, see
`04-firmware.md`).

## Next step

Disassemble `Setting.dll` starting from the 37 occurrences of `push 0x108` and
from the `CkeyboardLighting` / `CLightingEffect` classes. It needs **Ghidra**
or **radare2/rizin** — neither installed on this machine yet.

In parallel, and cheaper: decompile `mx-v0.59/App.exe` with ILSpy. It is .NET,
it uses the same `HidDevice.dll` with the same API, and managed code shows the
command structure with symbolic names. It is probably the origin of the names
(`commandSetLEDMatrixDatabase` and so on) that appear in the abandoned repo.
