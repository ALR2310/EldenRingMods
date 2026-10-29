# JourneyHUD: hiệu ứng đang chạy của kẻ địch (SpEffect) và thời gian còn lại

**Status 2026-10-28: hàng effect của kẻ địch khoá đã có code (người dùng xác nhận chạy 2026-10-29): đọc SpEffect có icon hoặc là effect nổ của độc, thối rữa, đóng băng, ngủ (ghép dải id, bảng bên dưới), vẽ ô icon và thời gian dưới thanh buildup, căn phải, 6 ô một hàng, tối đa 12 (khoá `Effects.Target`, `Effects.Player`). Chưa làm: hàng effect của người chơi (log `Effects: player` đã cho thấy dữ liệu đúng).**

## ERBuffTimer làm thế nào (phân tích tĩnh `ERBuffTimer.dll`, không chạy nó)

Mod đã tải về ở `C:\Users\Admin\Downloads\ERBuffTimer` (README, ini, tên, 172 icon, DLL x64 938 KB). Chỉ xem, chưa cài.

- **Nguồn dữ liệu:** danh sách SpEffect đang hoạt động trên nhân vật cùng thời gian còn lại; thống kê (`show_stats`) đọc từ các dòng `SpEffectParam` (tự tìm bảng trong repository: log `SpEffectParam found at repository+0x..`, kiểm cỡ dòng).
- **Tên:** tra kho text của game theo ngôn ngữ đang chạy, qua bảng `game_names.txt` (`ID effect=Loại:ID vật phẩm`, loại G vật phẩm/phép, A talisman, S kỹ năng, W vũ khí, P giáp); effect không có tên dùng `names_en.txt`/`names_ru.txt` (từ Paramdex).
- **Icon:** icon trạng thái của game đọc thẳng từ archive `Data*` (`menu\hi\01_common`, dùng `BCrypt`/`CRYPT32` giải mã header, như `map/archive.rs` của ta), giải nén atlas rồi vẽ bằng shader riêng; mỡ vũ khí và buff vũ khí dùng 172 PNG lấy từ wiki Fextralife (ta không dùng lại, chỉ dùng icon của game).
- **Vẽ và ẩn:** hook `Present` và `ExecuteCommandLists` bằng cách sửa vtable (không hook inline); đọc một cờ HUD của game để ẩn khi mở menu (log `HUD flag found at [exe+..]+0x..`).
- **Lọc:** chỉ buff có lợi (hiệu ứng xấu để thanh buildup của game); `hide_ids` bỏ id nội bộ; `show_unknown` hiện cả effect không tên.
- **An toàn:** không thấy import mạng nào; có `ShellExecuteW`, `VirtualProtect/Alloc` (hook), `GetCursorPos/SetCursorPos` (kéo panel). Mod nói rõ chỉ dùng offline.

## Dữ liệu ta có sẵn cho kẻ địch

`fromsoftware-rs` (rev của workspace) đã gõ kiểu cho danh sách này (`cs/sp_effect.rs`), nên không cần offset thô như phần resist. Offset lấy bằng `offset_of!`, đối chiếu IDA ở mục "Đã kiểm":

| Trường | Offset | Ý nghĩa |
|---|---|---|
| `ChrIns.special_effect` | +0x178 | con trỏ tới `SpecialEffect` |
| `SpecialEffect.head` | +0x08 | đầu danh sách liên kết (`entries()` duyệt) |
| `SpecialEffectEntry.param_data` / `param_id` | +0x00 / +0x08 | dòng `SpEffectParam` và id của hiệu ứng |
| entry kế (`next`) | +0x30 | nối danh sách |
| bộ đếm tick trước (`removal_timer` trong crate) | +0x40 | giá trị của tick trước |
| **thời gian còn lại thật** (`unk_removal_timer` trong crate) | **+0x44** | giây còn lại, trừ mỗi frame |
| `duration` | +0x48 | tổng thời gian |
| `interval_timer` | +0x4C | bộ đếm chu kỳ (độc, chảy máu theo nhịp) |
| cỡ một entry | 0x78 | |

Đọc cùng cách như `target.rs` đã làm với kẻ địch khoá: `chr_by_handle` rồi `chr.special_effect.entries()`; vì crate đặt tên +0x44 là `unk_removal_timer` (trường private), cần đọc thô trường này (`const` assert như các chỗ khác) hoặc dùng `removal_timer` (+0x40, lệch một frame).

