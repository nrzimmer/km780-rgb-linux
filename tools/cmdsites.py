#!/usr/bin/env python3
"""Finds every site that builds a KM780 packet in a PE.

    tools/cmdsites.py external/gds/rgb-v2.04/payload/app/Setting.dll

A packet always starts with buf[0]=0x07 (Report ID) and buf[1]=<command>, two
`mov byte ptr [reg+disp], imm8` with consecutive disp. Looking for the PAIR is
what separates a packet site from any old 0x07 in the code -- and it is what
pescan.py, which searches for a single immediate, cannot do.

It also reports buf[2]..buf[5] when they are immediates in the same stretch.
"""
import sys, struct, collections

def sections(d):
    pe = struct.unpack_from('<I', d, 0x3c)[0]
    nsec = struct.unpack_from('<H', d, pe + 6)[0]
    optsz = struct.unpack_from('<H', d, pe + 20)[0]
    base = struct.unpack_from('<I', d, pe + 24 + 28)[0]
    off = pe + 24 + optsz
    out = []
    for i in range(nsec):
        h = d[off + i * 40:off + (i + 1) * 40]
        name = h[0:8].rstrip(b'\0').decode('ascii', 'replace')
        vsize, va, rawsize, rawptr = struct.unpack_from('<IIII', h, 8)
        out.append((name, base + va, vsize, rawptr, rawsize))
    return base, out

def movs(b, va0):
    """Yields (va, reg, disp, imm, size) for each `mov byte [reg+disp], imm8`."""
    i, n = 0, len(b)
    while i < n - 8:
        if b[i] == 0xc6:
            m = b[i + 1]; mod, rm = m >> 6, m & 7
            sib = rm == 4
            if mod == 1:
                k = 3 + sib
                disp = struct.unpack_from('<b', b, i + 2 + sib)[0]
            elif mod == 2:
                k = 6 + sib
                disp = struct.unpack_from('<i', b, i + 2 + sib)[0]
            else:
                i += 1; continue
            reg = 'esp' if sib else ('ebp' if rm == 5 else 'r%d' % rm)
            yield (va0 + i, reg, disp, b[i + k], k + 1)
            i += k + 1
            continue
        i += 1

def main():
    d = open(sys.argv[1], 'rb').read()
    base, secs = sections(d)
    for name, va, vsize, rawptr, rawsize in secs:
        if name != '.text':
            continue
        b = d[rawptr:rawptr + rawsize]
        rows = list(movs(b, va))
        # One buf[0]=0x07 can serve several packets in a row, with only buf[1]
        # rewritten between them (the case of 0x92/0x95/0x8B in FUN_100db460).
        # So: every position that at some point receives 0x07 is a packet base,
        # and every store at base+1 is a command.
        bases = {(reg, disp) for v, reg, disp, imm, _ in rows if imm == 0x07}
        perpos = collections.defaultdict(list)
        for v, reg, disp, imm, _ in rows:
            perpos[(reg, disp)].append((v, imm))
        found = []
        for (reg, disp) in bases:
            for v, cmd in perpos.get((reg, disp + 1), []):
                fields = {}
                for k in range(2, 6):
                    for w, imm in perpos.get((reg, disp + k), []):
                        if abs(w - v) < 160:
                            fields[k] = imm
                found.append((v, cmd, reg, disp, fields))
        found.sort()
        percmd = collections.defaultdict(list)
        for v, cmd, reg, disp, fields in found:
            percmd[cmd].append((v, reg, disp, fields))
        print(f"{len(found)} packet sites, {len(percmd)} distinct commands\n")
        print(f"{'cmd':>5}  {'n':>2}  sites (buf[0] at ...)")
        for cmd in sorted(percmd):
            ss = percmd[cmd]
            det = "  ".join(f"{v:08x}" for v, _, _, _ in ss[:6])
            extra = ""
            cs = [c for _, _, _, c in ss if c]
            if cs:
                parts = []
                for k in (2, 3, 4, 5):
                    vals = sorted({c[k] for c in cs if k in c})
                    if vals:
                        parts.append("buf[%d]=%s" % (k, ",".join("%02x" % x for x in vals)))
                extra = "   " + " ".join(parts)
            print(f" 0x{cmd:02x}  {len(ss):>2}  {det}{'  ...' if len(ss) > 6 else ''}{extra}")

main()
