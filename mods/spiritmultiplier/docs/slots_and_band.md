# Slot, band và trần số spirit

**Status 2026-10-08: hoạt động, đã test offline (100 spirit) và Seamless Co-op 2 người. `MaxSpirits` mặc định và tối thiểu là 60. Mã: `src/band.rs`, `src/chain.rs`, `src/probe.rs`.**

## Kết luận

- Spirit nằm trong `WorldChrMan.summon_buddy_chr_set` (`ChrSet<ChrIns>`, chứa cả spirit lẫn Torrent). Capacity do game tạo là **80** (`sub_1404954B0`: `alloc(0x500)`, `capacity = 0x50`, 3 imm32). Slot #0 = Torrent (có sẵn từ lúc vào world), #1..#19 dành cho Torrent người chơi khác, band người chơi local bắt đầu ở **#20**.
- Trần 10 con của vanilla nằm ở `sub_140493380` (spawn 1 spirit): `band_start = 10 * (player_index + 2)`, `cursor %= 10`, duyệt 10 slot tìm entry trống, hết thì trả null (spirit bị bỏ). `sub_140493D60` tính cursor kế: `(slot - 20 + 1) % 10`. `SummonBuddyManager.last_buddy_slot = +0xBC`.
- Offset (bản 2.7.1.0): `WorldChrMan.summon_buddy_chr_set = +0x10F90`, `summon_buddy_manager = +0x1E538`.
- Dismiss/despawn không duyệt band hard-code (test 30 con ở #20-#49, dọn sạch cả slot >= #30).

## Cách mod vượt trần (`src/band.rs`, cài 1 lần lúc khởi động)

1. AOB `0F B6 C2 83 C0 02 8D 04 80 44 8D 14 00 B8 67 66 66 66 41 F7 E9` (`0x1404933AF`, duy nhất): thay cả vòng tìm slot bằng stub; band dài `MaxSpirits` cho người chơi index 0, kiểm tra `slot < capacity`, thoát đúng 2 lối của vanilla.
2. AOB `83 E8 14 78 22 8D 48 01 B8 67 66 66 66 F7 E9 C1 FA 02` (`0x140493D7C`): next-cursor trả `slot - 19` không wrap; stub tự đổi về vị trí trong band của người gọi `(cursor - (start - 20)) mod len`.
3. Chỉ khi `MaxSpirits > 60`: AOB `CAPACITY_INIT_AOB` (`0x140495534`) ghi `20 + MaxSpirits` vào 3 imm32 nới capacity (chạy trước khi world được tạo). Patch lỗi thì band tự lùi về 60.

`MaxSpirits` đọc 1 lần lúc khởi động (cần restart, F5 không áp dụng). Kẹp `60..=1000`: ở 60 mảng slot 80 của game giữ nguyên như vanilla. Test `MaxSpirits=100` offline: 100 sói ở #20-#119, load dần ~2 giây, dọn sạch, không crash; FPS tụt theo số spirit; **không tìm tiếp trần heap nhân vật** của engine.

`src/chain.rs` kéo dài chain SpEffect → BuddyParam (`WorldChrMan.summon_buddy_manager.trigger_speffect_to_buddy_map`) theo `Multiplier`/`Amount`, cap `band_len()`; copy từ `sometweaks/src/spirit/summon_count.rs` (không đưa lên `common`, người dùng chọn 2026-09-25). Hot reload; đặt về mặc định thì chain trả độ dài vanilla.

## Seamless Co-op

- Seamless tự nới capacity lên **1000** trên mọi máy. `sub_140493210` (tạo spirit nhận qua mạng) **không kiểm tra slot < capacity**, ghi thẳng `entries[slot]` và nếu slot đã có nhân vật thì trả nhân vật đó → band của mọi người chơi phải nằm gọn trong capacity của mọi máy và không chồng nhau.
- Stub chia band theo capacity thật (`band_layout`, có test): index 0 `[20, 20 + MaxSpirits)`; index 1..5 `part = (capacity - 20) / 6`, bắt đầu `20 + index × part`, dài `min(MaxSpirits, part)`. Capacity 80 (co-op vanilla) → `part = 10` → đúng band vanilla. Capacity 1000 → band bắt đầu `20, 183, 346, 509, 672, 835`.
- Test 2 người cùng cài mod (`Amount=30`): người join gọi đủ 30 con ở band #183.., cả 2 máy thấy đủ 60 spirit, dọn khớp nhau.
- Giới hạn: người chơi không cài mod vẫn dùng band vanilla #30..#79 (trùng band rộng của host); `MaxSpirits > 163` với Seamless thì band index 0 chồng band index 1; giả định mọi máy cùng capacity.
- Còn nghi vấn: đôi khi chỉ đồng bộ 1 phần spirit (11/30, 13/30) ngay sau khi 1 bên load lại khu vực; nghi do đồng bộ mạng của engine/Seamless (hoặc 2 game chung máy qua Sandboxie), chưa xác nhận.

## Hướng đã cân nhắc / bỏ

- B. Cấp phát lại mảng `entries` của ChrSet (rủi ro cao, engine có thể cache con trỏ/index) - không cần vì capacity 80 đã đủ cho 60.
- C. Spawn NPC ngoài hệ thống buddy - bỏ, mất AI "theo người chơi"/despawn theo Ash.
- Ghidra quá nhiễu (270 hàm khớp), chuyển sang IDA.

## Chẩn đoán

`SlotProbe` (ini, mặc định `false`): log capacity, histogram độ dài chain vanilla (`{1: 58, 2: 18, 3: 8, 4: 4, 5: 6}`), bố cục slot khi đổi, `last_buddy_slot`, và `stones:` (`+0x38`/`+0x3C`, cờ `+0xB5`/`+0xB7`).

## Tham chiếu

- Xung đột: hook "buddy slot-reserve" của Solid Uncapper (`0x1404933A9`) nằm trong đúng `sub_140493380` → không dùng chung khi họ bật Max Spirits Out / Multi Spirit.
- [activate_limit.md](activate_limit.md): vì sao 60 spirit làm kẻ địch biến mất.

## Ghi chú kỹ thuật chuyển từ code: `band.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Widens the local player's spirit-summon "band" in
`WorldChrMan.summon_buddy_chr_set` from the engine's hard-coded 10
slots to `MaxSpirits` ([band_len]) - the real reason no amount of chain lengthening
(`chain.rs`, `sometweaks`'s `summon_count.rs`, `er10x.dll`) could ever
spawn more than 10 spirits.

Found in IDA on `eldenring.exe` 2.7.1.0 (2026-09-25), confirmed by
`probe.rs`'s in-game slot log (see README):

- `sub_140493380(chr_set, player_index, spawn_info, cursor)` - the
  summon ChrSet's "spawn into my band" function, called once per spirit
  from the SummonBuddyManager spawn routine (`sub_1404BAEA0`, which
  passes the local player's session index, `< 6`, and
  `SummonBuddyManager::last_buddy_slot` as `cursor`):
  ```text
  band_start = 10 * (player_index + 2)   // player 0 -> slot 20
  cursor %= 10
  for i in 0..10 { slot = band_start + (cursor + i) wrapped to 10;
                   if entries[slot] is empty -> spawn there }
  return null                             // band full - spirit dropped
  ```
  The ChrSet's capacity is 80 = 20 + 6 bands * 10, so offline (player 0
  alone) slots 30..79 are never used by anything.
- `sub_140493D60(spawned_chr)` - the next cursor:
  `(slot_of(spawned_chr) - 20 + 1) % 10`.

Patches:
1. [BAND_ALLOC_AOB]: the slot-search loop (everything from computing
   `band_start` up to the "band full" exit) is replaced by a stub that
   does the same search over the band [band_layout] gives the calling
   player index, bounds-checked against the ChrSet's own `capacity`. Exits exactly where vanilla does: "found" jumps to
   the original spawn code with `rbx` = entry, `edi` = slot index; "not
   found" falls through to the original `mov rax, r15` (null) return.
2. [NEXT_CURSOR_AOB]: the next-cursor computation stops wrapping
   (`(slot - 20 + 1) % 10` -> `slot - 20 + 1`): it has no idea which
   player's band `slot` is in, so patch 1 turns that absolute value
   back into a position inside the caller's own band instead. (First
   version did `% band_len` here, which is only right for a band that
   starts at slot 20 - partners' spirits ended up scattered across
   their band, Seamless test 2026-09-26.)

