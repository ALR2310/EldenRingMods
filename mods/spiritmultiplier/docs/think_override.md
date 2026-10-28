# SpiritThinkParam.ini / SpiritParam.ini: chỉnh param spirit khi đang chơi (thử nghiệm)

**Status 2026-10-08: đã viết, đang test in-game. Chưa có trong ini mặc định và chưa nói trong CHANGELOG (công cụ để nghiên cứu độ hung hăng của spirit).**

## Cách dùng

File `SpiritThinkParam.ini` (AI) và `SpiritParam.ini` (NpcParam) cạnh DLL, chỉ đọc nếu tồn tại. Mỗi dòng `Tên=giá trị`; tên như trong ngoặc của Smithbox (`TeamAttackEffectivity`, `isGuard_Act`) hoặc tên Rust (`team_attack_effectivity`), không phân biệt hoa thường và dấu `_`. Lưu file lúc đang chơi: tối đa 1 giây sau mod áp lại (không cần F5), rồi triệu hồi lại spirit. Bỏ dòng hoặc xoá file thì về giá trị gốc của game.

## Cách hoạt động

- Hàng đích: mọi `npcThinkParamId` / `npcThinkParamId_ridden` của `BuddyParam`, chọn theo index (không qua bảng tra runtime).
- Lần đầu chụp nguyên 228 byte của mỗi hàng, mỗi lần áp bắt đầu từ bản chụp rồi ghi các field trong file (cùng mẫu baseline với `ghost_color.rs`).
- Bảng field (`src/think_fields.rs`) sinh từ `NPC_THINK_PARAM_ST` của fromsoftware-rs (85 field số + 12 cờ bit đơn (`enableNaviFlg_*`, `isNoAvoidHugeEnemy`...; giá trị 0/1)); test `offsets_match_struct` đối chiếu offset với setter của thư viện.

## Dữ liệu để thử (từ Age of Spirit, regulation.bin của họ so với gốc, dòng Lone Wolf)

| Field | Age of Spirit | Gốc |
|---|---|---|
| TeamAttackEffectivity | 100 | 34 (spirit đầu bảng: 50) |
| BattleStartDist | 15 | 2 |
| eye_dist / searchEye_dist | 15 | 2 / 0 |
| nose_dist | 5 | 0 |
| searchTarget_LV1/2_forgetTime | 5.0 | 1 |

Tooltip Smithbox của TeamAttackEffectivity nói "tăng thì ít người tấn công cùng lúc", ngược với mô tả của tác giả Age of Spirit; chưa biết bên nào đúng, cần test.

## Nhóm Navigation và Retreat (Age of Spirit)

- `enableNaviFlg_Ladder/Hole/InSideWall/Lava/Edge_Ordinary` = 1 (gốc 0): quyền đi qua thang, hố, tường navmesh, dung nham, mép vực khi tìm đường. Theo đoán: giúp spirit bám theo qua mép cao thay vì kẹt; Lava = 1 có thể cho spirit đi vào dung nham. Có thể liên quan đến chuyện rơi/lạc map (TODO).
- Retreat: `maxBackhomeDist`/`backhomeDist` 30 (gốc 9999/9979), `backhomeBattleDist` 60 (gốc 10), `BackHome_LookTargetTime/Dist` 15, `backToHomeStuckAct` 1 (gốc 0), `BackHomeLifeOnHitEneWal` 0.1 (gốc 5.0). Theo đoán: "home" là chỗ người chơi; gốc spirit đuổi địch gần như vô hạn, còn 30 m thì bị kéo về khi đi quá xa. Giá trị cũ của `Warp` (đã gỡ) gần giống bộ này.

## Phân tích bảng NpcThinkParam của Age of Spirit (200 dòng, export 2026-10-09)

- Gần như mọi field là hằng số cho cả 200 dòng (BattleStartDist 15, tầm nhìn, nhóm Navigation, Retreat...). Chỉ vài field khác nhau giữa các dòng: nhóm `goalAction_*`, `TeamAttackEffectivity` (100 ở 179 dòng; 0/30/50/60/80 ở số ít, thường là healer/phụ trợ), `useFall_onNormalCaution` (0 hoặc 2, chia đôi), `actTypeOnFailedPath`, `eye_BeginDist`.
- `goalAction_*` (ToCaution / ToCautionImportant / ToSearchLv1 / ToSearchLv2 / ToDisappear): 116 dòng cận chiến = 3/3/3/3/3 (chạy tới nguồn). 41 dòng pháp sư/cung thủ (Noble Sorcerer, Rennala, Page, Oracle Envoy, các Archer, cả Spirit Jellyfish) = 2/3/1/1/2: tiến lại gần / chỉ nhìn, không lao vào. Còn lại là biến thể nhỏ (healer có ToDisappear 0, hiệp sĩ lửa...). Nên đặt 3 cho tất cả sẽ làm pháp sư chạy vào cận chiến.
- Vì vậy `SpiritThinkParam.ini` có thêm phần `[Row id, id, ...]`: áp lên đúng dòng NpcThinkParam có ID đó, sau các dòng chung. File ini mẫu sinh sẵn 12 phần Row từ bảng này.

