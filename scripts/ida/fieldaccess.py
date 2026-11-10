"""Find every [reg+offset] access with a given offset, that is every function that reads or
writes one struct field (speed, HP, a flag...) when the offset is known from a crash dump or a
memory scanner.

Usage:
    run.ps1 fieldaccess <hex offset>... [--float] [--limit=N]

    --float      only SSE float instructions (movss/mulss/comiss/...), less noise when the
                 offset is small and common
    --limit=N    print at most N instructions (default 500)

Scans every function, so it is slow (a few minutes). Writes fieldaccess.txt, one line each:
`+offset  func=RVA  ea=RVA  function name | instruction`, sorted by offset then address.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import ida_funcs
import ida_ua
import idautils

import common

FLOAT_MN = {"movss", "mulss", "divss", "addss", "subss", "comiss", "ucomiss", "maxss", "minss",
            "cvtss2sd", "cvtsi2ss", "movd", "movaps", "movups", "shufps"}


def main():
    pos, flags = common.parse_args()
    if not pos:
        common.die("missing offset; see the docstring")
    targets = {int(p, 16) for p in pos}
    limit = int(flags.get("limit", 500))
    only_float = "float" in flags
    insn = ida_ua.insn_t()
    hits = []
    for fea in idautils.Functions():
        for ea in idautils.FuncItems(fea):
            if ida_ua.decode_insn(insn, ea) == 0:
                continue
            for op in insn.ops:
                if op.type == ida_ua.o_void:
                    break
                if op.type == ida_ua.o_displ and op.addr in targets:
                    if only_float and insn.get_canon_mnem() not in FLOAT_MN:
                        break
                    hits.append((op.addr, fea, ea, idc.GetDisasm(ea)))
                    break
    hits.sort()
    out = common.open_out("fieldaccess.txt")
    for off, fs, ea, dis in hits[:limit]:
        out.write("+%X  func=%X  ea=%X  %s | %s\n" % (off, common.rva(fs), common.rva(ea), common.fn_name(fs), dis))
    if len(hits) > limit:
        out.write("... %d more instructions (raise --limit)\n" % (len(hits) - limit))
    out.close()
    print("%d matching instructions -> %s" % (len(hits), common.out_path("fieldaccess.txt")))


common.init()
main()
common.finish()
