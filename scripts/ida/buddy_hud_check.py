"""Kiểm các chỗ patch của `HubHpBars` (mods/spiritmultiplier/src/buddy_hud.rs) trên một bản exe.

Dùng:
    run.ps1 buddy_hud_check -Version all

Mỗi mẫu AOB trong buddy_hud.rs phải UNIQUE, và điểm đến của từng `call`/`jmp`
phải đúng quan hệ giữa các hàm (không chỉ khớp byte):
  - lệnh gọi Blob nằm trong chính hàm mà chỗ gọi list builder trỏ tới;
  - hàm view update chứa cặp `lea` cửa sổ slot (header 136 byte, 5 slot);
  - constructor view tham chiếu string "Anim/GaugeList" và một vtable có slot 2
    là wrapper `add rcx,8; jmp` của hàm thêm hàng;
  - constructor view dựng component gắn với clip "Anim" ở view+776 (0x308);
  - `sub_140D872C0` (đặt `_x/_y` của clip) duy nhất, và được gọi từ hàm đặt vị trí component;
  - hàm mở tệp có slot con trỏ trong dữ liệu, mẫu bắt đầu từ byte thứ 17.
Mỗi dòng `RESULT OK|FAIL <tên> ...`; sửa mẫu ở đây khi sửa buddy_hud.rs.
"""
import os
import sys

import idc

sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))

import ida_bytes
import ida_funcs
import idautils

import common

BLOB = "84 C0 74 ?? 48 8B D6 49 8B CE E8 ?? ?? ?? ?? 84 C0 75 ?? 83 FF 05"
BUILD = "E8 ?? ?? ?? ?? 0F 28 CE 48 8B CB E8 ?? ?? ?? ?? 0F 28 CE 48 8B CB E8 ?? ?? ?? ?? 4C 8B C7"
VIEW = ("48 81 C2 ?? ?? 00 00 48 8D 8E ?? ?? ?? ?? E8 ?? ?? ?? ?? 4C 8B 46 60 49 81 C0 ?? ?? 00 00 "
        "48 8D 8E ?? ?? ?? ?? 0F 28 CE E8 ?? ?? ?? ??")
LAYOUT = "49 8D BF 88 00 00 00 41 BC FF FF FF FF 48 8D 77 78 48 3B FE"
CTOR = "4C 8D 05 ?? ?? ?? ?? 48 8D 54 24 ?? 49 8B CE E8 ?? ?? ?? ?? 90 49 8D 8E ?? ?? ?? ?? 48 8B D0 E8 ?? ?? ?? ??"
ITEM = ("40 57 48 83 EC 30 48 C7 44 24 20 FE FF FF FF 48 89 5C 24 48 48 89 74 24 50 48 8B DA 48 8B F1 "
        "48 8B 01 48 83 B8 B0 03 00 00 05 0F 83")
SET_POS = ("48 8B C4 48 89 58 10 55 48 8D 68 C8 48 81 EC 30 01 00 00 0F 29 70 E8 0F 29 78 D8 48 8B 05 ?? ?? ?? ?? "
           "48 33 C4 48 89 45 00 F6 41 20 8F 0F 28 F2 0F 28 F9 48 8B D9 0F 84 ?? ?? ?? ??")
OPENER = ("48 81 EC B0 00 00 00 48 C7 45 B7 FE FF FF FF 48 8B 05 ?? ?? ?? ?? 48 33 C4 48 89 45 1F 45 8B F9 45 8B F0 "
          "48 8B DA 48 8B F1 48 8B 0D ?? ?? ?? ?? 48 85 C9 75 0F E8")

out = []


def report(ok, name, detail=""):
    out.append("RESULT %s %s %s" % ("OK" if ok else "FAIL", name, detail))


def find(pattern):
    patt = ida_bytes.compiled_binpat_vec_t()
    ida_bytes.parse_binpat_str(patt, common.BASE, pattern.replace("??", "?"), 16)
    flags = ida_bytes.BIN_SEARCH_FORWARD | ida_bytes.BIN_SEARCH_NOBREAK | ida_bytes.BIN_SEARCH_NOSHOW
    hits = []
    for seg in idautils.Segments():
        ea, end = seg, idc.get_segm_end(seg)
        while ea < end:
            r = ida_bytes.bin_search(ea, end, patt, flags)
            ea = r[0] if isinstance(r, tuple) else r
            if ea == common.BADADDR or ea >= end:
                break
            hits.append(ea)
            ea += 1
    return hits


