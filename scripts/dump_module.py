"""Dump a module from a running process (e.g. ersc.dll, after Themida unpacked it).

    python -I scripts/dump_module.py ersc.dll [--proc eldenring.exe] [--out DIR]

Writes <name>.mem.bin (raw image, unreadable pages zero-filled) and
<name>.dumped.dll (PE with raw layout = memory layout, ImageBase = load base,
so IDA sees the relocated, decrypted code). Imports are not rebuilt: the IAT
holds resolved pointers. Run as the same user as the game, once in the world.
"""
import argparse
import ctypes
import ctypes.wintypes as wt
import pathlib

import pefile

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
k32.OpenProcess.restype = wt.HANDLE
k32.CreateToolhelp32Snapshot.restype = wt.HANDLE
k32.ReadProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_void_p, ctypes.c_size_t, ctypes.POINTER(ctypes.c_size_t)]

TH32CS_SNAPPROCESS, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32 = 2, 8, 0x10
PROCESS_VM_READ, PROCESS_QUERY_INFORMATION = 0x10, 0x400


class PROCESSENTRY32W(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("cntUsage", wt.DWORD), ("th32ProcessID", wt.DWORD),
                ("th32DefaultHeapID", ctypes.c_void_p), ("th32ModuleID", wt.DWORD), ("cntThreads", wt.DWORD),
                ("th32ParentProcessID", wt.DWORD), ("pcPriClassBase", wt.LONG), ("dwFlags", wt.DWORD),
                ("szExeFile", wt.WCHAR * 260)]


class MODULEENTRY32W(ctypes.Structure):
    _fields_ = [("dwSize", wt.DWORD), ("th32ModuleID", wt.DWORD), ("th32ProcessID", wt.DWORD),
                ("GlblcntUsage", wt.DWORD), ("ProccntUsage", wt.DWORD), ("modBaseAddr", ctypes.c_void_p),
                ("modBaseSize", wt.DWORD), ("hModule", ctypes.c_void_p), ("szModule", wt.WCHAR * 256),
                ("szExePath", wt.WCHAR * 260)]


def find_pid(name):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    e = PROCESSENTRY32W(); e.dwSize = ctypes.sizeof(e)
    ok = k32.Process32FirstW(snap, ctypes.byref(e))
    while ok:
        if e.szExeFile.lower() == name.lower():
            k32.CloseHandle(snap)
            return e.th32ProcessID
        ok = k32.Process32NextW(snap, ctypes.byref(e))
    k32.CloseHandle(snap)
    raise SystemExit(f"process {name} not running")


def find_module(pid, name):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid)
    e = MODULEENTRY32W(); e.dwSize = ctypes.sizeof(e)
    ok = k32.Module32FirstW(snap, ctypes.byref(e))
    while ok:
        if e.szModule.lower() == name.lower():
            k32.CloseHandle(snap)
            return e.modBaseAddr, e.modBaseSize
        ok = k32.Module32NextW(snap, ctypes.byref(e))
    k32.CloseHandle(snap)
    raise SystemExit(f"module {name} not loaded in pid {pid}")


def read_image(h, base, size):
    buf = bytearray(size)
    bad = 0
    for off in range(0, size, 0x1000):
        n = min(0x1000, size - off)
        tmp = (ctypes.c_char * n)()
        got = ctypes.c_size_t(0)
        if k32.ReadProcessMemory(h, base + off, tmp, n, ctypes.byref(got)) and got.value == n:
            buf[off:off + n] = tmp.raw
        else:
            bad += 1
    return buf, bad


def fix_pe(buf, base):
    pe = pefile.PE(data=bytes(buf), fast_load=True)
    pe.OPTIONAL_HEADER.ImageBase = base
    for s in pe.sections:
        s.PointerToRawData = s.VirtualAddress
        s.SizeOfRawData = (max(s.Misc_VirtualSize, s.SizeOfRawData) + 0xFFF) & ~0xFFF
    # relocations are already applied in memory
    pe.FILE_HEADER.Characteristics |= 0x0001  # IMAGE_FILE_RELOCS_STRIPPED
    return pe.write()


def list_modules(pid):
    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid)
    e = MODULEENTRY32W(); e.dwSize = ctypes.sizeof(e)
    mods = []
    ok = k32.Module32FirstW(snap, ctypes.byref(e))
    while ok:
        mods.append((e.modBaseAddr, e.modBaseSize, e.szModule))
        ok = k32.Module32NextW(snap, ctypes.byref(e))
    k32.CloseHandle(snap)
    return mods


