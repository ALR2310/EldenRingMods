"""Find callers and data references to one or more addresses, over several levels.

Usage:
    run.ps1 xrefs <ea>... [--depth=N] [--decompile]

    --depth=N     walk back N levels of callers (default 1)
    --decompile   write the decompiled code of each caller to xrefs/sub_<RVA>.c

Writes xrefs.txt: one line `d<level> <target> <- <call site> (func ...)` per code reference,
and `<-data` lines for data references (vtables, pointer tables). Addresses are RVAs.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))


import idautils

import common


def main():
    pos, flags = common.parse_args()
    if not pos:
        common.die("missing address; see the docstring")
    depth = int(flags.get("depth", 1))
    want_c = "decompile" in flags
    sub = os.path.join(common.OUT_DIR, "xrefs")
    if want_c:
        os.makedirs(sub, exist_ok=True)
    out = common.open_out("xrefs.txt")
    seen = set()
    level = [common.parse_ea(t) for t in pos]
    for d in range(depth):
        nxt = []
        for tgt in level:
            for frm, fs in common.callers(tgt):
                out.write("d%d %X <- %X (func %s %s)\n" % (
                    d, common.rva(tgt), common.rva(frm),
                    "%X" % common.rva(fs) if fs else "-", common.fn_name(fs) if fs else ""))
                if fs and fs not in seen:
                    seen.add(fs)
                    nxt.append(fs)
                    if want_c:
                        with open(os.path.join(sub, "sub_%X.c" % common.rva(fs)), "w", encoding="utf-8") as fh:
                            fh.write(common.decompile(fs))
            for x in idautils.DataRefsTo(tgt):
                out.write("d%d %X <-data %X (%s)\n" % (d, common.rva(tgt), common.rva(x), idc.get_name(x) or ""))
        level = nxt
    out.close()
    print("%d caller functions -> %s" % (len(seen), common.out_path("xrefs.txt")))


common.init()
main()
common.finish()
