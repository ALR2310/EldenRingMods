# JourneyHUD: thanh tích tụ trạng thái (buildup) của kẻ địch

**Status 2026-10-27: nghiên cứu xong và đã có code (`Target.Buildup`, `target.rs::read_buildup`, `ui.rs::draw_buildup`), chưa thử trong game. Vị trí dữ liệu và bố cục module đã kiểm tĩnh trong IDA trên 2.6.2.0, 2.7.0.0, 2.7.1.0, 2.7.2.0; chiều của giá trị (đếm xuống từ tối đa) và cách nhận kẻ địch miễn nhiễm còn là suy ra từ souls_vision, cần thử trong game.**

Mục tiêu: dưới thanh máu và posture của kẻ địch đang khoá, hiện 7 thanh tích tụ (độc, thối đỏ, chảy máu, tử khí, đóng băng, ngủ, điên loạn) giống cách souls_vision làm, bằng icon và khung của chính game.

## souls_vision làm thế nào (chỉ lấy ý tưởng, GPL-3.0, không copy code)

Đọc `tmp/repos/souls_vision` (`effect_bar.cpp`, `overlay.cpp`, `structs/chr_resist_module.h`):

- Dữ liệu nằm ở module resist của chính kẻ địch: `ChrIns.modules` (+0x190) -> con trỏ thứ 5 (+0x20) -> `CSChrResistModule`. Bảy giá trị hiện tại ở +0x10 và bảy giá trị tối đa ở +0x2C, thứ tự: độc, thối đỏ (scarlet rot), chảy máu, tử khí (death blight), đóng băng, ngủ, điên loạn.
- Giá trị trong module **đếm xuống**: thanh hiện `tối đa - giá trị`, tức phần đã tích tụ. Chỉ vẽ thanh khi tối đa > 0 và phần tích tụ > 0, nên kẻ địch chưa dính gì thì không có thanh nào.
- Kẻ địch miễn nhiễm một loại: lấy từ `NpcParam` (`resist_poison`, `resist_desease` = thối đỏ, `resist_blood`, `resist_curse` = tử khí, `resist_freeze`, `resist_sleep`, `resist_madness`); giá trị >= 999 coi là miễn nhiễm, vẽ riêng (icon mờ), không có thanh.
- Mỗi thanh gồm icon loại, khung (`ConditionWaku`), nền, thanh (`Bar` tô màu theo loại), vạch sáng ở đầu và chữ `giá trị / tối đa`.

## Đã kiểm trong IDA (cả 4 bản exe)

- Hàm khởi tạo `CSChrResistModule` giống hệt ở 4 bản (RVA 43D790 / 43DCF0 / 43DCF0 / 43DD80): vtable gán rồi một vòng 7 lần xoá lần lượt bảy mảng 7 phần tử ở `+0x10`, `+0x2C`, `+0x48`, `+0x64`, `+0x80` (số nguyên) và `+0x9C` (số thực, khởi tạo 1,0), cuối cùng `+0xB8` (dword) và `+0xBC` (byte). Khớp bố cục của souls_vision: giá trị hiện tại +0x10, tối đa +0x2C.
- Con trỏ module ở +0x20 của `ChrInsModuleContainer` (`resist: usize`, thứ 5 sau `data`, `action_flag`, `behavior_script`, `time_act`) cũng khớp `fromsoftware-rs`. Crate chưa có kiểu cho module này, nên phải đọc offset thô như `target.rs` đọc poise.
- Chưa kiểm: ba mảng +0x48, +0x64, +0x80 là gì (có thể ngưỡng hay hệ số), và chiều đếm xuống của hai mảng đầu. Hàm cập nhật module không đi qua vtable của chính nó (vtable chỉ có hàm huỷ), nên cần tìm chỗ ghi `+0x10` từ phía hệ thống sát thương/trạng thái.

## Icon và khung của game (không cần đóng gói ảnh)

