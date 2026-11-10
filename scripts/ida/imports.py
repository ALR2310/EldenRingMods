"""List the exe's imports that match a substring, with their callers.

Usage:
    run.ps1 imports <string>... [--noxrefs]

    <string>      name (or DLL) containing this string, case-insensitive
    --noxrefs     list the imports only, do not look up callers

Writes imports.txt: `<DLL> <name> @<RVA of the import slot>` followed by lines
`  <- <RVA of call site> (func <RVA of function start>)`. Imports by ordinal print as `ord<N>`.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import ida_nalt
import idautils

import common


def main():
    pos, flags = common.parse_args()
    if not pos:
        common.die("missing search string; see the docstring")
    want = [p.lower() for p in pos]
    xrefs = "noxrefs" not in flags
    out = common.open_out("imports.txt")
    for i in range(ida_nalt.get_import_module_qty()):
        dll = ida_nalt.get_import_module_name(i) or "?"
        rows = []

        def cb(ea, name, ordinal):
            rows.append((ea, name or ("ord%d" % ordinal)))
            return True

        ida_nalt.enum_import_names(i, cb)
        for ea, name in rows:
            hay = (dll + " " + name).lower()
            if not any(w in hay for w in want):
                continue
            out.write("%s %s @%X\n" % (dll, name, common.rva(ea)))
            if xrefs:
                for frm, fs in common.callers(ea):
                    out.write(
                        "  <- %X (func %s)\n"
                        % (common.rva(frm), "%X" % common.rva(fs) if fs else "?")
                    )
    out.close()
    common.finish()


main()
