# Giới hạn 60 nhân vật active và kẻ địch biến mất

**Status 2026-10-08: sửa xong, luôn hoạt động (không có key bật/tắt), đã test ở Stormgate với 60 spirit. Mã: `src/activate_limit.rs`.**

## Triệu chứng và nguyên nhân

Gọi 60 spirit trước Stormgate thì lính ở đó biến mất. Nguyên nhân: spirit chiếm suất trong giới hạn **60 nhân vật active** của engine.

- Không phải omission (`ChrIns::omission_mode`, budget `WorldChrMan+0x1F220/+0x1F224`, `sub_14050FEA0`): chỉ hạ tần suất update, làm giật chứ không làm biến mất.
- Không phải ngưỡng khoảng cách (`sub_1403FBE70`, `chr_activate_threshold +0x40C`, cờ `+0x1CA` bit 2): không đếm số lượng.
- Thủ phạm: `sub_140510970` gom ứng viên activate vào vector `WorldChrMan+0x1F1D0` và tăng bộ đếm `WorldChrMan+0x1E618` cho mọi nhân vật đã active; `sub_14050F9E0` chỉ activate `limit - bộ_đếm` con đầu, phần còn lại bị cắt (`sub_1403E93A0(chr, 2)`).
- `limit` nằm trong singleton `qword_143D6A208` (constructor `sub_145AE6B75`): `+0xE8 = 1` (bật), **`+0xEC = 60`, `+0xF0 = 60`**, `+0xF4` = cờ chế độ thay thế, `+0xF8/+0xFC = 40`. Chỉ áp dụng khi `+0xE8` hoặc `+0xF4` khác 0.

## Cách sửa

AOB `48 8B 05 ?? ?? ?? ?? 80 B8 E8 00 00 00 00 75 0D 80 B8 F4 00 00 00 00 0F 84` (`0x14050FAEC`, duy nhất) → địa chỉ singleton. Mỗi giây đếm slot có nhân vật trong `summon_buddy_chr_set` từ `BAND_START` (20) trở lên (bỏ Torrent, tính cả spirit người chơi co-op hiện trên máy mình) và đặt `+EC/+F0/+F8/+FC = giá trị gốc + số đó`. Chỉ đọc con trỏ entry, không đọc `ChrIns`. Không có spirit thì giữ đúng vanilla (không tốn CPU thêm). Giá trị gốc `[60, 60, 40, 40]` được chụp lần đầu thấy singleton.

`ActiveCharacterLimit` (ini, debug, mặc định 0): `> 0` ghi đè giá trị cố định (test 120 → lính không biến mất lại; F5 về 60 → biến mất lại; về 0 → trả giá trị gốc). Comment ini ghi "DO NOT CHANGE".

## Bài học

- `EnemyProbe` (`src/enemy_probe.rs`, ini mặc định `false`) **gây crash 2 lần**: đọc `ChrIns` của entry `Unloaded` (con trỏ cũ đã free), rồi vẫn crash khi duyệt `chr_sets` lúc map đang stream. Không dùng nếu chưa đổi cách duyệt (vd. `chr_inses_by_distance` của game).
- `WorldChrMan+0x1E618` là số nhân vật đã activate trong frame (game gốc đã chạm 60 sẵn); bit 9 của `load_state` **không** có nghĩa "bị cắt" (đã đính chính), đừng dùng làm bằng chứng.
- Agent tìm cộng đồng không thấy báo cáo công khai nào về triệu chứng này, cũng không có mod nào nâng giới hạn.
- Ban đầu có key `KeepEnemySlots`; bỏ 2026-09-26 vì kẻ địch biến mất là lỗi của mod, không phải tính năng để bật/tắt.
- Chi phí thật là game xử lý thêm nhân vật khi giới hạn được nâng (CPU nặng hơn khi vừa nhiều spirit vừa nhiều kẻ địch).

## Ghi chú kỹ thuật chuyển từ code: `activate_limit.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Keeps the engine's cap on how many open-field characters get activated
at once from being eaten by our spirits - ~60 summoned spirits crowded
every nearby enemy out of it (Stormgate soldiers vanishing, confirmed
2026-09-26 by raising the cap to 120).

Always on - enemies vanishing was a bug of this mod, not a setting
(user decision 2026-09-26; an earlier `KeepEnemySlots` toggle was
removed): every tick the cap becomes the game's own value plus the
number of spirits currently out, so enemies keep exactly the slots
they'd have in vanilla, and with no spirits out the game runs at its
vanilla cap (no extra CPU) - user's choice over a fixed
`60 + MaxSpirits`. `ActiveCharacterLimit` (> 0, debug only) overrides it
with a fixed value.

Found in IDA on `eldenring.exe` 2.7.1.0: `sub_14050F9E0` sorts the
open-field activation candidates and activates only
`limit - WorldChrMan[+0x1E618]` of them (that counter = characters
already activated), cutting the rest. `limit` lives in the singleton at
`qword_143D6A208`, set by its constructor (`sub_145AE6B75`):
`+0xE8 = 1` (limit on), `+0xEC = 60`, `+0xF0 = 60` (the two limits
`sub_14050F9E0` reads), `+0xF4` = alternate-mode flag, `+0xF8 = 40`,
`+0xFC = 40` (the limits used in that mode).

The singleton's address is read from the RIP-relative `mov rax,
[qword_143D6A208]` right before `sub_14050F9E0`'s `cmp byte [rax+0xE8],0`
([LIMIT_READ_AOB]); its fields are (re)written on a tick rather than
once, since the singleton may not exist yet at startup - which also
makes the key hot-reloadable (F5), with 0 restoring the game's own
values.

## Ghi chú kỹ thuật chuyển từ code: `enemy_probe.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Debug-only (`EnemyProbe`, off by default) diagnostics for "enemies near
the player vanish when ~60 spirits are summoned" (reported at Stormgate,
2026-09-26) - no gameplay change, only logging.

Candidates, from `fromsoftware-rs`'s reflected types:
- Omission (update LOD): `ChrIns::omission_mode` (Normal / 30 / 20 / 5 /
  1 FPS / NoUpdate), handed out per frame from
  `WorldChrMan::omission_update_budget_near/far`, whose size depends on
  `WorldChrManDbg::omission_update_num_type` (Normal / Overload /
  Emergency, "changed depending on current performance load").
- Activation: `ChrIns::chr_activate_threshold` compared against
  `CSOpenChrActivateThresholdRegionMan` (open-field characters), result
  in `chr_flags1ca.activate_threshold_exceeded`; plus
  `chr_activation_flags.activation_enabled`.
- Unloading: `ChrSetEntry::chr_load_status`, or the character simply
  disappearing from its ChrSet.

- Activation COUNT limit (found in IDA 2026-09-26, ER 2.7.1.0):
  `sub_14050F9E0` sorts open-field activation candidates and only
  activates `limit - already_active` of them (the rest get
  `sub_1403E93A0(chr, 2)` = cut, `load_state.evaluation_value_cutoff`);
  `limit` is 60 (hard-coded in the singleton ctor `sub_145AE6B75`,
  `+0xEC`/`+0xF0`; 40 in an alternate mode) and `already_active` is
  the counter at `WorldChrMan+0x1E618` (124440), bumped for every
  always-active character - logged here as `active_count`.

Every tick this collects every non-summon character within
[RADIUS_M] of the player across all of `WorldChrMan.chr_sets`, and logs
a summary line (plus the per-character list) whenever the summary
changes, together with the live summon count - so summoning at a spot
with enemies shows which of the above actually happens to them.