`SpEffectParam` có các cột (tên ở `tmp/csv/base/SpEffectParam.csv`) để lọc: `iconId` (icon trạng thái; 0 là không hiện icon), `effectEndurance` (thời gian hiệu lực, -1 là vĩnh viễn), `enableLifeTime`, `stateInfo` và `dispIconNonactive`.

## Hướng làm (đã làm cho kẻ địch, xem "Đã làm")

1. **Đọc** mỗi frame trong task `Target`, cho kẻ địch khoá: các entry có `param_data.icon_id != 0` và thời gian còn lại (+0x44) > 0, bỏ loại đã có thanh buildup, giữ tối đa vài cái, sắp theo thời gian còn lại. Bản đầu chỉ ghi log (`Effects: <param id> icon <iconId> <còn lại>s` khi tập hiệu ứng đổi) để xem kẻ địch có hiệu ứng nào đáng hiện.
2. **Icon:** sprite `MENU_StatusIcon_<iconId>` của `SB_Status_00`, nạp một lần cùng các sprite khác (thêm dải sprite theo `iconId` hoặc đọc theo yêu cầu qua `sprites.rs`).
3. **Vẽ** dưới các thanh buildup: icon, thời gian còn lại dạng `12.3s` (cỡ chữ 10 px như số trên thanh), tối dần khi sắp hết như ERBuffTimer; thêm khoá `Target.Effects`, mặc định cân nhắc.
4. **Tên:** bỏ qua cho gọn (chỉ icon và thời gian); nếu cần sau này thì tra theo kiểu `game_names.txt` của ERBuffTimer.

## Bố cục (người dùng chốt hướng, 2026-10-28)

Bản vẽ: `tmp/scratchpad/hud_layout3.png` (script `hud_layout3.py`, không nằm trong git); bản vẽ đầu xếp dọc (`hud_layout.png`) bỏ vì tốn chỗ: 8 effect người chơi cao 240 px, trong khi HUD gốc đã chiếm góc trên trái.

Kiểu xếp theo ERBuffTimer (người dùng, 2026-10-28): **mỗi effect là một ô xếp ngang, icon 16 px, thời gian ngay dưới icon** (chữ 10 px, ô cao khoảng 28 px, rộng khoảng 28 px). Tám effect chỉ chiếm một hàng thay vì 240 px.

- **Kẻ địch khoá:** dưới các hàng buildup trong cột bên phải, **căn phải** (ô đầu sát mép phải).
- **Người chơi:** ngay dưới hàng icon trạng thái có sẵn của game ở góc trên trái, **căn trái** (vị trí chỉnh được trong `[Layout]`). Làm sau phần kẻ địch.
- **Giới hạn chung cho cả hai bên (người dùng, 2026-10-28): một hàng tối đa 6 ô, tối đa 2 hàng, tức tối đa 12 effect**; quá thì bỏ các effect còn nhiều thời gian nhất hoặc ít quan trọng nhất (chọn khi làm). 7 ô x 28 px = 196 px vẫn vừa cột 220 px nhưng khít, 6 ô chừa lề; cột phải rộng theo `Target.Width` và scale nên 6 ô là con số cố định, ô co theo scale.
- Thời gian chỉ số nguyên, không thập phân (người dùng, 2026-10-28), để chữ dài nhất còn 4 ký tự: `24s`, `9s`, từ 60 giây trở lên `1:30`; hiệu ứng vĩnh viễn chỉ hiện icon.
- Hai khoá riêng: `Effects.Target` và `Effects.Player`.
- Chừa chỗ cho khối sát thương giữa người chơi (co-op) ở cột trái dưới hàng effect; vị trí khung đồng đội của game khi co-op chưa kiểm.

## Đã thử trong game (2026-10-28, 2.7.2.0)

Log `Effects: player ...` khi dùng Golden Vow và Lightningproof Pickled Liver:

