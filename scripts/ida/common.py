"""Helpers shared by the IDAPython scripts in scripts/ida/.

The scripts run headless through `run.ps1` (idat -A -S"<script> <args>"). Each script does
`import common`, calls `common.init()` first and `common.finish()` last.

- Command-line arguments: `common.ARGS` (script name removed); split flags with `parse_args`.
- Output: `common.OUT_DIR` (run.ps1 sets the ER_OUT environment variable; the default is an
  `out/` folder next to the .i64 file). Scripts only read the database; they never rename
  or retype anything.
- Addresses are accepted as RVA (`1e92100`, `0x1e92100`), VA (`0x141e92100`) or a symbol
  name; see `parse_ea`.
"""
import os
import sys

import ida_auto
import ida_funcs
import ida_hexrays
import idaapi
import idautils
import idc

BADADDR = idaapi.BADADDR
BASE = idaapi.get_imagebase()
ARGS = list(idc.ARGV[1:])
OUT_DIR = os.environ.get("ER_OUT") or os.path.join(
    os.path.dirname(idc.get_idb_path()), "out"
)

_HEXRAYS = False


def init():
    """Wait for auto-analysis, enable Hex-Rays, create the output folder."""
    global _HEXRAYS
    ida_auto.auto_wait()
    _HEXRAYS = bool(ida_hexrays.init_hexrays_plugin())
    os.makedirs(OUT_DIR, exist_ok=True)


def finish(code=0):
    idc.qexit(code)


def die(msg):
    print("ERROR: " + msg)
    idc.qexit(1)


def parse_args(args=None):
    """Split `--flag` / `--key=value` from the positional arguments.

    Returns (positional, flags); `--x` gives flags['x'] = True, `--x=3` gives '3'.
    """
    pos, flags = [], {}
    for a in args if args is not None else ARGS:
        if a.startswith("--"):
            k, _, v = a[2:].partition("=")
            flags[k] = v if _ else True
        else:
            pos.append(a)
    return pos, flags


def parse_ea(tok):
    """RVA / VA / symbol name -> virtual address. A value below the image base is an RVA."""
    t = tok.strip()
    if t.lower().startswith("0x"):
        t = t[2:]
    try:
        v = int(t, 16)
        if not t.lower().startswith(("sub_", "loc_")):
            return v + BASE if v < BASE else v
    except ValueError:
        pass
    ea = idc.get_name_ea_simple(tok)
    if ea == BADADDR:
        die("cannot resolve address or name: %r" % tok)
    return ea


def rva(ea):
    return ea - BASE


def fn_name(ea):
    name = idc.get_func_name(ea)
    return name or "sub_%X" % ea


def func_start(ea):
    f = ida_funcs.get_func(ea)
    return f.start_ea if f else None


def decompile(ea):
    """Pseudocode (str), or a comment line on failure; never raises."""
    if not _HEXRAYS:
        return "// Hex-Rays is not available"
    try:
        return str(ida_hexrays.decompile(ea))
    except Exception as e:  # DecompilationFailure, or no function at ea
        return "// decompile failed @%X: %s" % (ea, e)


def callees(func_ea):
    """Entry addresses of the functions called directly from the function at func_ea."""
    f = ida_funcs.get_func(func_ea)
    res = set()
    if not f:
        return res
    for h in idautils.FuncItems(f.start_ea):
        for x in idautils.CodeRefsFrom(h, False):
            g = ida_funcs.get_func(x)
            if g and g.start_ea == x and x != f.start_ea:
                res.add(x)
    return res


def callers(ea):
    """List of (call site, entry of the containing function or None) reaching ea by code xref."""
    out = []
    for x in idautils.CodeRefsTo(ea, False):
        out.append((x, func_start(x)))
    return out


def out_path(name):
    return os.path.join(OUT_DIR, name)


def open_out(name):
    return open(out_path(name), "w", encoding="utf-8", newline="\n")
