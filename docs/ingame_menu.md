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

## Mẫu client trong repo

`shared/src/erui.rs` (`common::erui`): `connect`, `register_provider`, `add_submenu`, `add_slider`, `add_toggle`,
`commit`, `set_row_value`/`get_row_value`; mod thiếu host thì ghi log và chạy bằng ini. Mẫu dùng:
`mods/runemultiplier/src/menu.rs` (menu "Rune Multiplier" -> Multiplier 0..10, Log file; thread ghi ini sau khi
dừng thao tác 1 s; F5 reload đồng bộ lại hàng). Chưa có `add_inline_choice`, `add_text_input` trong `common::erui`.

## Nghiên cứu nền (IDA 2.7.1.0, đối chiếu với ERNativeUI)

- `0x959DF0` dựng trang Game Options; `0x94A590` thêm hàng slider, `0x94A140` thêm hàng bool,
  `0x94A2C0` inline choice; chữ tra theo ID `GR_MenuText` (`0x7617C0`) / `GR_LineHelp` (`0x7615E0`) qua
  `0x764220` (ERNativeUI hook hàm này để chèn chữ riêng). Mảng hàng của trang giới hạn 16 (`0x928630`).
- `mod-settings-menu.dll` của bên thứ ba (mã đóng, mẫu trong `tmp/menu_in_game`) chỉ chạy trên 2.7.1.0, API chỉ
  slider số; Solid_Uncapper dùng GFX loose khác hướng. Không dùng cả hai.