## Sát thương rơi (thử 2026-10-09)

**Kết quả: `NpcParam.fallDamageDump = 100` không giúp spirit sống sót khi rơi (người dùng thử trong Convergence, cùng ngày). Bỏ hướng này.**

`NpcParam.fallDamageDump` (u8) là 0 ở mọi `[Spirit Summon]`, 100 ở dummy/Bonfire/Maliketh/Gurranq, 90 ở rồng và Giant Crow; ta đoán là % giảm sát thương rơi nhưng spirit vẫn chết với 100 (có thể field này không áp dụng cho spirit, hoặc cú rơi đó là chết ngay viết cứng trong code). Phần `hp=1` để làm tín hiệu rõ hơn chưa từng được bật trong lần test này (dòng bị comment).

Hướng còn lại: SpEffect có `fall_damage_rate` (f32); `SpiritParam.ini` có thể gắn nó vào slot `spEffectID0..31` của NpcParam spirit mà không cần code. Khi module chạy lần đầu nó ghi log các SpEffect có `fallDamageRate` khác 1.0 (dòng `SpEffect fallDamageRate=...`) để chọn ID. `HitMtrlParam.disable_fall_damage` là theo chất liệu nền nên không dùng riêng cho spirit được. Rơi ra khỏi map vẫn thuộc việc warp đang hoãn.

## Bộ giá trị tích hợp: ImproveSenses / ImproveAggression / ImproveFollow (2026-10-09)

**Status 2026-10-09: viết xong, mặc định `true`; từng giá trị đã thử trong game (bật cả bộ, hành vi spirit được cải thiện), chưa tách từng nhóm để biết nhóm nào quyết định.**

Ba key ini bật/tắt (hot reload bằng F5) nạp sẵn các field của Age of Spirit vào `NpcThinkParam` của spirit (`SENSES`, `AGGRESSION`, `FOLLOW` trong `think_override.rs`):

- `ImproveSenses`: tầm nhìn / khứu giác / thính giác, thời gian nhớ mục tiêu, tầm nhìn khi chiến đấu.
- `ImproveAggression`: `TeamAttackEffectivity`, `BattleStartDist`, kêu gọi đồng đội (Platoon), `isGuard_Act`, `thinkAttr_doAdmirer`, nhảy.
- `ImproveFollow`: quay về chỗ chủ khi xa hơn ~30 m (gốc 9999), quyền đi thang / hố / tường navmesh / mép vực. Bỏ Lava có chủ ý.

Mỗi mục có chế độ: `Max` chỉ nâng giá trị gốc lên, `Min` chỉ hạ xuống, `Set` luôn ghi, để không làm hỏng các mod đã chỉnh sẵn các field này (vd. Age of Spirit đặt giá trị riêng cho healer). Các bộ được ghi trước, `SpiritThinkParam.ini` / `SpiritParam.ini` ghi sau nên thắng. Không đưa vào bộ: `isBuddyAI`, `disableDark` (gốc đã là 1), `enableNaviFlg_Lava`, nhóm `goalAction_*` (người dùng quyết định bỏ), `fallDamageDump` (không có tác dụng).

## Dòng dùng chung với kẻ địch (2026-10-28)

**Status 2026-10-28: đã sửa trong code (3 bộ `Improve*` chỉ ghi dòng có `isBuddyAI = 1`), đã thử trong game cùng LoA_Ashmaker: kẻ địch không còn bị kéo về. Nguyên nhân đã tái hiện và xác nhận bằng cách tắt `ImproveFollow`.**

Báo cáo Nexus (loltem154, 1.5.0): chạy cùng LoA_Ashmaker, kẻ địch và boss đuổi một đoạn rồi dịch chuyển về điểm spawn và không đuổi nữa (một số boss lún xuống đất, không đánh chết được). Tái hiện được khi bật cả hai mod; chỉ một trong hai thì bình thường; `ImproveFollow=false` thì hết.

