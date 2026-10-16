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

It is for **targeted** decompilation: list classes/methods, search by name,
follow xrefs and decompile function by function — instead of dumping 2.9 MB of pseudo-C. That is how Ghidra works
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

## Other LLM+RE projects evaluated

There is no GitHub project called "reverse engineer anything"; the name
probably comes from GhidraMCP's description ("allowing LLMs to autonomously
reverse engineer applications"). Alternatives considered:

| Project | Verdict |
|---|---|
| [GhidraMCP](https://github.com/LaurieWired/GhidraMCP) | **chosen** |
| [LLM4Decompile](https://github.com/albertan017/llm4decompile) | no use — x86-64 Linux only, our target is PE32 |
| [GhidraGPT](https://github.com/weirdmachine64/ghidragpt) | one function at a time, no global analysis |
| [Codex Decompiler](https://github.com/trailofbits/Codex-Decompiler) | from 2022, tested only up to Ghidra 10.3 |

## Already installed earlier (via dnf, with root)

    innoextract 1.9      to extract the Inno Setup installers
    wireshark-cli 4.6.9  tshark, for the USB capture line of work

## Other tools present

`7z`, `file`, `strings`, `objdump`, `unzip`, `python3` 3.14.8, and a venv with
`pyusb` 1.3.1 in `venv/`.