3. [CAPACITY_INIT_AOB] (only when `MaxSpirits > 60`): the summon
   ChrSet's own init (`sub_1404954B0`) allocates a fixed 80-entry array
   (`alloc(0x500)`, `capacity = 0x50`, `memset(.., 0x500)`) - those
   three imm32s are raised to `20 + MaxSpirits` so the game builds a
   bigger array itself, instead of this mod reallocating a live one.
   Must run before the world is created (the DLL loads at process
   start); if it ran too late the array just stays at 80, and patch 1's
   `slot < capacity` check keeps everything in bounds regardless. Slot
   indices are encoded in FieldInsHandles with a 20-bit mask
   (`0xFFFFF`), so large capacities are representable.

Player index 0 is the host (and the only player offline). Co-op
partners (index 1..5) get bands of their own, laid out by [band_layout]
from the ChrSet's real capacity so they never overlap player 0's - a
remote spirit is created on every machine at its OWNER's slot number
(`sub_140493210`, which also does no bounds check), so bands must not
overlap across players. With vanilla's 80-slot capacity that layout
collapses to exactly vanilla's 10-slot bands; with Seamless Co-op's
1000 (observed 2026-09-25) every player gets up to 163. Limits: a
partner WITHOUT this mod still uses vanilla's 30..79 bands, which do
overlap player 0's widened band; and all machines are assumed to share
the same capacity (true under Seamless, which sets it on everyone).

