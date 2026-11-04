"""Decompile every function in an RVA range, keeping only those that contain given strings.
Useful when a class's code sits in one region (around its constructor and destructor) but the
function you need is not reached through the vtable.

Usage:
    run.ps1 decompile_range <rva_from> <rva_to> [--grep=a,b] [--max-lines=N]

    --grep=a,b    write only functions whose pseudocode contains ANY of the strings (for
                  example `+ 160)` for offset 0xA0); empty means every function
    --max-lines=N truncate pseudocode longer than N lines (default 200)
Writes decompile_range.txt with one block per function, after a header line with the count.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import ida_funcs
import idautils

import common


def main():
    pos, flags = common.parse_args()
    if len(pos) != 2:
        common.die("need <rva_from> <rva_to>; see the docstring")
    lo, hi = common.parse_ea(pos[0]), common.parse_ea(pos[1])
    needles = [s for s in flags.get("grep", "").split(",") if s]
    limit = int(flags.get("max-lines", 200))
    out = common.open_out("decompile_range.txt")
    funcs = [f for f in idautils.Functions(lo, hi)]
    out.write("// %d functions in %X..%X\n" % (len(funcs), common.rva(lo), common.rva(hi)))
    kept = 0
    for ea in funcs:
        f = ida_funcs.get_func(ea)
        text = common.decompile(ea)
        if needles and not any(n in text for n in needles):
            continue
        kept += 1
        text = "\n".join(text.splitlines()[:limit])
        out.write("// ===== %s  RVA %#x  size %#x =====\n%s\n\n" % (common.fn_name(ea), common.rva(ea), f.end_ea - f.start_ea, text))
    out.close()
    print("%d/%d functions matched -> %s" % (kept, len(funcs), common.out_path("decompile_range.txt")))


common.init()
main()
common.finish()
