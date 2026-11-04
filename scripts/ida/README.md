# IDAPython tools for `eldenring.exe`

Reusable scripts that query an existing IDA database
(`dumps/eldenring/<version>/eldenring.exe.i64`) for the questions that keep coming up while
writing mods: what does this function do, who calls it, is this AOB unique in every exe
version, who reads or writes the field at `+0x17C8`. A run takes about 15 seconds (it opens
the database, it does not re-analyze).

`dumps/` is a local folder that is not tracked by git. Put your own databases there. For
Ghidra, a few general scripts are in `scripts/ghidra/`.

## Running

```powershell
pwsh scripts/ida/run.ps1 <script> [-Version <ver|all>] <arguments...>
```

- `-Version`: a folder in `dumps/eldenring/` (for example `2.7.1.0`); `all` runs every
  version in turn; omitted means the latest version.
- Output goes to `dumps/eldenring/<version>/out/` (with a `<script>.log` file). Addresses in
  the output are **RVAs**.
- Addresses you pass can be an RVA (`1e92100`), a VA (`0x141e92100`) or a symbol name. An
  address in the middle of a function is fine (for example a crash address `exe+0x1E92100`).
- Close the IDA GUI first: a database cannot be open in two places. `idat` is taken from the
  `ER_IDAT` environment variable, otherwise from `PATH`.
- Scripts only read the database; they do not rename or retype anything. idat still writes
  the `.i64` file back on exit (about 1.4 GB, the size barely changes).

## Tools

| Script | Use | Example |
|---|---|---|
| `decompile.py` | Decompile functions, with short callees and referenced strings | `run.ps1 decompile 1e92100 --callees --strings` |
| `disasm.py` | Disassemble with raw bytes (build an AOB, pick a patch length) | `run.ps1 disasm 25e0e0:30` or `... --func` |
| `xrefs.py` | Callers and data references, several levels | `run.ps1 xrefs 25e0e0 --depth=2 --decompile` |
| `imports.py` | Imports matching a string (DLL or name), with call sites | `run.ps1 imports dinput xinput GetForegroundWindow` |
| `strings.py` | Find functions from a string (ASCII and UTF-16) | `run.ps1 strings "Warping to" --decompile` |
| `aob.py` | Is an AOB pattern unique, and in which exe versions | `run.ps1 aob -Version all "48 8B 05 ?? ?? ?? ?? 48 85 C0"` |
| `vtable.py` | Vtable slots by RTTI class name or address | `run.ps1 vtable CSChrBehaviorModule --slots=40` |
| `callpath.py` | Is there a direct call path from A to B | `run.ps1 callpath <target> <root>... --depth=4` |
| `fieldaccess.py` | Every `[reg+offset]` access (about 1 minute) | `run.ps1 fieldaccess 17c8 --float` |
| `switches.py` | Large switches in a call tree (ID dispatchers) | `run.ps1 switches <root> --depth=3 --min-cases=15` |
| `decompile_range.py` | Decompile every function in an RVA range, filtered by strings (for functions not reached through a vtable) | `run.ps1 decompile_range 406000 40A000 --grep=a,b` |
| `buddy_hud_check.py` | Checks the patch sites of SpiritMultiplier's `HubHpBars` (patterns unique, function relations) | `run.ps1 buddy_hud_check -Version all` |
| `export_all.py` | Export the whole exe to text for grep (very heavy, hours) | `run.ps1 export_all --asm` |

Quote arguments that contain spaces (AOB patterns, search strings). Each tool documents its
arguments in the docstring at the top of the file.

`aob.py` prints a `RESULT UNIQUE|MULTI(n)|NONE ...` line per pattern, so `-Version all`
gives a comparison table on the console. Run it before putting a pattern into
`common::memscan::find_pattern_in_module`: the pattern must be `UNIQUE` in every exe version
the mod supports. RVAs shift between versions, which is why mods use AOB patterns and never
hard-coded RVAs.

## Writing a new script

Add a `.py` file to this folder with this skeleton:

```python
import os, sys, idc
sys.path.insert(0, os.path.dirname(os.path.abspath(idc.ARGV[0])))
import common

common.init()                       # auto_wait + Hex-Rays + output folder
pos, flags = common.parse_args()    # positional arguments + --flag / --key=value
...                                 # common.decompile / callees / callers / open_out
common.finish()
```

Keep `import common` as written (not `scripts.ida.common`). Only the script's own folder is
on idat's `sys.path`, and an IDE's move-file refactoring may rewrite the import.

Conventions: take arguments on the command line (no constants to edit in the code), write
output to `common.OUT_DIR`, never modify the database, and put usage in the docstring. A
probe that answers one question once can live in a scratch folder, but the second time you
write it again, add it here with parameters.