## Ghi chú kỹ thuật chuyển từ code: `chain.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Lengthens every Ash's SpEffect -> BuddyParam chain so one cast asks for
more spirits - the other half of this mod, next to `band.rs` (which
makes room for them).

Same technique as `sometweaks`'s `spirit/summon_count.rs` (itself
ported from `x10_summon/er10x.dll`'s "chain rebuild"), kept as a
separate copy here rather than moved to `common` at the user's request
(2026-09-25). The only behavioral difference is the cap:
[crate::band::band_len] (`MaxSpirits`) instead of 10. See `summon_count.rs`'s module
doc comment for the full reasoning; in short:

- `WorldChrMan.summon_buddy_manager.trigger_speffect_to_buddy_map` maps
  an Ash's summon SpEffect to a linked chain of BuddyParam IDs, one per
  creature. The game's own head node is left untouched; only its `next`
  is redirected to a `Box::leak`'d extension that cycles through the
  original IDs (never freed - the game may still hold the old chain).
- Original chains are snapshotted once per SpEffect ID and every rebuild
  is computed from that snapshot, never from the (possibly already
  rebuilt) current chain - otherwise `Multiplier` compounds every tick
  (the bug `summon_count.rs` hit on 2026-08-26).
- Re-checked on a periodic tick (cheap and idempotent), which also
  picks up ini changes after a hot reload.

## Ghi chú kỹ thuật chuyển từ code: `probe.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Debug-only (`SlotProbe`, off by default) diagnostics that located the
10-spirit cap (see `band.rs`) - no gameplay
change at all, only logging.

`sometweaks`'s `spirit::summon_count` (and `x10_summon/er10x.dll`
before it) can lengthen an Ash's SpEffect -> BuddyParam chain freely, but
the engine only ever spawns 10: per `er10x.ini`, each player gets a fixed
"band" of 10 slots inside `WorldChrMan.summon_buddy_chr_set`, whose start
and length are engine constants. Before deciding between widening that
band inside the ChrSet's existing capacity or reallocating the ChrSet's
`entries` array outright, we need two facts this module logs:

1. The ChrSet's real capacity (both the `capacity` field and the vtable's
   own `get_capacity()`, in case they disagree) - logged once, the first
   time the player is in the world.
2. Which slot indices summoned spirits (and Torrent) actually land in -
   logged every time the occupied-slot layout changes, so summoning a
   multi-creature Ash shows where the local player's band starts.

Also logs the SpEffect -> BuddyParam chain-length histogram once (how
many vanilla Ashes summon 1, 2, 3... creatures), plus a few
`SummonBuddyManager` fields that look slot-related (`last_buddy_slot`,
`active_summon_speffect_id`) alongside each layout change.

The player SpEffect log is in `effect_probe.rs` (`EffectProbe`).