- **+0x44 là giây còn lại thật:** `20501140 icon 20408 120s` rồi `67s` sau 53 giây; `1660000 icon 20406 80s` rồi `13s` sau 68 giây; effect biến mất đúng lúc hết giờ.
- **Có dòng phụ:** `1660001` và `1660002` (Golden Vow "Cycled", `effectEndurance` 0.1) có icon nhưng thời gian luôn 0s; chúng tồn tại cùng Golden Vow. Bỏ bằng bộ lọc `duration > 1` (trường `duration` +0x48 của entry); effect vĩnh viễn (không có thời gian) chưa gặp, hiện bị lọc luôn.
- **Hai effect khác nhau có thể trùng icon:** `20501140` và `1660002` cùng icon 20408 (icon "buff" chung của game).
- **Kẻ địch: effect có icon hiếm, effect của trạng thái nổ có thời gian nhưng không có icon.** Khi nổ độc, thối rữa, đóng băng, log (`Effects: enemy timed`, 2.7.2.0) cho thấy các SpEffect nội bộ có thời gian: `6511 30/30s` (Poison - Type 2 - Special 2) sau khi độc nổ, `6600 90/90s` (Scarlet Rot - Type 1 - Special 1), `6702 30/30s` (Frostbite - Type 1 - Special 3), thời gian đếm xuống đúng (`6600 85/90s`, `68/90s`). Cả ba có `iconId = -1` trong param, nên đọc theo "effect có icon" không thấy gì.
- **`505` (Poison/Scarlet Rot (Cycled), 900 giây, `stateInfo 275`)** xuất hiện cùng lúc với effect độc hoặc thối rữa và kéo dài 900 giây: dấu hiệu nội bộ, bỏ qua.
- **Effect mặc định có icon của kẻ địch (từ `NpcParam.spEffectID0..31` với `iconId > 0`, vanilla):** chỉ 8 effect. Sáu cái vĩnh viễn: 17354 và 17131 và 18917 (icon 20286, Kindred of Rot, Disciple of Rot, Kindred of Rot Larva), 13865 (icon 20261, Cleanrot Knight), 11720 và 10695 (icon 20285, Sanguine Noble, Mohg); hai cái 60 giây của DLC (20502025 Priestess Heart và 20502015 Rock Heart, icon 20406; Dragon Communion Priestess, Ancient Dragon-Man).

## Ghép effect nổ của kẻ địch với icon trạng thái (suy ra từ tên trong `SpEffectParam`, xác nhận trong game cho 6511, 6600, 6702)

Các dải id theo tên "`<Trạng thái> - Type N - Special M`":

| Trạng thái | Dải hàng | Thời gian (`effectEndurance`) | Hiện không |
|---|---|---|---|
| Độc (poison) | 6500-6515 | 30 hoặc 90 s (tuỳ loại) | có |
| Thối rữa (scarlet rot) | 6600-6605 | 90 s | có |
| Đóng băng (frostbite) | 6700-6705 | 30 s | có |
| Ngủ (sleep) | 6450-6460 | 60 s | có |
| Chảy máu (blood loss) | 6400-6415 | 1 s (tức thời) | không |
| Phát điên (madness) | 6750-6755 | 1 s (tức thời) | không |
| Tử huyết (death blight) | 6800-6805 | 0 (tức thời) | không |

Thứ tự trạng thái khớp `BUILDUP` của `target.rs`, nên ô của effect nổ dùng icon `MENU_RE_*` đã nạp cho thanh buildup (không cần sprite `MENU_StatusIcon_*`); đếm ngược theo `+0x44`. Với effect có `iconId`, dùng sprite `MENU_StatusIcon_<iconId>`.

## Đã kiểm (2026-10-28)

