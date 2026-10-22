# NoRestResummon và SummonAnywhere (BuddyStoneParam)

**Status 2026-10-08: hoạt động (vanilla, Reforged, Seamless), đã test. Mã: `src/buddy_stone.rs`. `SummonAnywhere` mặc định `true` từ 1.1.0.**

## NoRestResummon

Chỉ sửa param (cách của Solid Uncapper, quét chuỗi trong DLL của họ): `BuddyStoneParam.summonedEventFlagId` (`+0xC`) = 0. Event flag này bật khi triệu hồi ở bia đá và chỉ tắt khi nghỉ → khoá "mỗi lần nghỉ 1 lần". Giá trị gốc chụp 1 lần, áp từ bản chụp (F5 bật/tắt 2 chiều); log `304 summoning pool row(s), 302 with a once-per-rest flag`.

Giới hạn: không gỡ được luật của Reforged (sau Spirit-Severing Blade ô Ash xám tới khi ngồi grace) - xem [open_issues.md](open_issues.md).

**Tương tác với SummonAnywhere (2026-10-09, đã test in-game: đúng):** khoá của vanilla (Ash xám sau khi thu hồi/chết tới khi ngồi grace) chỉ có khi `SummonAnywhere` tắt; bật thì stub stateInfo 373 ép `al = 1` nên đè luôn khoá, `NoRestResummon=false` mất tác dụng (người dùng Nexus báo, bạn xác nhận bằng cách tắt SummonAnywhere). Sửa: khi `NoRestResummon=false` + Anywhere, mỗi frame tick đọc event flag `summonedEventFlagId` gốc của bia hiện tại (`CSEventFlagMan`); cờ bật và không còn spirit thì `ANYWHERE = 0` để game tự trả lời (xám). Khoá theo từng bia, nên sang bia khác (ô map khác) thì mở lại.

## SummonAnywhere

Sửa param như Solid Uncapper (`activateRange +0x1C = 65535`, `overwriteReturnRange +0x1E = -1`, `overwriteActivateRegionEntityId +0x20 = 0`, `eliminateTargetEntityId +0x8 = 0`) **chưa đủ**: bản 1.1.0 báo lỗi ở Shadowlands và ngoài pool. Còn 3 chỗ cần đối tượng bia đá đang được nạp gần người chơi:

1. `GetBuddyState` (`sub_1404B72F0`) chỉ cho dùng Ash khi người chơi mang SpEffect **stateInfo 373** (`sub_1404FA370(player+0x178, 0x175)`, `0x1404B7368`); `sub_1404B7610` (HUD, tail-call cùng check) cũng vậy. Hiệu ứng do vùng pool gắn lên người chơi.
2. Bia hiện tại `+0x38` chỉ do **script ESD của chính bia đá** đặt (lệnh ESD 122 → `sub_1404B8050`), và bị **xoá mỗi frame** bởi `sub_1404B6E80` (từ `sub_140EAFDE0`). Không bia nào nạp → `+0x38 = 0` → `GetBuddyState` = -1 (xám).
3. `sub_1404BD870` (trong `Update`, `0x1404B8B2E`) tính lại `+0xB5` (`is_within_activation_range`) / `+0xB7` (`is_within_warn_range`) mỗi frame bằng cách tìm đối tượng bia `+0x3C` (`sub_1405EEFD0`); bia bị gỡ → cờ false → nhánh "rời khu" của `sub_1404B92B0` cho về hết.

Các patch (chỉ khi `SummonAnywhere` bật, cờ `ANYWHERE` mà stub đọc; từ 1.1.4 đều đổi đích `call`, xem [seamless_conflict.md](seamless_conflict.md)):

- stateInfo 373: bản `call` (AOB `BA 75 01 00 00 48 8B 88 78 01 00 00 E8 ...`, `0x1404B735C`) và bản tail `jmp` (`0x1404B7655`) → `al = 1`. Stub trong `GetBuddyState` (rbx = manager) ghi `FALLBACK_STONE` vào `+0x38` khi = 0, ngay trước khi nó đọc.
- In-range: stub cho `call sub_1404BD870` (AOB `41 0F 28 CA 49 8B CF E8 ...`, `0x1404B8B1B`).
- `FALLBACK_STONE` = bia thật gần nhất vừa thấy, hoặc dòng BuddyStoneParam đầu tiên không có `dopingSpEffectId` (vd. `10000100`); khi có spirit thì ưu tiên bia đang hoạt động `+0x3C` để `+0x38 == +0x3C`.
- Nếu 3 patch không cài được, lùi về chỉ sửa param (log lỗi).

### Sửa lỗi Reforged (2026-10-01)

Stub in-range ép `+0xB5 = 1, +0xB7 = 0` **mỗi frame kể cả khi chưa có spirit** nên `sub_1404B8EB0` (chạy trước, gắn SpEffect "vào vùng triệu hồi": `GameSystemCommonParam.onBuddySummon_inActivateRange_spEffectId_pc` = 9540 lên người chơi, `_buddy` = 9541 lên spirit khi `+0xB5` đổi 0 → 1) không thấy cạnh lên → không SpEffect nào được gắn; Reforged dựa vào chúng cho Spirit Ring và cho phép dùng Ash/Blade. Sửa: chỉ đặt `+0xB5 = 1` khi có spirit (`+0xB4 player_has_alive_summon`) **và** game không tính ra vùng nào (`+0xB5 == 0 && +0xB7 == 0`); còn lại để nguyên giá trị game. Phát hiện bằng cách tách `SummonAnywhere` thành 3 key test (Params/State/Range) và bật từng phần; Range là thủ phạm.