Menu sprite sheet của game đã có sẵn, đọc được bằng cách `map/sprites.rs` đọc các sprite khác:

- `SB_Condition`: `MENU_RE_Poison`, `MENU_RE_Plague` (thối đỏ), `MENU_RE_bleed`, `MENU_RE_Death` (tử khí), `MENU_RE_Cold` (đóng băng), `MENU_RE_Sleep`, `MENU_RE_Mad` (điên loạn); mỗi cái khoảng 126 x 108 px.
- `SB_FE`: `MENU_FL_ConditionWaku` (khung 840 x 48) và `MENU_FL_ConditionWaku_L` (nắp trái 12 x 48).
- `SB_FE_01`: `MENU_FL_Condition_Bar` và `MENU_FL_Condition_BarBlack` (thanh và nền, 820 x 26), cùng họ với `Bar.png` ta đang dùng cho thanh máu.

Đây là các thành phần của thanh trạng thái người chơi trong HUD game, nên vẽ giống HUD hơn là bản souls_vision tự đóng gói.

## Hướng làm (đã làm, chưa thử trong game)

1. Đọc: trong `target.rs` thêm đọc module resist của kẻ địch khoá (kiểm con trỏ, đọc an toàn): 7 giá trị hiện tại và 7 tối đa, `const` assert offset như các chỗ khác.
2. Lọc: hiện thanh khi tối đa > 0 và tối đa < 999 (không miễn nhiễm) và phần tích tụ > 0; sắp theo thứ tự cố định, tối đa vài thanh cùng lúc.
3. Vẽ: dưới thanh posture, mỗi thanh một hàng nhỏ (icon + thanh theo màu loại + số), dùng sprite `SB_Condition` và `Condition_Bar`; sprite nạp một lần cùng các sprite khác trong `ICON_SPRITES`.
4. Khoá cấu hình `Target.Buildup` (mặc định bật, cũng trong menu game). Log ghi một dòng `Buildup: <loại> <đã tích>/<tối đa>` mỗi khi tập loại đang tích tụ đổi, để xem giá trị thật cư xử thế nào.

Bố cục vẽ (`ShowNumbers` bật thì mỗi thanh có `đã tích / tối đa` ở giữa, cùng cỡ chữ 10 px như số máu): mỗi loại một hàng 17 px (1080p) dưới đường be: icon `SB_Condition` cao 16 px (đã cắt lề trong suốt của sprite, chỉ còn phần hình; sprite gốc 126x108 có lề nên trước khi cắt icon trông nhỏ hơn và cách thanh vài px), thanh dày 12 px (bằng thanh máu) tô màu theo loại nối liền ngay sau icon (không cách), hai icon trên dưới cách nhau 1 px; thanh posture dày 12 px như thanh máu; không số.

## Cần thử trong game trước khi tin

- Giá trị module thật sự đếm xuống từ tối đa (đánh một kẻ địch bằng vũ khí độc hoặc chảy máu rồi theo dõi).
- Ngưỡng miễn nhiễm: đánh một boss miễn nhiễm (vd. một loại không dính chảy máu) xem tối đa trong module ra bao nhiêu, có phải >= 999 hay 0.
- Hành vi trên boss nhiều pha, kẻ địch bị hạ, kẻ địch đổi giữa lúc khoá.
- **Co-op (Seamless):** người dùng thấy souls_vision hiện buildup không đồng bộ giữa hai người chơi (2026-10-28), đoán là mỗi người tự gây buildup riêng chứ không cộng dồn. Cả hai mod đọc module resist của bản kẻ địch **trên máy mình**, nên chỉ thấy phần máy mình biết; trạng thái thật quyết định ở máy chủ. Chưa kiểm chứng: cho hai máy cùng đánh một kẻ địch và so dòng log `Buildup:` của hai bên (nếu máy chủ thấy tổng còn máy khách chỉ thấy phần mình thì đúng như đoán).