- **Danh sách:** hàm huỷ `CS::SpecialEffect` giống nhau ở 2.6.2.0 (4F327E), 2.7.0.0 và 2.7.1.0 (4F404E), 2.7.2.0 (4F4110): đầu danh sách ở `a1[1]` (+0x08), mỗi entry nối tới entry kế bằng `*(entry + 48)` (+0x30). `ChrIns.special_effect` +0x178 đã dùng ở các mod phát hành (SpeedMultiplier, SpiritMultiplier đọc `entries()` của người chơi) và ở IDA (`sub_1404FA370(player+0x178, ...)`, `mods/spiritmultiplier/docs/summon_anywhere.md`).
- **Bộ đếm (2.7.2.0):** hàm khởi tạo entry `sub_1404FFEC0` đặt +0x40, +0x44 và +0x48 bằng `effectEndurance` của dòng param (+0x08 của dòng) nhân một hệ số; hàm cập nhật mỗi frame `sub_140501ED0` **trừ +0x44** (kẹp ở khoảng 0) và chép giá trị tick trước sang +0x40. Vậy thời gian còn lại thật là **+0x44**; `fromsoftware-rs` gọi +0x40 là `removal_timer` và +0x44 là `unk_removal_timer`, nên cần đọc +0x44 (lệch một frame với +0x40 thì không đáng kể). Hệ số trừ có thể bị đổi theo cờ `0x20` ở +356 và +364 của dòng param (thời gian chạy nhanh hoặc chậm theo tham số), nên số giây còn lại là số đếm của game chứ không phải đồng hồ tường.
- **Icon:** `SpEffectParam.iconId` ứng với sprite `MENU_StatusIcon_<iconId>` (40x40) trong `SB_Status_00` của `menu/hi/01_common` (cùng gói với `SB_Condition`, đọc bằng `map/sprites.rs`). Trong `SpEffectParam` vanilla: 11354 dòng, 695 dòng có `iconId`, 150 icon khác nhau, 332 dòng có `effectEndurance > 0`.
- **Lọc:** cột `stateInfo` gần như vô dụng để lọc (499 trong 695 dòng có icon là 0); nên lọc theo `iconId > 0` và `effectEndurance > 0` (hoặc thời gian còn lại > 0).

## Việc cần kiểm trước khi tin

- Bộ đếm +0x44 trên 2.6.2.0, 2.7.0.0 và 2.7.1.0 (mới kiểm trên 2.7.2.0).
- Trong game: kẻ địch có những hiệu ứng nào có icon (bản đầu nên ghi log `Effects: <param id> icon <iconId> <còn lại>s` khi tập hiệu ứng đổi, như dòng `Buildup:`).
- Hiệu ứng vĩnh viễn (`effectEndurance` bằng -1) hiện thế nào: bỏ qua hoặc hiện không kèm thời gian.
- Trong co-op Seamless, thời gian còn lại của hiệu ứng trên kẻ địch có thể chỉ đúng với phần máy mình biết, như buildup (`docs/buildup.md`).
- Hiệu ứng trạng thái đã kích hoạt (Scarlet Rot, Frostbite... đang chạy) có icon riêng không, và có trùng với thanh buildup đang hiện không.

## Đã làm (2026-10-28)

- `target.rs`: `read_effects` lấy effect có `iconId > 0` hoặc là effect nổ (`proc_kind`: độc 6500-6515, thối rữa 6600-6605, đóng băng 6700-6705, ngủ 6450-6460) và (vĩnh viễn hoặc `duration > 1`); `shown_effects` giữ tối đa 12 (bỏ cái còn nhiều thời gian nhất, vĩnh viễn trước), thứ tự game; log `Effects: enemy|player ...` khi tập effect đổi.
- `map/sprites.rs` và `map/layer.rs`: nạp mọi sprite `MENU_StatusIcon_<id>` (`SB_Status_00..03`) thành một lưới ô 40 px (`status_icon(id)`).
- `ui.rs`: `draw_effects`: ô 28 px (icon 16 px, thời gian 10 px dưới icon, `24s` hoặc `1:30`), căn phải, 6 ô một hàng, icon mờ dần trong 5 giây cuối; effect nổ dùng icon `MENU_RE_*` của thanh buildup.
- Khoá `Target.Effects` (mặc định bật) trong `JourneyHUD.toml` và menu.

## Cỡ icon (đo trên ảnh chụp 2560x1600, 2026-10-28)

Khung hình game là vùng 16:9 (2560x1440), nên 1 px 1080p = 1,333 px ảnh.

- **Icon trạng thái của game (hàng trên cùng bên trái, hình thoi có mũi tên):** rộng khoảng 38 px ở 1080p, cách nhau 42 px, từ x 137, y 105 đến 138. Hàng effect của người chơi đặt ngay dưới: gốc (140, 146).
- **Bản đầu của mod (icon 16 px):** đo được khoảng 15 đến 16 px, tức nhỏ hơn icon của game khoảng 2,4 lần; người dùng thấy hơi nhỏ.
- **Hiện tại (người dùng chốt: giảm số ô miễn là icon ngang icon của game):** icon **38 px**, ô 42 px (đúng cỡ và bước của hàng icon trạng thái của game), chữ thời gian 11 px, hàng cao 54 px, tối đa 2 hàng. Số ô mỗi hàng của kẻ địch tính từ bề rộng thanh: `floor(width / 42)`, tối đa 6 (220 px cho 5 ô, tức tối đa 10 effect); hàng của người chơi 6 ô (tối đa 12). Quá thì bỏ các effect còn nhiều thời gian nhất.