def owner(mods, addr):
    for b, s, n in mods:
        if b <= addr < b + s:
            return f"{n}+{addr - b:#x}"
    return "unmapped/anon"


def peek(h, pid, module, rva, hops=4):
    """Print bytes at module+rva and follow E9 / FF 25 / mov rax,imm64; jmp rax chains."""
    import struct
    mods = list_modules(pid)
    base, _ = find_module(pid, module)
    addr = base + rva
    for i in range(hops):
        tmp = (ctypes.c_char * 32)()
        got = ctypes.c_size_t(0)
        if not (k32.ReadProcessMemory(h, addr, tmp, 32, ctypes.byref(got)) and got.value == 32):
            print(f"hop {i}: {addr:#x} ({owner(mods, addr)}) unreadable"); return
        b = tmp.raw
        print(f"hop {i}: {addr:#x} ({owner(mods, addr)}): {b.hex(' ')}")
        if b[0] == 0xE9:
            addr = addr + 5 + struct.unpack_from("<i", b, 1)[0]
        elif b[:2] == b"\xff\x25":
            p = addr + 6 + struct.unpack_from("<i", b, 2)[0]
            t = (ctypes.c_char * 8)()
            if not k32.ReadProcessMemory(h, p, t, 8, ctypes.byref(got)): return
            addr = struct.unpack("<Q", t.raw)[0]
        elif b[:2] == b"\x48\xb8" and b[10:12] == b"\xff\xe0":
            addr = struct.unpack_from("<Q", b, 2)[0]
        elif b[:2] == b"\x48\xb8" and b[10:15] == b"\x66\x48\x0f\x6e\xe8" and b[15:17] == b"\x49\xbb":
            # ersc thunk: mov rax,ctx; movq xmm5,rax; mov r11,target; jmp r11
            ctx = struct.unpack_from("<Q", b, 2)[0]
            target = struct.unpack_from("<Q", b, 17)[0]
            print(f"  ctx {ctx:#x}, target {target:#x} ({owner(mods, target)})")
            follow_ctx(h, mods, ctx)
            return
        else:
            return


def read_q(h, addr):
    import struct
    t = (ctypes.c_char * 8)()
    got = ctypes.c_size_t(0)
    if k32.ReadProcessMemory(h, addr, t, 8, ctypes.byref(got)) and got.value == 8:
        return struct.unpack("<Q", t.raw)[0]
    return None


def follow_ctx(h, mods, ctx, off=56, slot=16):
    cb = read_q(h, ctx + off)
    print(f"  [ctx+{off}] = {cb and hex(cb)} ({cb and owner(mods, cb)})")
    if not cb:
        return
    vt = read_q(h, cb)
    print(f"  vtable = {vt and hex(vt)} ({vt and owner(mods, vt)})")
    if not vt:
        return
    for s in (0, 8, 16, 24):
        fn = read_q(h, vt + s)
        print(f"    vtable+{s} = {fn and hex(fn)} ({fn and owner(mods, fn)})")
    for o in (8, 16):
        v = read_q(h, cb + o)
        print(f"  [obj+{o}] = {v and hex(v)} ({v and owner(mods, v)})")
    # ersc ghost-colour handler sub_180083260(a1 = [obj+8]): v3 = [a1+72]; flag byte = [v3+152]+641 & 0x40
    a1 = read_q(h, cb + 8)
    v3 = a1 and read_q(h, a1 + 72)
    if v3:
        s = read_q(h, v3 + 152)
        fl = None
        if s:
            t = (ctypes.c_char * 1)(); g = ctypes.c_size_t(0)
            if k32.ReadProcessMemory(h, s + 641, t, 1, ctypes.byref(g)) and g.value == 1:
                fl = t.raw[0]
        orig = read_q(h, v3 + 51 * 8)
        print(f"  a1 {a1:#x}, v3 {v3:#x}, settings [v3+152] {s and hex(s)}, flag byte +641 = {fl is not None and hex(fl)}"
              f" (&0x40 = {fl is not None and bool(fl & 0x40)}), original fn [v3+408] = {orig and hex(orig)} ({orig and owner(mods, orig)})")


