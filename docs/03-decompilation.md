# Line of work: decompiling the official app

## Why it is the most promising line

A USB capture shows only what was exercised in the interface. The binary has
the **whole key→index table** and the **complete command set**, including the
ones the UI does not expose. On top of that: it runs on Linux, needs no reboot
and poses no risk to the hardware.

A strong hint: the command names in the abandoned repo
(`commandSetLEDMatrixDatabase`, `commandGetLEDEffectDatabase`) are symbolic.
Nobody deduces those names from hex — they came from inside the app.

## The central finding: v1.x was .NET

The v2.0 announcement (2017-09-25) says that from then on the G.SKILL Driver
System **no longer required the .NET Framework**. Which means:

- **v1.x / 0.9x = managed .NET** → ILSpy/dnSpy gives back practically readable
  C#. **That is the priority target.**
- **v2.x = native** → far more work.

The abandoned repo runs from 2016-07 to 2017-02, before v2.0. That confirms the
protocol was mapped from a .NET build.

## Downloads

Official KM780 RGB page:
<https://www.gskill.com/download/1502180935/1551783153/RIPJAWS-KM780-RGB>

It lists only 2.x versions:

| Version | Date | File |
|---|---|---|
| SW 2.04 / FW 77 | 2019-08-14 | `G.SKILL_RIPJAWSKeyboard_KM780_RGB_Setup_V2.04.zip` |
| SW 2.03 / FW 76 | 2017-10-03 | `G.SKILL_RIPJAWSKeyboard_KM780_RGB_Setup_V2.03.zip` |

URL base: `https://www.gskill.com/gskill-device/keyboard/`

**The site returns HTTP 403 to a non-browser client** (tested with curl and
wget, with and without a Referer). Download it in the browser.

### Hunting for a v1.x

Older versions were dropped from the listing. Internet Archive query:

    https://web.archive.org/cdx/search/cdx?url=gskill.com/gskill-device/keyboard*&fl=timestamp,original,length&collapse=original&limit=500

Guesses following the known pattern are in `external/gds/candidates.md`.

### Alternative: the KM780 MX software

Build **0.90** of the KM780 MX (2016-10) requires **.NET Framework 3.5** — so
it is managed and decompiles easily. The model is not RGB, but the GDS shares
the base (command framing, profiles, macros), and it may contain inactive RGB
code.

<https://www.gskill.com/download/1502180935/1551783139/RIPJAWS-KM780-MX>

## Tools

Available on this machine: `7z`, `file`, `strings`, `objdump`.
Missing (in the Fedora repos): `innoextract`, `binwalk`.
For .NET: `ilspycmd` / ILSpy, or `monodis`.

## Installer naming schemes

The CDX query (`external/gds/cdx-2026-10-16.md`) revealed **two** schemes,
different from the initial guesses:

**Old era (2015), in a per-model subfolder:**

    /gskill-device/keyboard/RGB/G.SKILL_RIPJAWS_Keyboard_KM780_RGB_v0.61_Setup.zip
    /gskill-device/keyboard/MX/G.SKILL_RIPJAWS_Keyboard_KM780_MX_v0.59_Setup.zip

(underscore after `RIPJAWS`, lowercase `v`, `_Setup` at the end)

**New era, at the root:**

    /gskill-device/keyboard/G.SKILL_RIPJAWSKeyboard_KM780_RGB_Setup_V0.105.zip
    /gskill-device/keyboard/G.SKILL_RIPJAWSKeyboard_KM780_RGB_Setup_V2.03.zip

So a **V0.105** RGB build exists, besides the **v0.61** from 2015.

## Findings

### Archived and intact

- **KM780 MX v0.59** (2015-10-16), 12,946,127 bytes — it was .NET. A viable
  secondary target (non-RGB model, but a shared code base).

      https://web.archive.org/web/20151016190901id_/http://www.gskill.com/gskill-device/keyboard/MX/G.SKILL_RIPJAWS_Keyboard_KM780_MX_v0.59_Setup.zip

### Archived but corrupt (an error page, not the binary)

- KM780 RGB v0.61 — snapshot 20151015105904 is only 572 B
- KM780 RGB V0.105 — the 2025/2026 snapshots are 359 to 8946 B

For both: query the CDX again **without** `collapse=original`, which hides the
other snapshots. Look for `statuscode` 200 with a `length` in the millions.

### A note on the Wayback Machine

To download a binary from the Wayback Machine, insert `id_` after the
timestamp: `/web/<timestamp>id_/<url>`. Without it you get the HTML wrapper and
the file is corrupt.

### Manuals (they may hold the key map)

The software manuals 1.15h/1.16d/1.16e/1.17f are archived intact (2 to 3 MB).
Worth a look: RGB manuals sometimes show the LED numbering/zones, which would
help validate the key→offset map.