## Hàng effect của người chơi

`target::own_effects_now` (snapshot riêng `OWN`, đọc mỗi frame) và `ui::draw_effects` với `Cells { right_aligned: false }`; không có effect nổ của buildup (game có icon riêng cho người chơi), nên chỉ hiện effect có `iconId` (cả vĩnh viễn, không có số). Khoá `Effects.Player` (mặc định bật), cùng giới hạn 6 ô một hàng, 12 ô.

## Kẻ địch không có effect có icon

Black Blade Kindred (`NpcParam 47701242`) chỉ có các effect "NPC: Disable Poison / Scarlet Rot / Blood Loss / Death Blight / Frostbite / Madness" (miễn nhiễm, `iconId = -1`) và các effect của AI/scale; nên không có ô nào: đúng như thiết kế. Có thể hiện hàng "miễn nhiễm" sau này nếu muốn.

## Chỉnh sau lần thử đầu (2026-10-28)

- Chữ giờ và `Deaths` cách vành minimap 4 px (`TEXT_GAP`); thanh máu bên dưới dịch xuống theo.
- Icon effect của **kẻ địch nhỏ lại 30%** (hệ số 0,7: icon 27 px, ô 30 px, chữ 10 px, 6 ô một hàng vì cột 220 px chứa được 7); hàng của người chơi giữ 38 px, đúng cỡ icon của game.

## Hàng icon trạng thái của game (nghiên cứu để hợp nhất với hàng của mod, 2026-10-28, 2.7.2.0)

Ý người dùng: thay vì vẽ hàng thứ hai (icon lặp), chèn thời gian vào chính hàng icon trạng thái của game. Đã tìm được:

- **Movie:** `menu/01_000_fe.gfx` (1,6 MB; trích bằng test `#[ignore] hud_movie_from_the_installed_game` trong `map/sprites.rs`, ghi ra `ER_GFX_OUT`; công cụ dựng cây clip `tmp/scratchpad/swf_tree.py`). Hàng icon là `PlayerHUD/Gauge/Equip/Fade/Item_0_0 .. Item_0_41`: **42 ô xếp một hàng ngang, không xuống dòng**, ô i ở x = 10 + 42 x i, y = 15 (đơn vị 1080p, cách nhau đúng 42 px như đo trên ảnh). Mỗi ô chỉ có clip `Icon` (một hình, ảnh gắn lúc chạy); **không có ô chữ**.
- **Lớp của game:** `CS::PlayerStateIconViewParts` (vtable RVA 0x2AEFD18, 2.7.2.0). Constructor `sub_1408D5470` gom các ô `Fade/Item_0_%d` (tìm tới khi thiếu) vào một vector (đầu `+112`, cuối `+120`, ô 304 byte, clip `Icon` ở `+120` của ô). Hàm cập nhật mỗi frame **`sub_1408D58F0(view, a2, model)`** (gọi từ `sub_1408CA1C0` với `view = hud + 126608` và `model = *(hud + 96) + 14344`).
- **Khối dữ liệu (`model`, 2736 byte):** 42 slot x 64 byte tại `model + 64 x i` (mô tả ảnh: gắn vào clip `Icon` bằng `sub_1407361C0`), cờ **`Flicker`** từng slot tại `model + 2688 + i` (icon nhấp nháy, nhiều khả năng khi sắp hết hạn), cờ chung tại `model + 2730`. Hàm điền khối này từ SpEffect của người chơi chưa tìm ra.
- **Quan sát:** Golden Vow (`1660000`, `1660001`, `1660002`) cộng Pickled Liver (`20501140`, icon 20408 trùng `1660002`) là 4 effect có icon nhưng game hiện **3 icon**: có vẻ game gộp theo icon.

