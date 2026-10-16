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

## Findings

(nothing yet — waiting for the installers)