def unique(name, pattern):
    hits = find(pattern)
    report(len(hits) == 1, name + " unique", "%d match(es) %s" % (len(hits), ["%X" % common.rva(h) for h in hits]))
    return hits[0] if len(hits) == 1 else None


def call_target(site):
    """RVA-agnostic destination of the `call`/`jmp rel32` at `site`."""
    if idc.get_wide_byte(site) not in (0xE8, 0xE9):
        return None
    return (site + 5 + idc.get_wide_dword(site + 1) - (1 << 32 if idc.get_wide_dword(site + 1) >> 31 else 0)) & 0xFFFFFFFFFFFFFFFF


def fn(ea):
    f = ida_funcs.get_func(ea)
    return f.start_ea if f else None


def label(ea):
    return "%X" % common.rva(ea) if ea is not None else "none"


def main():
    blob = unique("blob", BLOB)
    build = unique("build", BUILD)
    view = unique("view", VIEW)
    layout = unique("layout", LAYOUT)
    ctor = unique("ctor", CTOR)
    item = unique("item", ITEM)
    opener = unique("opener", OPENER)
    set_pos = unique("set_pos", SET_POS)

    # blob call lives in the function the build call leads to
    if blob is not None and build is not None:
        build_target = call_target(build + 11)
        blob_fn = fn(blob + 10)
        report(build_target is not None and build_target == blob_fn, "blob-in-build",
               "build->%s blob in %s" % (label(build_target), label(blob_fn)))
        report(call_target(blob + 10) is not None, "blob call", "-> %s" % label(call_target(blob + 10)))

    # view update call leads to the function holding the slot window
    if view is not None and layout is not None:
        view_target = call_target(view + 40)
        report(view_target is not None and view_target == fn(layout), "layout-in-view",
               "view->%s layout in %s" % (label(view_target), label(fn(layout))))

    # constructor: gauge-list string, and a vtable whose slot 2 is the row wrapper
    wrapper = None
    if item is not None:
        wrappers = []
        for site in idautils.CodeRefsTo(item, False):
            if idc.get_wide_byte(site) == 0xE9 and ida_bytes.get_bytes(site - 4, 4) == b"\x48\x83\xC1\x08":
                wrappers.append(site)
        report(len(wrappers) == 1, "item wrapper", "%d jmp(s) %s" % (len(wrappers), [label(w) for w in wrappers]))
        wrapper = fn(wrappers[0]) if len(wrappers) == 1 else None
    if ctor is not None:
        ctor_target = call_target(ctor + 31)
        report(ctor_target is not None, "ctor call", "-> %s" % label(ctor_target))
        if ctor_target is not None:
            strings = [s for s in idautils.Strings() if s.ea and str(s) == "Anim/GaugeList"]
            hit = any(fn(x.frm) == ctor_target for s in strings for x in idautils.XrefsTo(s.ea, 0))
            report(hit, "ctor uses Anim/GaugeList")
            slot2 = False
            for h in idautils.FuncItems(ctor_target):
                for ref in idautils.DataRefsFrom(h):
                    if wrapper is not None and ida_bytes.get_qword(ref + 16) == wrapper:
                        slot2 = True
            report(slot2, "ctor vtable slot 2 is the row wrapper", "wrapper %s" % label(wrapper))

    # the "Anim" component sits at view+776 in the constructor
    if ctor is not None and call_target(ctor + 31) is not None:
        ctor_target = call_target(ctor + 31)
        hits = [h for h in idautils.FuncItems(ctor_target) if "308h" in idc.GetDisasm(h) and "rcx" in idc.GetDisasm(h)]
        report(len(hits) >= 1, "ctor places the Anim component at +0x308", "%d instruction(s)" % len(hits))

    # the set-position function is what the component position setter calls
    if set_pos is not None:
        callers = [fn(r) for r in idautils.CodeRefsTo(set_pos, False)]
        report(len(callers) >= 1, "set_pos is called", "%d caller(s)" % len(callers))

    # opener: starts 16 bytes before the pattern, referenced from data
    if opener is not None:
        start = opener - 16
        report(fn(start) == start, "opener starts a function", label(start))
        refs = [r for r in idautils.DataRefsTo(start) if ida_bytes.is_data(ida_bytes.get_full_flags(r))]
        report(len(refs) >= 1, "opener has data slot(s)", "%d: %s" % (len(refs), [label(r) for r in refs]))

    path = common.out_path("buddy_hud_check.txt")
    with open(path, "w", encoding="utf-8") as f:
        f.write("\n".join(out) + "\n")
    print("\n".join(out))


common.init()
main()
common.finish()
