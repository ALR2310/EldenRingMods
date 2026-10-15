# Menu trong game (ERNativeUI)

**Status 2026-10-13: hướng đã chốt = dùng ERNativeUI làm phụ thuộc tùy chọn; đã thử trong game với RuneMultiplier (trang con + slider + công tắc), kết quả tốt. Chưa phát hành, chưa mod nào khác dùng.**

Đường đi đã bỏ: ImGui qua hudhook (nhánh `feat/modmenu`, tạm dừng 2026-10-02: xung đột hudhook với
SoulsTeleport/SoulsChat); tự dịch ngược và tự dựng menu native (ERNativeUI đã làm sẵn, mã mở).

## ERNativeUI

https://github.com/Flammrock/ERNativeUI (MIT, Flammrock). Host `ERNativeUI.dll` dùng
chung + SDK C ABI (`erui.h`). Release 1.0.0 (2026-08-30), 1.1.0 (09-06), 1.1.1
(09-11). Mã nguồn đã clone ở `tmp/ERNativeUI` (git-ignored); bản phát hành giải nén ở
`tmp/` hoặc scratchpad.

- Hook hàm dựng trang menu của game (không ImGui, không file GFX bắt buộc); các mod gọi API khai báo lúc khởi động.
- Chỉ hỗ trợ exe **2.7.0.0 và 2.7.1.0** (nhận theo PE timestamp + SizeOfImage, fail-closed). Không có
  2.6.2.0. Chỉ offline, tắt EAC. Tương thích Solid Uncapper có tài liệu (tải Solid Uncapper trước).
- Control: Toggle, Slider, Button, Submenu, Inline/Popup choice, TextInput, ColorPicker, input bindings,
  storage INI, dialog. Trang đích: Game Options (mặc định) và Camera/Display/Sound/Network/Keyboard/Graphics.

### Ràng buộc đã kiểm bằng tài liệu + source

- **Slider**: giá trị là **một byte** (0..255) nằm trong bộ nhớ của game; `minimum >= 0`, `maximum <= 255`,
  `step > 0`. Số hiển thị là chính byte đó nên **không có số thập phân/số âm**. Muốn số lẻ: Inline/Popup
  choice (1-32 nhãn tuỳ ý) hoặc TextInput (tự parse, tối đa 35 ký tự UTF-16, host không kiểm nội dung;
  chưa thử gõ `.` và `-` trong editor của game).
- **Mỗi hàng đúng một widget**; không ghép 2 ô trên 1 hàng.
- **Đăng ký chỉ trong cửa sổ khởi động** (yên 750 ms, tối đa 5 s). Phải connect + register ngay đầu thread
  khởi động của mod; chậm là `ERUI_REGISTRATION_CLOSED`.
- **Callback** slider/toggle/choice chạy ở thread worker của host (nút bấm: thread UI): ngắn, không panic,
  không block. `set_row_value` từ phía mod không gọi lại callback.
- `provider_id` duy nhất toàn tiến trình (`[A-Za-z0-9_.-]`, 1-255), `owner_module` = HMODULE của mod.
- Host không tự lưu giá trị: mod tự ghi ini (hoặc dùng storage của host).

### Kết quả thử trong game (2.7.1.0, 2026-10-12/13)

- Hàng của mod hiện ở System > Game Options; trang con mở được; slider/công tắc hoạt động.
- **Không cài GFX**: hàng dạng nút (submenu/button) **mất nhãn bên trái** (chỉ còn chữ trong nút); Game Options chỉ
  còn **2 chỗ trống** cho mod (6 hàng vanilla - 4 của game).