def find_world_chr_man(h, base, size):
    """AOB mov rax,[rip+d]; test rax,rax; jz +0F; cmp [rax+88],ecx -> address of the WorldChrMan global."""
    import re
    import struct
    pat = re.compile(rb"\x48\x8b\x05....\x48\x85\xc0\x74\x0f\x48\x39\x88", re.S)
    chunk = 0x1000000
    for off in range(0, size, chunk):
        n = min(chunk + 0x20, size - off)
        buf, _ = read_image(h, base + off, n)
        m = pat.search(bytes(buf))
        if m:
            at = base + off + m.start()
            disp = struct.unpack_from("<i", buf, m.start() + 3)[0]
            return at, at + 7 + disp
    return None, None


def chr_dump(h, pid):
    """Read the fields Seamless' ghost-colour hook compares (sub_180028700)."""
    mods = list_modules(pid)
    base, size = find_module(pid, "eldenring.exe")
    at, glob = find_world_chr_man(h, base, size)
    if not glob:
        raise SystemExit("WorldChrMan AOB not found")
    wcm = read_q(h, glob)
    print(f"AOB at exe+{at - base:#x}, global exe+{glob - base:#x}, WorldChrMan {wcm:#x}")
    if not wcm:
        raise SystemExit("WorldChrMan null (not in world)")

    def q(a):
        return read_q(h, a)

    def d(a):
        t = (ctypes.c_char * 4)(); g = ctypes.c_size_t(0)
        if k32.ReadProcessMemory(h, a, t, 4, ctypes.byref(g)) and g.value == 4:
            import struct
            return struct.unpack("<I", t.raw)[0]
        return None

    main = q(wcm + 124168)
    print(f"main player [wcm+0x1E508] = {main and hex(main)}")
    print(f"+69352 (team id A) = {d(wcm + 69352)}, +69528 (team id B) = {d(wcm + 69528)}")
    cnt = d(wcm + 69360)
    arr = q(wcm + 69368)
    print(f"player array: count {cnt}, base {arr and hex(arr)}")
    for i in range(min(cnt or 0, 8)):
        p = q(arr + 16 * i)
        if not p:
            print(f"  [{i}] null"); continue
        mod = q(p + 1408)
        print(f"  [{i}] ChrIns {p:#x} handle(+8)={q(p+8) and hex(q(p+8))} "
              f"+536={q(p+536) and hex(q(p+536))} +1456={q(p+1456) and hex(q(p+1456))}"
              f" module(+1408)={mod and hex(mod)} [mod+2384]={mod and q(mod+2384) and hex(q(mod+2384))}"
              f"{'  <- main' if p == main else ''}")
    buddy = q(wcm + 124216)
    print(f"buddy list obj [wcm+124216] = {buddy and hex(buddy)}")
    if buddy:
        for o in range(0, 0x60, 8):
            v = q(buddy + o)
            print(f"  +{o:#04x}: {v and hex(v)} ({v and owner(mods, v)})")


class MEMORY_BASIC_INFORMATION(ctypes.Structure):
    _fields_ = [("BaseAddress", ctypes.c_void_p), ("AllocationBase", ctypes.c_void_p), ("AllocationProtect", wt.DWORD),
                ("PartitionId", wt.WORD), ("RegionSize", ctypes.c_size_t), ("State", wt.DWORD),
                ("Protect", wt.DWORD), ("Type", wt.DWORD)]


