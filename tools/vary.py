#!/usr/bin/env python3
"""Shows, per command, which packet bytes vary across the captures.

    tools/vary.py 0x02                 every 0x02 packet from every capture
    tools/vary.py 0x02 captures/02-*   only those captures
    tools/vary.py --all                summary of every command

This is the tool that locates an unknown field: a byte that never varies
carries no information, and one that varies together with a UI action is that
action's field. Reads the .txt files from tools/extract.sh.
"""
import sys, glob, collections

def load(cmd, files):
    packets = []
    for fn in files:
        for line in open(fn):
            c = line.rstrip("\n").split("\t")
            if len(c) < 5 or c[2] == "ACK":
                continue
            if cmd is None or int(c[3], 16) == cmd:
                packets.append((fn.split("/")[-1], int(c[0]), c[2], bytes.fromhex(c[4])))
    return packets

def variance(packets):
    if not packets:
        return []
    n = min(len(p[3]) for p in packets)
    out = []
    for i in range(n):
        vals = collections.Counter(p[3][i] for p in packets)
        if len(vals) > 1:
            out.append((i, vals))
    return out

def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    every = "--all" in sys.argv
    files = [a for a in args if "/" in a or a.endswith(".txt")] or sorted(glob.glob("captures/*.txt"))
    if every:
        pk = load(None, files)
        per_cmd = collections.defaultdict(list)
        for p in pk:
            per_cmd[p[3][1]].append(p)
        print(f"{'cmd':>5} {'dir':>4} {'packets':>8} {'bytes that vary':>17}   ranges")
        for cmd in sorted(per_cmd):
            ps = per_cmd[cmd]
            v = variance(ps)
            idx = [i for i, _ in v]
            ranges, start = [], None
            for k, i in enumerate(idx):
                if start is None:
                    start = i
                if k + 1 == len(idx) or idx[k + 1] != i + 1:
                    ranges.append(f"{start}" if start == i else f"{start}-{i}")
                    start = None
            d = "SET" if cmd < 0x80 else "GET"
            print(f" 0x{cmd:02x} {d:>4} {len(ps):>8} {len(idx):>17}   {' '.join(ranges[:14])}")
        return
    if not args:
        print(__doc__); sys.exit(1)
    cmd = int(args[0], 16)
    ps = load(cmd, files)
    print(f"command 0x{cmd:02x}: {len(ps)} packets in {len(files)} captures\n")
    for i, vals in variance(ps):
        top = ", ".join(f"{v:02x}×{c}" for v, c in vals.most_common(8))
        extra = f" (+{len(vals)-8} values)" if len(vals) > 8 else ""
        print(f"  byte {i:>3}  {len(vals):>3} values   {top}{extra}")

main()