- Spirit không chỉ dùng dòng AI riêng: `BuddyParam.npcThinkParamId` có thể trỏ tới dòng của kẻ địch. Vanilla: 98 dòng, 7 dòng có `isBuddyAI=0` và tên kẻ địch (Demi-Human 1-5 `141000000`..., Land Squirt `144400000`, Miranda Sprout `144810000`); 91 dòng còn lại `isBuddyAI=1`. Trong toàn bộ `NpcThinkParam` vanilla, `isBuddyAI=1` còn có thêm 10 dòng không do `BuddyParam` tham chiếu (ID `100000000`, `1500000xx`, `199999999`, không tên).
- LoA_Ashmaker (dữ liệu export `tmp/csv/LoA_Ashmaker/`): 640 dòng think do spirit dùng, 429 dòng `isBuddyAI=1`, **210 dòng `isBuddyAI=0`** dùng chung với kẻ địch. Dòng kẻ địch của LoA có `maxBackhomeDist` thường 85 (leash); `ImproveFollow` (`Min 30`) hạ xuống 30 nên kẻ địch bỏ đuổi sớm.
- Cả `ImproveSenses` và `ImproveAggression` cũng ghi lên các dòng đó (tầm nhìn, `BattleStartDist`...), nên kẻ địch dùng chung đổi hành vi theo; vanilla đã bị từ trước ở 7 dòng kể trên.
- Sửa: `apply` chỉ ghi các bộ lên dòng `isBuddyAI=1` (đọc từ bản chụp gốc); dòng khác bị bỏ qua và log `bundles skipped N shared row(s)`. File `SpiritThinkParam.ini` (global và `[Row ...]`) vẫn ghi mọi dòng của spirit vì người dùng tự chọn. Cái giá: 7 spirit vanilla kể trên (và spirit mượn dòng kẻ địch ở mod khác) không còn được cải thiện AI.
- Đã test 2026-10-28 (loltem154 báo lỗi, người dùng tái hiện và xác nhận hết lỗi sau sửa).

## Ghi chú kỹ thuật chuyển từ code: `think_override.rs` (2026-10-22)

Nội dung này trước đây là doc đầu file `.rs`; chuyển sang đây nguyên văn (tiếng Anh) để doc code chỉ còn vài dòng.

Spirit AI tweaks, on the `NpcThinkParam` rows spirits use (the AI:
aggressiveness, sight, battle start distance, ...). Hot reload, no F5 for
the files below:

- `ImproveSenses` / `ImproveAggression` / `ImproveFollow` (ini keys,
  default `true`): built-in bundles of those fields, see [SENSES],
  [AGGRESSION] and [FOLLOW]. Values were taken from the Age of Spirit
  mod's regulation (its spirits notice enemies from farther away, engage
  together and keep up with the player) and tried in game 2026-10-09.
- `SpiritThinkParam.ini` and `SpiritParam.ini` (optional, experimental):
  set any scalar field / bit flag of the `NpcThinkParam` / `NpcParam` rows
  spirits use, so tweaks can be tried in-game without rebuilding the DLL.
  They are written after the bundles and win over them.

The files live next to the DLL, are never created and are only read if they exist. One
`Name=value` per line, `;`/`#` comments. `Name` is the
field name as in Smithbox's parentheses (`TeamAttackEffectivity`,
`BattleStartDist`, `isGuard_Act`) or the Rust one
(`team_attack_effectivity`) - case and underscores are ignored when
matching. The file is checked once a second; saving it re-applies it.
Spirits already out keep the old values: re-summon them.

Which rows: every `npcThinkParamId` / `npcThinkParamId_ridden` of
`BuddyParam` (the rows a spirit spawns with). Rows are addressed by index
(`common::params`), never through the runtime lookup table.

`SpiritParam.ini` (NpcParam: fields as in Smithbox, e.g. `fallDamageDump`)
hits the `NpcParam` rows of the same spirits (`npcParamId` /
`npcParamId_ridden`). In `SpiritThinkParam.ini` a `[Row id, ...]` section
applies to the listed `NpcThinkParam` row IDs only; any other section header
goes back to the global lines. Same snapshot / restore logic for both.

The first time it runs, each target row's bytes are saved; every later
apply starts from that snapshot and writes the listed fields, so a line
removed from the file goes back to the game's value (same baseline
pattern as `ghost_color.rs`).

The field table in `think_fields.rs` was generated from
`NPC_THINK_PARAM_ST` (fromsoftware-rs): scalar fields and single-bit flags
(`enableNaviFlg_*`, `isNoAvoidHugeEnemy`, ...; value 0 or 1).
