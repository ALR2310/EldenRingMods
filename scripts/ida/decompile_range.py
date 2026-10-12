"""Decompile mọi hàm nằm trong 1 dải RVA, chỉ giữ hàm có chứa chuỗi cho trước.
Dùng khi biết 1 class có code nằm gọn trong 1 vùng (quanh constructor/destructor)
nhưng hàm cần tìm không đi qua vtable.

Dùng:
    run.ps1 decompile_range <rva_từ> <rva_đến> [--grep=a,b] [--max-lines=N]

    --grep=a,b    chỉ ghi hàm có pseudocode chứa MỘT trong các chuỗi (vd.
                  `+ 160)` cho offset 0xA0); bỏ trống = ghi mọi hàm
    --max-lines=N cắt pseudocode dài hơn N dòng (mặc định 200)
Ghi decompile_range.txt, mỗi hàm 1 khối; đầu file là bảng RVA / kích thước.
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
        common.die("cần <rva_từ> <rva_đến>; xem docstring")
    lo, hi = common.parse_ea(pos[0]), common.parse_ea(pos[1])
    needles = [s for s in flags.get("grep", "").split(",") if s]
    limit = int(flags.get("max-lines", 200))
    out = common.open_out("decompile_range.txt")
    funcs = [f for f in idautils.Functions(lo, hi)]
    out.write("// %d hàm trong %X..%X\n" % (len(funcs), common.rva(lo), common.rva(hi)))
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
    print("%d/%d hàm khớp -> %s" % (kept, len(funcs), common.out_path("decompile_range.txt")))


common.init()
main()
common.finish()
