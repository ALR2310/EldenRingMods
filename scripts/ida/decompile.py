"""Decompile one or more functions by RVA/VA/name and write decompile.txt.

Usage:
    run.ps1 decompile <ea>... [--callees] [--strings] [--max-lines=N]

    --callees     also decompile directly called functions (short ones only: fewer than
                  --max-lines lines, default 80)
    --strings     list the string literals the function references
    --max-lines=N truncate pseudocode longer than N lines (default: no truncation)
Each <ea> can be any address inside the function, for example the faulting address of a
crash dump.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import idautils

import common


def strings_in(func_ea):
    seen = []
    for h in idautils.FuncItems(func_ea):
        for x in idautils.DataRefsFrom(h):
            for kind, tag in ((idc.STRTYPE_C, "str"), (idc.STRTYPE_C_16, "wstr")):
                s = idc.get_strlit_contents(x, -1, kind)
                if s and len(s) > 3:
                    txt = s.decode("utf-8", "replace") if isinstance(s, bytes) else s
                    seen.append("  %s@%X: %s" % (tag, common.rva(h), txt[:100]))
                    break
    return seen


def main():
    pos, flags = common.parse_args()
    if not pos:
        common.die("missing address; see the docstring")
    limit = int(flags["max-lines"]) if "max-lines" in flags else None
    callee_limit = int(flags["max-lines"]) if "max-lines" in flags else 80
    done = set()
    out = common.open_out("decompile.txt")

    def dump(ea, title):
        fs = common.func_start(ea)
        if fs is None:
            out.write("// ===== %s %X: not inside any function =====\n\n" % (title, ea))
            return None
        if fs in done:
            return fs
        done.add(fs)
        text = common.decompile(fs)
        if limit:
            text = "\n".join(text.splitlines()[:limit])
        out.write("// ===== %s %s  RVA %#x  VA %#x =====\n" % (title, common.fn_name(fs), common.rva(fs), fs))
        if "strings" in flags:
            out.write("// strings referenced:\n" + "\n".join(strings_in(fs) or ["  (none)"]) + "\n")
        out.write(text + "\n\n")
        return fs

    for tok in pos:
        ea = common.parse_ea(tok)
        fs = dump(ea, tok)
        if fs is not None and "callees" in flags:
            for c in sorted(common.callees(fs)):
                if common.decompile(c).count("\n") < callee_limit:
                    dump(c, "callee of %X" % common.rva(fs))
    out.close()
    print("decompiled %d functions -> %s" % (len(done), common.out_path("decompile.txt")))


common.init()
main()
common.finish()
