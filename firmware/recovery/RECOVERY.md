# Recovering the G.SKILL KM780 RGB keyboard

## Situation

The keyboard is in bootloader mode. It shows up on USB as
"SKILL Gaming Keyboard" with bcdDevice 0.01, instead of
"G.SKILL RIPJAWS KM780 RGB" with bcdDevice 0.77.

Nothing was erased: the bootloader is the keyboard's own recovery mechanism.
All that is missing is reflashing the application firmware.

## What to do

1. Leave the keyboard plugged in (it will be dark and will not type --
   normal in bootloader mode). Use another keyboard to drive Windows.
2. Run `FWUpdate.exe` from this folder, as Administrator.
3. Click Update ("Click Update to begin").
4. Wait for the message "Update Finished!!".
5. Unplug and replug the keyboard.

The keyboard should type and light up again.

## Checking that it worked

In Device Manager the name goes back to
"G.SKILL RIPJAWS KM780 RGB GAMING KEYBOARD".

## Files

| File | What it is |
|---|---|
| `FWUpdate.exe` | G.SKILL's official updater |
| `Skill_1_FW_2017_10_11_77.hex` | FW 77 firmware (what was on the keyboard) |
| `HidDevice.dll` | USB communication library |
| `info.ini` | points at the firmware and the MCU (LPC11U35) |
| `alternative-firmware/` | FW 75 and FW 76, in case 77 fails |

## If it does not work

There is a safety-pin reset documented on the G.SKILL forum. Say the word and
I will look up the exact procedure.

Source: official installer `G.SKILL_RIPJAWSKeyboard_KM780_RGB_Setup_V2.04.zip`,
extracted into `~/src/km780-rgb-linux/external/gds/` on Fedora.