def scan_handles(h, pid, handles, set1344=None):
    """Scan committed private RW memory for 8-byte values (ChrIns+8 = handle) and print candidate ChrIns."""
    import struct
    k32.VirtualQueryEx.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.POINTER(MEMORY_BASIC_INFORMATION), ctypes.c_size_t]
    want = {struct.pack("<Q", v): v for v in handles}
    mods = list_modules(pid)
    addr, found = 0, []
    mbi = MEMORY_BASIC_INFORMATION()
    while addr < 0x7FFFFFFFFFFF:
        if not k32.VirtualQueryEx(h, addr, ctypes.byref(mbi), ctypes.sizeof(mbi)):
            break
        base, size = mbi.BaseAddress or 0, mbi.RegionSize
        if mbi.State == 0x1000 and mbi.Type == 0x20000 and mbi.Protect in (0x04, 0x40) and size < (1 << 30):
            buf, _ = read_image(h, base, size)
            b = bytes(buf)
            for key, v in want.items():
                i = b.find(key)
                while i != -1:
                    if i % 8 == 0:
                        found.append((v, base + i - 8))
                    i = b.find(key, i + 8)
        addr = base + size
    for v, c in found:
        q = lambda o: read_q(h, c + o)
        vt = q(0)
        def d(o):
            t = (ctypes.c_char * 4)(); g = ctypes.c_size_t(0)
            if k32.ReadProcessMemory(h, c + o, t, 4, ctypes.byref(g)) and g.value == 4:
                return struct.unpack("<i", t.raw)[0]
        m400 = q(400)
        m184 = m400 and read_q(h, m400 + 184)
        t68 = None
        if m184:
            t = (ctypes.c_char * 4)(); g = ctypes.c_size_t(0)
            if k32.ReadProcessMemory(h, m184 + 68, t, 4, ctypes.byref(g)) and g.value == 4:
                t68 = struct.unpack("<i", t.raw)[0]
        print(f"handle {v:#x}: candidate ChrIns {c:#x} vtable {vt and hex(vt)} ({vt and owner(mods, vt)}) "
              f"+536={q(536) and hex(q(536))} +1408={q(1408) and hex(q(1408))} +1456={q(1456) and hex(q(1456))} "
              f"+1344={d(1344)} +104={d(104)} +524={d(524)} chain[+400][+184]+68={t68}")
        if set1344 is not None and vt and owner(mods, vt).endswith("+0x2a49090"):
            buf = struct.pack("<i", set1344)
            written = ctypes.c_size_t(0)
            k32.WriteProcessMemory.argtypes = [wt.HANDLE, ctypes.c_void_p, ctypes.c_char_p, ctypes.c_size_t,
                                               ctypes.POINTER(ctypes.c_size_t)]
            ok = k32.WriteProcessMemory(h, c + 1344, buf, 4, ctypes.byref(written))
            print(f"  -> wrote {set1344} to +1344: {bool(ok) and written.value == 4}, now {d(1344)}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("module")
    ap.add_argument("--proc", default="eldenring.exe")
    ap.add_argument("--out", default="dumps/ersc/fcd11a18/unpacked")
    ap.add_argument("--peek", metavar="RVA", help="hex RVA in MODULE: print bytes and follow the jump chain, then exit")
    ap.add_argument("--chr", action="store_true", help="print WorldChrMan player/buddy fields, then exit")
    ap.add_argument("--handles", metavar="HEX,HEX", help="scan heap for ChrIns with these handles (+8), then exit")
    ap.add_argument("--set1344", type=int, metavar="N", help="with --handles: write int32 N to +1344 (phantom override) of the matches")
    a = ap.parse_args()
    pid = find_pid(a.proc)
    if a.handles:
        access = PROCESS_VM_READ | PROCESS_QUERY_INFORMATION
        if a.set1344 is not None:
            access |= 0x20 | 0x8  # VM_WRITE | VM_OPERATION
        h = k32.OpenProcess(access, False, pid)
        if not h:
            raise SystemExit(f"OpenProcess failed ({ctypes.get_last_error()})")
        scan_handles(h, pid, [int(x, 16) for x in a.handles.split(",")], a.set1344)
        k32.CloseHandle(h)
        return
    if a.chr:
        h = k32.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False, pid)
        if not h:
            raise SystemExit(f"OpenProcess failed ({ctypes.get_last_error()})")
        chr_dump(h, pid)
        k32.CloseHandle(h)
        return
    if a.peek:
        h = k32.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False, pid)
        if not h:
            raise SystemExit(f"OpenProcess failed ({ctypes.get_last_error()})")
        peek(h, pid, a.module, int(a.peek, 16))
        k32.CloseHandle(h)
        return
    base, size = find_module(pid, a.module)
    print(f"pid {pid}, {a.module} base {base:#x} size {size:#x}")
    h = k32.OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, False, pid)
    if not h:
        raise SystemExit(f"OpenProcess failed ({ctypes.get_last_error()}); run as the game's user/admin")
    buf, bad = read_image(h, base, size)
    k32.CloseHandle(h)
    print(f"unreadable pages: {bad}; MZ header: {bytes(buf[:2]) == b'MZ'}")
    out = pathlib.Path(a.out); out.mkdir(parents=True, exist_ok=True)
    stem = pathlib.Path(a.module).stem
    (out / f"{stem}.mem.bin").write_bytes(buf)
    if bytes(buf[:2]) == b"MZ":
        (out / f"{stem}.dumped.dll").write_bytes(fix_pe(buf, base))
        print(f"wrote {out / (stem + '.dumped.dll')}")
    print(f"wrote {out / (stem + '.mem.bin')}")


if __name__ == "__main__":
    main()
