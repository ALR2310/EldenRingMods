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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("module")
    ap.add_argument("--proc", default="eldenring.exe")
    ap.add_argument("--out", default="dumps/ersc/fcd11a18/unpacked")
    a = ap.parse_args()
    pid = find_pid(a.proc)
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
