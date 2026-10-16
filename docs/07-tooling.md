# Installed tooling

## Ghidra 12.1.4

**It is not in dnf** — nothing in the Fedora repos. The Flatpak
(`org.ghidra_sre.Ghidra`) only has 12.1.3, and GhidraMCP 2.0 requires 12.1.4;
on top of that the Flatpak sandbox makes installing an extension and pointing
`GHIDRA_INSTALL_DIR` awkward. Hence: the official zip from GitHub.

Installed **in the home directory, without root**:

    ~/.local/opt/ghidra_12.1.4_PUBLIC    876 MB
    ~/.local/opt/ghidra                  -> symlink (makes upgrading easier)
    ~/.local/bin/ghidraRun               -> on PATH
    ~/.local/bin/analyzeHeadless         -> on PATH

Java: OpenJDK 25.0.3 (the requirement is 21+).

### Outstanding

`~/.zshrc` is a symlink into the `~/dotfiles` repo, so it was not edited. Still
to be added there:

    export GHIDRA_INSTALL_DIR="$HOME/.local/opt/ghidra"

Without that, export it on the command line each time (needed for headless
GhidraMCP and for building extensions from source).

## GhidraMCP 2.0

Extension installed in `~/.local/opt/ghidra/Ghidra/Extensions/GhidraMCP/`.
Release 2.0 (2026-10-10), matching Ghidra 12.1.4.

Installed, but in practice redundant with REA (see below), which solves the
same problem with more reach. It is for **targeted** decompilation: list
classes/methods, search by name, follow xrefs and decompile function by
function — instead of dumping 2.9 MB of pseudo-C. That is how Ghidra works
anyway:

- **auto-analysis** sweeps the whole binary; it is unavoidable, costs minutes,
  but runs **once** and is saved in the project;
- **decompilation is per function and on demand**; Ghidra never decompiles
  everything.

To enable it in the GUI: File → Configure → Developer → GhidraMCPPlugin.
HTTP server port: Edit → Tool Options → GhidraMCP HTTP Server
(default `127.0.0.1:8080`).

Registering it in Claude Code:

    claude mcp add ghidra -- uvx --from git+https://github.com/LaurieWired/GhidraMCP \
        ghidra-mcp --ghidra-server http://127.0.0.1:8080/

**Mind the Python version:** GhidraMCP's headless mode supports 3.10–3.13, and
this system has **3.14.8**. If it complains, `uvx` solves it by fetching an
older interpreter in isolation (`uvx --python 3.13 ...`).

`uv`/`uvx` are already in `/usr/bin`.

## Ghidra project

    ghidra-proj/   the "KM780" project (not versioned, see .gitignore)

Created with:

    GHIDRA_INSTALL_DIR=$HOME/.local/opt/ghidra analyzeHeadless \
        ~/src/km780-rgb-linux/ghidra-proj KM780 \
        -import external/gds/rgb-v2.04/payload/app/Setting.dll

## REA — Reverse Engineer Anything

<https://github.com/morluto/rea> — `rea-agents` 6.5.0, MIT. **The main tool.**
Installed globally with npm, without root (nvm's prefix is writable):

    npm install --global rea-agents

An MCP/CLI for reverse engineering that uses Hopper, Ghidra or IDA as a
*provider*. In one package it covers **every line of work in this project**:
native PE32 binary (`Setting.dll`), .NET assembly (the MX's `App.exe`),
firmware, and packet captures.

### Why it fits

Two commands solve exactly the "decompile only until we find what we need"
problem, without dumping 2.9 MB of pseudo-C into the context:

    rea decompile <path> [address]    # read ONE part of the app as code
    rea function <path> <addr>        # analyse ONE whole function, with evidence

And two important flags:

- `--snapshot <file>` — persists the analysis between calls, so Ghidra's very
  expensive auto-analysis runs only once;
- `--token-limit <n>` — caps the output size.

Other commands useful here: `analyze`, `compare-managed-members` (an inventory
of .NET members, for the MX app), `extract-firmware`,
`annotate-native-function`, `execute-rizin-command`.

### State of `rea doctor`

`healthy: true`. Ghidra 12.1.4 passes all 8 checks:

    configuration, platform (linux), architecture (x64), installation,
    version (12.1.4), headless, native_decompiler (elf x86_64 matches
    linux/x64), java (25.0.3, 64-bit JDK, accepts 21+)

The remaining `missing_analysis_engine` entries are irrelevant to us:
**Hopper** (requires macOS), **IDA** (no licence) and **JADX** (Android).

### Registering it in Claude Code

    rea setup --client claude_code --yes

Two actions, both local and reversible:

- registers the MCP in `~/.claude-personal/.claude.json`
  (automatic backup in `.claude.json.rea.backup`)
- installs the skill in `~/.claude-personal/skills/reverse-engineer-anything`

The MCP tools only appear after restarting Claude Code; **the CLI works
immediately**. Use `--dry-run` first to review the plan.

`binwalk`/`unblob` are missing (REA uses them in `extract-firmware`) — they
only matter for the firmware line of work, which is low priority.

## Other LLM+RE projects evaluated

| Project | Verdict |
|---|---|
| [REA](https://github.com/morluto/rea) | **chosen** — covers every line of work |
| [GhidraMCP](https://github.com/LaurieWired/GhidraMCP) | installed as a Ghidra extension; redundant with REA |
| [LLM4Decompile](https://github.com/albertan017/llm4decompile) | no use — x86-64 Linux only, our target is PE32 |
| [GhidraGPT](https://github.com/weirdmachine64/ghidragpt) | one function at a time, no global analysis |
| [Codex Decompiler](https://github.com/trailofbits/Codex-Decompiler) | from 2022, tested only up to Ghidra 10.3 |

## Already installed earlier (via dnf, with root)

    innoextract 1.9      to extract the Inno Setup installers
    wireshark-cli 4.6.9  tshark, for the USB capture line of work

## Other tools present

`7z`, `file`, `strings`, `objdump`, `unzip`, `python3` 3.14.8, and a venv with
`pyusb` 1.3.1 in `venv/`.