Còn thiếu (đã làm xong ở mục "Giải mã khối dữ liệu" bên dưới): đọc được `model` (hook `sub_1408D58F0` bằng inline hook để giữ con trỏ, hoặc tìm đường từ singleton), giải mã mô tả 64 byte ra icon id, và cách chọn thời gian khi nhiều effect chung một icon. Hướng làm gọn nhất: giữ nguyên hàng của game, vẽ chữ thời gian bằng ImGui ngay dưới từng icon (x = gốc + 42 x slot) theo mô tả của slot; không cần thêm ô chữ vào movie.

## Giải mã khối dữ liệu của hàng icon (log thử game 2026-10-28)

`state_icons.rs` chuyển hướng lệnh gọi `sub_1408D58F0` (AOB duy nhất trên 2.6.2.0 đến 2.7.2.0, như `HubHpBars`) để giữ con trỏ `model`. Log `Icons:` khi dùng Golden Vow và Pickled Liver:

- **Icon id nằm ở từ cuối (byte +60) của slot 64 byte:** `4FB6` = 20406, `4FB7` = 20407, `4FB8` = 20408. Slot trống có `FFFFFFFF` ở đó (từ thứ 15 là `FFF1E702` khi có icon, `FF6F8F00` khi trống).
- **Slot điền liền từ 0, không khoảng trống,** theo thứ tự danh sách SpEffect; khi một effect hết hạn các slot sau dồn lên (`[0]=20407,[1]=20408` rồi `[0]=20408`).
- **Gộp theo icon:** Pickled Liver (`3910`, icon 20408) và hàng phụ Golden Vow `1660002` (icon 20408) chỉ chiếm một slot. Hàng phụ "Cycled" (`1660001`, 20407; `1660002`, 20408) vẫn có slot, game hiện chúng như icon thường.

## Cách làm hiện tại cho người chơi (một danh sách duy nhất)

Mod **không vẽ hàng icon của người chơi nữa**; `ui::draw_icon_times` vẽ chữ thời gian ngay dưới từng icon của game: slot k ở tâm x = 155,5 + 42 x k, chữ ngay dưới mép dưới của hình thoi (y 142, đơn vị 1080p trong khung hình của game; số đo từ ảnh chụp). Với icon của slot, lấy effect có icon đó còn **nhiều thời gian nhất** (hàng phụ 0,1 giây đã lọc nên các icon chỉ có hàng phụ không có chữ); effect vĩnh viễn không có chữ. Khoá `Effects.Player` bật tắt các chữ này. Giới hạn: vị trí tính theo bố cục HUD mặc định (nếu người chơi đổi cỡ HUD trong tuỳ chọn game thì chữ lệch); chưa thử.

## Ẩn cùng hàng icon của game

Game ẩn hàng icon (và thanh HP/FP) khi không đánh nhau; ảnh chụp cho thấy chữ thời gian vẫn hiện trong khi icon đã mờ. Hàm cập nhật `sub_1408D58F0` truyền cờ ở `model + 2730` (byte ngay sau 42 cờ nhấp nháy) vào `sub_14076B850(view + 136, cờ, ...)`, hàm này phát hoạt ảnh `FadeIn` khi cờ khác 0 và `FadeOut` khi bằng 0 (đọc từ IDA, 2.7.2.0). `state_icons::shown()` đọc cờ đó; `ui.rs` giữ `icons_alpha` (cùng tốc độ mờ `FADE_RATE` của panel) và nhân vào chữ. Đã thử trong game (2026-10-28): chữ mờ cùng hàng icon khi game tự ẩn HUD.

## Thanh buildup của người chơi (2026-10-29)

Thanh có icon ở giữa phía dưới màn hình là `ConditionGaugeList/Item_*` của movie HUD, một clip cho mỗi trong 7 trạng thái (cùng thứ tự `BUILDUP`). Nó vừa hiện phần tích luỹ vừa hiện phần còn lại của trạng thái sau khi nổ.