- **Có GFX** (`menu/win/02_040_optionsetting.gfx`, `02_042_pc_graphicsetting.gfx` đặt trong thư mục ModEngine
  mod loader, vd. `Game\mod\menu\win\`): nhãn hiện cả bên trái và trong nút; chỗ tăng lên **13** (9 trống);
  13 hàng của mod đã thử và host phân trang bằng hàng Next/Previous.
- **Phân trang là chuyển trang rời, không cuộn**: sức chứa trang là số vị trí hàng cố định trong GFX của game.
- GFX thay thế file của game, xung đột với mod khác sửa cùng file (Solid Uncapper...): **không nên bắt buộc**.
  Thiết kế: mỗi mod 1 hàng trang con (tên chứa tên mod, vì nhãn trùng nhau giữa các mod), mọi setting nằm trong.

- Slider số âm / thập phân: ERNativeUI 1.2 có sẵn Numeric Slider -> [ingame_menu_slider.md](ingame_menu_slider.md). Bố cục từng mod: [ingame_menu_layouts.md](ingame_menu_layouts.md). Menu cuộn thay vì phân trang: [ingame_menu_scroll.md](ingame_menu_scroll.md).

## Mẫu client trong repo

`shared/src/erui.rs` (`common::erui`), mod thiếu host thì ghi log một dòng và chạy bằng file cấu hình như cũ.

- **Lớp thấp**: `connect`, `register_provider`, `add_submenu`, `add_slider`, `add_toggle`, `add_inline_choice`, `add_text_input`, `commit`,
  `set_row_value`/`set_text`. `commit` thất bại thì bản nháp bị huỷ (`Drop`), vì host vẫn giữ nó.
- **Lớp dùng cho mod** (mỗi `menu.rs` chỉ còn phần bố cục): `static MENU: OnceLock<Menu>`; `fn build(&mut MenuBuilder)` dựng hàng bằng
  `submenu`/`toggle`/`choice`/`slider`/`number` (mỗi hàng kèm hàm `read` đọc giá trị từ cấu hình); `spawn_menu(&MENU, hmodule, id, tên, build)`
  đăng ký trên thread riêng (gọi thật sớm trong thread khởi động của mod: host chỉ nhận đăng ký ở vài giây đầu); `Menu::sync()` đưa giá trị
  cấu hình vừa nạp lại vào các hàng (gọi sau reload).
- **Lưu về file**: `Setting::flag(key)` / `Setting::int(key)` (hoặc `Setting { format, apply }`) làm `user_data` của hàng cùng callback
  `setting_changed`; `start_saver(ini_path, on_saved)` (hoặc `start_saver_with(ghi, on_saved)` cho TOML qua `TomlConfig::save_values`) chạy 1 thread ngủ cho đến
  khi có thay đổi, gộp theo khoá, ghi file (nguyên tử: ghi tệp tạm rồi đổi tên) và map cấu hình sau 1 s yên rồi gọi `on_saved`.
  Mod áp lại qua `RELOAD_GENERATION` dùng `on_saved = Some(erui::reload_from_menu)`: lần đồng bộ ngay sau đó của `Menu::sync` bị bỏ qua để hàng
  người chơi đang sửa không bị đặt lại.
- **Số**: `NumberSetting` + `MenuBuilder::number`: gõ `1.5` (dấu phẩy cũng được), kẹp vào [min, max], chữ không phải số thì trả về giá trị cũ.
  Key mod đọc bằng `config::get_int` phải dùng `NumberSetting::whole` (làm tròn, vì `1.5` làm `get_int` hỏng và rơi về mặc định); key TOML kiểu float
  dùng `NumberSetting::float` (ghi `2.0`); số hiển thị làm tròn 6 chữ số thập phân (số qua `f32` hiện `1.2`). Đã thử trong game (2026-10-14): gõ được `.`, `,`, `-`.
  `NumberSetting::slider(từ, đến, bước)` là dạng dùng khi phạm vi mod chấp nhận rộng hơn nhiều phạm vi slider (vd. 0..1000 nhưng slider 0..10): ô gõ dự phòng và giá trị trong tệp vẫn nhận số rộng, giá trị ngoài slider hiện ở mép gần nhất và tệp chỉ đổi khi người chơi kéo.
  `NumberSetting::stepped(bước)` biến hàng thành **Numeric Slider** native (API 1.2, ERNativeUI >= 1.2) khi phạm vi có 2..256 vị trí (bước chạm đúng `max`, tối đa 6 chữ số thập phân);
  host cũ (API 1.1) hoặc phạm vi không hợp thì vẫn là ô gõ. Chữ menu tiếng Việt: `common::i18n` + `locales/vi.toml` của mod, xem [shared/docs/i18n.md](../shared/docs/i18n.md). `connect` xin API 1.2 trước, lùi về 1.1. Giá trị ini nằm ngoài lưới bước chỉ làm tròn khi hiển thị. SpeedMultiplier dùng bước 0.1 cho 0.1..10 (chưa thử trong game).
- Mod gate "đã vào thế giới" phải giữ yêu cầu reload đến khi vào thế giới (xem `dropmultiplier/src/drop_rate.rs`).
- Mẫu: `mods/runemultiplier/src/menu.rs` (áp ngay trong `apply`, file lưu sau), `mods/dropmultiplier/src/menu.rs` (nhóm, hàng chọn, reload).
  Bố cục mọi mod: [ingame_menu_layouts.md](ingame_menu_layouts.md).

## Nghiên cứu nền (IDA 2.7.1.0, đối chiếu với ERNativeUI)

- `0x959DF0` dựng trang Game Options; `0x94A590` thêm hàng slider, `0x94A140` thêm hàng bool,
  `0x94A2C0` inline choice; chữ tra theo ID `GR_MenuText` (`0x7617C0`) / `GR_LineHelp` (`0x7615E0`) qua
  `0x764220` (ERNativeUI hook hàm này để chèn chữ riêng). Mảng hàng của trang giới hạn 16 (`0x928630`).
- `mod-settings-menu.dll` của bên thứ ba (mã đóng, mẫu trong `tmp/menu_in_game`) chỉ chạy trên 2.7.1.0, API chỉ
  slider số; Solid_Uncapper dùng GFX loose khác hướng. Không dùng cả hai.
