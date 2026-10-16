#!/usr/bin/env python3
"""Searches a PE for byte patterns and reports them as virtual addresses.

Usage:  pescan.py <file.dll> <hex-pattern-with-??> [...]
E.g.:   pescan.py Setting.dll "C685????????09"

It exists because every REA invocation re-analyses the whole binary (2-3 min),
unworkable for iterative queries.
"""
import sys, struct


def sections(d):
    pe = struct.unpack_from('<I', d, 0x3c)[0]
    assert d[pe:pe+4] == b'PE\0\0', 'not a PE'
    nsec = struct.unpack_from('<H', d, pe+6)[0]
    optsz = struct.unpack_from('<H', d, pe+20)[0]
    base = struct.unpack_from('<I', d, pe+24+28)[0]   # ImageBase
    off = pe + 24 + optsz
    out = []
    for i in range(nsec):
        h = d[off+i*40:off+(i+1)*40]
        name = h[0:8].rstrip(b'\0').decode('ascii', 'replace')
        vsize, va, rawsize, rawptr = struct.unpack_from('<IIII', h, 8)
        out.append((name, va, vsize, rawptr, rawsize))
    return base, out


def off2va(base, secs, o):
    for name, va, vsize, rawptr, rawsize in secs:
        if rawptr <= o < rawptr + rawsize:
            return base + va + (o - rawptr), name
    return None, None


def parse(pat):
    pat = pat.replace(' ', '')
    if len(pat) % 2:
        sys.exit('pattern with an odd number of nibbles')
    by, mask = bytearray(), bytearray()
    for i in range(0, len(pat), 2):
        p = pat[i:i+2]
        if p == '??':
            by.append(0); mask.append(0)
        else:
            by.append(int(p, 16)); mask.append(0xFF)
    return bytes(by), bytes(mask)


def find(d, by, mask):
    n = len(by)
    res = []
    if all(m == 0xFF for m in mask):           # fast path
        i = 0
        while True:
            i = d.find(by, i)
            if i < 0:
                break
            res.append(i); i += 1
        return res
    anchor = next((k for k, m in enumerate(mask) if m), 0)
    ab = by[anchor:anchor+1]
    i = 0
    while True:
        i = d.find(ab, i)
        if i < 0 or i - anchor + n > len(d):
            break
        s = i - anchor
        if s >= 0 and all((d[s+k] & mask[k]) == by[k] for k in range(n)):
            res.append(s)
        i += 1
    return res


if __name__ == '__main__':
    if len(sys.argv) < 3:
        sys.exit(__doc__)
    d = open(sys.argv[1], 'rb').read()
    base, secs = sections(d)
    print('ImageBase 0x%08x' % base)
    for name, va, vsize, rawptr, rawsize in secs:
        print('  %-8s VA 0x%08x  size 0x%06x  raw 0x%06x' % (name, base+va, vsize, rawptr))
    for pat in sys.argv[2:]:
        by, mask = parse(pat)
        hitlist = find(d, by, mask)
        print('\n### %s  -> %d occurrences' % (pat, len(hitlist)))
        for o in hitlist:
            vaddr, sec = off2va(base, secs, o)
            if vaddr is None:
                continue
            print('   0x%08x  [%-7s]  %s' % (vaddr, sec, d[o:o+len(by)].hex(' ')))
