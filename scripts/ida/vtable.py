"""List the vtable of a class (by RTTI name) or at an address, with its slots.

Usage:
    run.ps1 vtable <class name part | address>... [--slots=N] [--decompile[=i,j,..]]
                                                  [--rtti-scan]

    --slots=N       maximum slots per vtable (default 40; stops at the first slot that
                    does not point into a function)
    --decompile     decompile every slot (or only the indices i,j,..) into
                    vtable/<vtable>_slot<i>.c
    --rtti-scan     ignore symbol names and scan the RTTI type descriptors in
                    .data/.rdata (use when the database has no `??_7...` names)

An argument that starts with 0x, or is only hex digits (6 or more characters), is an
address; anything else is a substring of the class name (for example `CSChrBehaviorModule`).
Writes vtable.txt: `slot i  RVA  function name`.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import re

import ida_bytes
import ida_funcs
import ida_segment
import idautils

import common


def is_addr(tok):
    t = tok.lower()
    return t.startswith("0x") or (len(t) >= 6 and re.fullmatch(r"[0-9a-f]+", t) is not None)


def slots_of(vt, limit):
    res = []
    for i in range(limit):
        p = ida_bytes.get_qword(vt + i * 8)
        if not ida_funcs.get_func(p):
            break
        res.append(p)
    return res


def vtables_by_name(sub):
    for ea, name in idautils.Names():
        if name.startswith("??_7") and sub.lower() in name.lower():
            yield ea, idc.demangle_name(name, idc.get_inf_attr(idc.INF_SHORT_DN)) or name


def vtables_by_rtti(sub):
    """Scan '.?AV<...sub...>' -> COMPLETE_OBJECT_LOCATOR -> vtable (MSVC x64 RTTI layout)."""
    rx = re.compile(rb"\.\?A[VU][A-Za-z0-9_@?$]*" + re.escape(sub.encode()) + rb"[A-Za-z0-9_@?$]*", re.I)
    segs = [(s, idc.get_segm_end(s)) for s in idautils.Segments() if idc.get_segm_name(s) in (".data", ".rdata")]
    rdata = {s: ida_bytes.get_bytes(s, e - s) or b"" for s, e in segs if idc.get_segm_name(s) == ".rdata"}

    def find(value, size):
        needle = value.to_bytes(size, "little")
        for s, buf in rdata.items():
            i = buf.find(needle)
            while i != -1:
                yield s + i
                i = buf.find(needle, i + 1)

    for s, e in segs:
        buf = ida_bytes.get_bytes(s, e - s) or b""
        for m in rx.finditer(buf):
            td = s + m.start() - 0x10
            for x in find(td - common.BASE, 4):
                col = x - 12
                if ida_bytes.get_dword(col) != 1:
                    continue
                for ref in find(col, 8):
                    yield ref + 8, m.group(0).decode()


def main():
    pos, flags = common.parse_args()
    if not pos:
        common.die("missing class name or address; see the docstring")
    limit = int(flags.get("slots", 40))
    want = flags.get("decompile")
    only = None if want in (None, True) else {int(x) for x in str(want).split(",")}
    sub = os.path.join(common.OUT_DIR, "vtable")
    if want:
        os.makedirs(sub, exist_ok=True)
    out = common.open_out("vtable.txt")
    n = 0
    for tok in pos:
        if is_addr(tok):
            found = [(common.parse_ea(tok), tok)]
        else:
            found = [] if "rtti-scan" in flags else list(vtables_by_name(tok))
            if not found:
                found = list(vtables_by_rtti(tok))
        if not found:
            out.write("== %s: no vtable found\n" % tok)
        for vt, name in found:
            n += 1
            out.write("== %s  vtable RVA %#x\n" % (name, common.rva(vt)))
            for i, f in enumerate(slots_of(vt, limit)):
                out.write("slot %-3d %X  %s\n" % (i, common.rva(f), common.fn_name(f)))
                if want and (only is None or i in only):
                    safe = re.sub(r"[^A-Za-z0-9_]", "_", name)[:40]
                    with open(os.path.join(sub, "%s_slot%d.c" % (safe, i)), "w", encoding="utf-8") as fh:
                        fh.write(common.decompile(f))
    out.close()
    print("%d vtable -> %s" % (n, common.out_path("vtable.txt")))


common.init()
main()
common.finish()
