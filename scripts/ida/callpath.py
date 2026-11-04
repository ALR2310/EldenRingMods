"""Find a call path (through direct calls) from a set of root functions to a target function.

Usage:
    run.ps1 callpath <target> <root>... [--depth=N] [--decompile]

    --depth=N     maximum BFS depth (default 4)
    --decompile   also decompile the functions right before the target on the paths found

Answers "does X reach Y, and through what" when backward xrefs are too broad. Indirect calls
through a vtable are not followed; use vtable.py or xrefs.py for those.
Writes callpath.txt: `path: root <- ... <- caller of target -> target` (RVAs).
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

from collections import deque

import common


def main():
    pos, flags = common.parse_args()
    if len(pos) < 2:
        common.die("need <target> and at least one <root>; see the docstring")
    depth = int(flags.get("depth", 4))
    target = common.func_start(common.parse_ea(pos[0])) or common.parse_ea(pos[0])
    roots = [common.func_start(common.parse_ea(t)) or common.parse_ea(t) for t in pos[1:]]
    parent = {r: None for r in roots}
    queue = deque((r, 0) for r in roots)
    found = []
    while queue:
        f, d = queue.popleft()
        if d >= depth:
            continue
        for c in common.callees(f):
            if c == target:
                found.append(f)
            if c not in parent:
                parent[c] = f
                queue.append((c, d + 1))
    out = common.open_out("callpath.txt")
    if not found:
        out.write("no direct call path within %d levels\n" % depth)
    for f in found:
        chain, x = [], f
        while x is not None:
            chain.append("%X" % common.rva(x))
            x = parent[x]
        out.write("path: %s -> %X\n" % (" <- ".join(chain), common.rva(target)))
        if "decompile" in flags:
            out.write("\n===== %X =====\n%s\n\n" % (common.rva(f), common.decompile(f)))
    out.close()
    print("%d paths -> %s" % (len(found), common.out_path("callpath.txt")))


common.init()
main()
common.finish()