## Quan sát

- Test ở Limgrave: `stones: current=` đổi theo chỗ đứng (`1045380100`, `1046380100`...; 2 cặp số giữa = ô map `m60_XX_YY`), đôi khi về `0` (không có bia).
- Mỗi bia có `dopingSpEffectId` riêng nên buff của spirit có thể khác theo bia được chọn.
- Test cuối: gọi được ở Lands Between và Shadowlands ngoài pool, gọi ở pool rồi cưỡi Torrent ra xa spirit vẫn ở lại, cho về rồi gọi lại vẫn được.
- Param 134 (BuddyStoneParam) tra bằng entity id (`sub_140D28320`) → row id = entity id của bia; `+0x3C` về 0 trong `sub_1404B8EB0` khi hết spirit.
- Cách ghi `+0x38` từ tick FrameBegin vô ích (bị xoá mỗi frame) - đã bỏ; cách patch inline lệnh xoá `mov [rsi+38h],0` cũng bỏ ở 1.1.4 (xung đột Seamless).

## Ghi chú kỹ thuật chuyển từ code: `buddy_stone.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Spirit summoning pools (`BuddyStoneParam` - one row per summoning pool
area), two hot-reload toggles:

- `NoRestResummon`: re-summon after the spirits die (or are sent back)
  without resting at a Site of Grace. The once-per-rest lock is the
  row's `summonedEventFlagId` (+0xC): the game raises that event flag on
  a summon and refuses the pool while it is on; resting clears it.
  Written as 0 = no flag, so nothing ever locks the pool.
- `SummonAnywhere`: summon outside the vanilla areas. Every row gets
  `activateRange` (+0x1C) = 65535, `overwriteReturnRange` (+0x1E) = -1
  (no override), `overwriteActivateRegionEntityId` (+0x20) = 0 (use the
  range, not a map region) and `eliminateTargetEntityId` (+0x8) = 0 (the
  pool no longer closes once its boss is dead).

Both are exactly what Solid Uncapper 2.3.3 writes for its `No-Rest
Re-Summon` / `Summon Anywhere` settings (`sub_180031EC0` /
`sub_1800321B0`, snapshot in `sub_180031D30` - see README 2026-09-29);
re-implemented here. Rows are walked by index (`common::params`), and
the original values are captured once before the first write, so every
later change is applied from that snapshot (F5 works both ways).

The param edits alone only work while some pool's stone object is
loaded near the player (Nexus reports 2026-09-30: nothing outside pools
in the Shadowlands, or anywhere for one player). Three more things in
`SummonBuddyManager` depend on a loaded stone (2.7.1.0):

1. `GetBuddyState` (`sub_1404B72F0`) - and the HUD's
   `sub_1404B7610` - require the player to carry a SpEffect with
   stateInfo 373 (`sub_1404FA370(.., 0x175)`), which the pool area puts
   on the player. [STATE_CALL_AOB] / [STATE_TAIL_AOB]: that check
   answers true while Anywhere is on.
2. The current stone `+0x38` is cleared every frame by
   `sub_1404B6E80` (called from the talk-script update `sub_140EAFDE0`)
   and set again only by a loaded stone's own talk script (ESD command
   122 -> `sub_1404B8050`) - with none loaded it stays 0 and
   `GetBuddyState` says -1 (greyed out). The same stateInfo stub inside
   `GetBuddyState` fills an empty `+0x38` right before `GetBuddyState`
   reads it - every summon path asks `GetBuddyState` first - with the
   active stone `+0x3C` while spirits are out (so `+0x38 == +0x3C` and
   the state stays 2 = "this group is out"), else [FALLBACK_STONE] (the
   last real stone seen, or a default row). A BuddyStoneParam row id is the stone's entity id (param 134
   is looked up with it directly, `sub_140D28320`). (First try wrote
   `+0x38` from the FrameBegin tick - the clear ran after it; 1.1.1-1.1.3
   then patched the clear itself, an inline hook.)
3. `sub_1404BD870` recomputes "in the active stone's range" (`+0xB5`)
   every frame by finding the stone object; unloaded -> false -> the
   "left the summoning area" dismiss-all. [INRANGE_AOB]: right after it,
   while Anywhere is on, `+0xB5` is set to 1 **only** when spirits are
   out (`+0xB4`) and the game found neither range (`+0xB5` and `+0xB7`
   both 0 - the stone is unloaded). Until 2026-10-01 it forced
   `+0xB5 = 1, +0xB7 = 0` every frame, spirits out or not: the 0 -> 1
   edge on `+0xB5` that `sub_1404B8EB0` turns into the "entered the
   summoning range" SpEffects (GameSystemCommonParam
   `onBuddySummon_inActivateRange_spEffectId_pc/_buddy`, 9540/9541) never
   happened, which broke ELDEN RING Reforged's Fortune of the
   Spiritcaller (no Spirit Ring, Ash + Spirit-Severing Blade greyed out
   while spirits were out) - found by toggling each part in game.

4. With `NoRestResummon=false` the vanilla lock must survive Anywhere:
   vanilla greys the Ash out after a recall/death until a rest because the
   summoned flag turns the pool's stateInfo 373 off, and the stub in 1.
   would hide that. While no spirit is out and the current stone's
   original `summonedEventFlagId` is raised, [ANYWHERE] is set to 0 so the
   stubs pass the game's own answers through (greyed, like vanilla).

All three only redirect an existing `call` / `jmp`
(`codepatch::redirect_rel32`, see `multi_spirit.rs` - Seamless Co-op
aborts the game when its own byte signatures stop matching).