- **Cỡ (đo trên ảnh chụp, cửa sổ 1600x900, quy về 1080p):** icon khung vuông khoảng **42 px**, thanh dài khoảng **292 px**, cao khoảng **14 px**. So sánh: icon trạng thái hình thoi của game 38 px, ô effect của người chơi do mod vẽ 38 px, icon effect của kẻ địch khoảng 29 px (hệ số 0,77 của 38).
- **Cập nhật:** `sub_1408D2AE0(view, index, model)` gọi cho từng trạng thái trong `sub_1408CA1C0` (view `hud + 80848 + 1144 x i`, `model = *(hud + 96) + 9888 + 104 x i`). Hàm đặt `FadeIn`/`FadeOut` theo byte `model[65]` (cùng `sub_14076B850` như hàng icon), phần còn lại dùng cho độ đầy. AOB của lệnh gọi duy nhất trên 2.6.2.0 đến 2.7.2.0.
- **Cách ẩn:** `state_icons::install_gauge` chuyển hướng lệnh gọi đó; khi `Effects.Player` bật **và người chơi đang dính trạng thái đó** (có effect nổ tương ứng, `AFFLICTED` do `target.rs` cập nhật mỗi frame, bit k ứng với `BUILDUP[k]`), bản sao 104 byte của `model` với `model[65] = 0` được đưa vào hàm gốc, nên thanh mờ đi. Trong lúc mới tích luỹ thanh vẫn hiện; tắt `Effects.Player` thì thanh luôn như cũ. Thứ tự 7 clip của thanh khớp `BUILDUP` là giả định (chưa kiểm).
- **Hiện trong danh sách effect:** effect nổ của người chơi (độc 6500-6515, thối rữa 6600-6605, đóng băng 6700-6705, ngủ 6450-6460) không có icon trong hàng của game, nên `ui::draw_icon_times` vẽ chúng sau các slot của game, mỗi trạng thái một ô (icon `MENU_RE_*` 38 px, giờ đếm ngược ở dưới, lấy effect còn nhiều thời gian nhất). Phần tích luỹ trước khi nổ vẫn hiện bằng thanh của game.

## Nhận diện effect nổ theo `stateInfo` (2026-10-29)

Thử game: thanh buildup của người chơi vẫn đếm thời gian sau khi nổ và không có ô effect, vì log `Effects: player` không có effect nổ nào. Nguyên nhân: effect thối rữa người chơi nhận ở đầm (Caelid, Lake of Rot) là hàng `4051`, `4053`, `4055` ("Scarlet Rot Swamp", 180 giây), còn đòn của kẻ địch dùng `21000-21102` ("NPC: Scarlet Rot", 180 giây) và `107000-107275` (90 giây); dải `6500-6705` chỉ là những hàng của vũ khí và vài nguồn khác. Dải id không bao hết được.

Cột `stateInfo` của `SpEffectParam` cho đúng loại trạng thái bất kể hàng nào (đếm hàng trong vanilla): **2 độc** (309 hàng), **5 thối rữa** (227), **6 chảy máu** (284), **116 tử khí** (210), **260 đóng băng** (276), **436 ngủ** (224), **437 phát điên** (65). Các hàng chữa trạng thái có giá trị khác (10, 11, 12, 13, 118, 276...) và 275 là dấu nội bộ của game (`505`). `target::proc_kind` nay đổi `stateInfo` ra chỉ số `BUILDUP`, chỉ cho effect không có icon và có thời gian (`duration > 1`, vĩnh viễn thì bỏ); bảng dải id ở mục trước chỉ còn là ví dụ.

## Thanh buildup: clip theo trạng thái đang hoạt động, không theo loại (2026-10-29)

Thử game: thanh độc ẩn đúng, thanh thối rữa không. Log `Gauge:` (độc, từ thứ 17 của model giảm dần theo giờ còn lại của effect nổ) chỉ có clip `[0]`, với từ đầu của model bằng 1, `model[64] = 1` (đã nổ, đang đếm giờ) và `model[65] = 1` (hiện). Từ đầu là **khung hình của clip `Icon`** (7 hình trong movie, theo thứ tự `BUILDUP`: 1 độc, 2 thối rữa...). Game phát các clip cho những trạng thái đang hoạt động theo lượt, như hàng icon, nên chỉ số clip không nói lên trạng thái: thối rữa đứng đầu cũng ở clip 0 chứ không phải clip 1 (giả định trước đó sai, trùng đúng với độc vì độc ở vị trí 0).

`gauge_update` nay lấy `kind = model[0] - 1` và chỉ ẩn thanh khi người chơi đang dính trạng thái `kind` (`AFFLICTED`). Log `Gauge:` giờ ghi `frame`, `state`, `shown` của từng thanh đang hiện. Chưa thử với thối rữa.
