# Map of Setting.dll (RGB v2.04)

Target: `external/gds/rgb-v2.04/payload/app/Setting.dll`
Analysed with Ghidra 12.1.4 via REA. Persistent project also in
`ghidra-proj/KM780.rep` (created with `analyzeHeadless`, 68 MB).

## Facts about the binary

    format         PE32, x86
    image base     0x10000000
    procedures     19,251
    strings        3,188
    PDB            references Setting.pdb (not shipped) -> no original names

Sections:

| Name | Start | End | Size |
|---|---|---|---|
| `.text` | `0x10001000` | `0x10210e00` | 2,162,176 |
| `.rdata` | `0x10211000` | `0x1027d800` | 444,416 |
| `.data` | `0x1027e000` | `0x103e6e40` | 1,478,208 |
| `.rsrc` | `0x103e7000` | `0x10401a00` | 109,056 |

## Anchors

### Strings and RTTI (`rea search Lighting`)

| Address | Content |
|---|---|
| `0x102841b0` | `.?AVCkeyboardLighting@@` (RTTI type descriptor) |
| `0x102841f0` | `.?AVCLightingEffect@@` (RTTI type descriptor) |
| `0x102479f8` | `CkeyboardLighting` |
| `0x102480d4` | `CLightingEffect` |
| `0x1024e418` | `LIGHTING_PROFILE_%d` |
| `0x1024e0c0` | `LIGHTING_PROFILE` |
| `0x10246c1c` | `Lighting Profiles_Data\` |
| `0x10247b38` | `\skin_lighting.ini` |

### Code that uses `LIGHTING_PROFILE_%d`

`rea xrefs 0x1024e418` → **`0x10099cf3`** and **`0x100a4c18`**

Note: `rea xrefs` on the RTTI type descriptor (`0x102841b0`) comes back
**empty** — the descriptor is referenced by the *Complete Object Locator*,
which is data, and `xrefs` seems to list only code references. Reaching
`CkeyboardLighting`'s virtual methods would mean going through the locator and
the vftable; the shorter route is to start from the two functions above.

## The 37 packet builders

`rea search-bytes 6808010000` (= `push 0x108`, the packet size), 37 matches,
`complete: true`:

    0x100c9a09
    0x100d1ca8  0x100d1d5f  0x100d1e04  0x100d1ec1  0x100d1f74
    0x100d4eda  0x100d5001
    0x100d6af8  0x100d7110  0x100d7632  0x100d7705  0x100d7736  0x100d7d40
    0x100d9025  0x100d9046  0x100d910a  0x100d912b
    0x100d9b33  0x100d9b55  0x100da935  0x100da953
    0x100db082  0x100db0a6  0x100db513  0x100db537
    0x100db58c  0x100db5ae  0x100db613  0x100db635
    0x100dbab0  0x100dbbb5  0x100dbc77  0x100dbcb4
    0x101c11e1  0x101c54b4  0x101c5512

**34 of the 37 sit in `0x100c9000`–`0x100dc000`** — that is the device
communication module.

### The pattern of the pairs

Eight pairs about 0x20 bytes apart:

    0x100d9025 / 0x100d9046      0x100d910a / 0x100d912b
    0x100d9b33 / 0x100d9b55      0x100da935 / 0x100da953
    0x100db082 / 0x100db0a6      0x100db513 / 0x100db537
    0x100db58c / 0x100db5ae      0x100db613 / 0x100db635

The classic `memset(buf, 0, 0x108)` followed by `send(buf, 0x108)` idiom —
that is, the packet builders proper.

## Useful REA commands (learned the hard way)

    rea search <bin> <text> --snapshot <f>        # strings and procedure names
    rea search-bytes <bin> <hex>                  # does NOT accept --snapshot
    rea xrefs <bin> <addr>                        # code references only
    rea decompile <bin> <addr> --snapshot <f>
    rea function <bin> <addr>

Two pitfalls to avoid:

- **zsh does not word-split** an unquoted variable, so keeping flags in a
  variable (`S="--snapshot x"; rea ... $S`) sends the whole thing as a single
  argument. Pass them inline, or use `${=S}`.
- `--token-limit` truncated the list of 37 matches. For long lists, use
  `--filter-output normalized_result --format json` and parse it.

## In progress

Decompiling `0x100db513` and `0x100d9025` (representatives of the dense
cluster).
