# JourneyHUD: kéo thả minimap và panel mục tiêu

**Status 2026-11-03: mới nghiên cứu, chưa viết mã. Kết luận: làm được, nhưng phần khó là cho người chơi một con trỏ chuột tự do mà không làm game xoay camera hay xung đột với DLL khác; vị trí và lưu cấu hình thì đã có sẵn gần hết. `souls_vision` có kéo thả nhưng chỉ kiểu "mở menu game để có con trỏ" (hướng C). Đề xuất chia giai đoạn (phím mũi tên hoặc hướng C trước, kéo thả có bắt chuột sau).**

## Hiện trạng

- **Một cột, một neo.** Minimap và thanh mục tiêu nằm trong cùng một cột, neo ở góc trên phải của khung 16:9 (`Placement.area`): `Layout.OffsetX` (cách mép phải, âm thì dịch ra ngoài, hợp với màn ultrawide) và `Layout.OffsetY` (cách mép trên), tính theo đơn vị 1080p nhân `res = area.h / 1080`. Minimap: `radar_ui.rs` (`center = [area.x + area.w - offset_x * res - radius, area.y + offset_y * res + radius]`). Thanh mục tiêu: `ui.rs::draw_target`, đặt ngay dưới minimap bằng `y = top + offset_y * res + radar_ui::column_height(res)` (`column_height` co lại khi minimap ẩn nên thanh trượt lên).
- **Đã có đường sửa và lưu offset:** hai hàng slider `Layout.OffsetX/OffsetY` trong menu ERNativeUI (`menu.rs`) gọi `settings::set_offset_x/y` và `config::save(&[("Layout.OffsetX", ..)])` (giữ nguyên mọi comment của template), phím reload nạp lại. Kéo thả chỉ là một cách nhập khác cho hai số này.
- **Chưa có chuột:** `ui.rs` không đọc chuột, không dùng `before_wnd_proc`/`message_filter` của hudhook, và JourneyHUD link `common` **không có feature `overlay`** (tự dùng hudhook, không dùng `shared/src/overlay`).

## Vì sao chuột là phần khó

Trong chơi bình thường game ẩn con trỏ, giữ nó ở giữa cửa sổ (`SetCursorPos`, `ClipCursor`) và đọc chuột tương đối (`WM_INPUT`, DirectInput, `DLUID::MouseDevice::Poll`) cho camera. Muốn kéo thả cần: (1) thả con trỏ, (2) đọc toạ độ tuyệt đối, (3) chặn chuột khỏi game để camera không xoay và nhát chuột không thành đòn đánh. `shared/src/overlay` (dùng ở `soulsteleport`, `toolbox`) đã giải đủ ba việc (xem `shared/docs/overlay.md` mục 3 và `mods/soulsteleport/docs/menu_ui.md` về các lỗi gặp phải: mất con trỏ ở cửa sổ nhỏ, raw input ghi đè toạ độ, DirectInput), nhưng chúng gắn với khung menu của overlay đó (`MENU_OPEN`, `feed_mouse` là hàm private) nên JourneyHUD chưa dùng lại được ngay.

## Các hướng

| Hướng | Cách làm | Ưu | Nhược |
|---|---|---|---|
| **A. Phím mũi tên (giai đoạn 1)** | Một phím vào chế độ chỉnh (vd. `General.EditKey`); vẽ khung quanh minimap và thanh mục tiêu; Tab chọn khung, mũi tên dịch 1 px, Shift dịch 10 px; phím chỉnh lần nữa thoát và lưu | Không hook gì thêm, không đụng DLL khác (ERSS, Toolbox), chạy mọi bản game | Không phải kéo thả |
| **B. Kéo thả chuột có hook (giai đoạn 2)** | Chế độ chỉnh bật "bắt chuột": thả con trỏ, vẽ con trỏ ImGui, đọc chuột bằng `GetCursorPos` + `ScreenToClient`, chặn chuột khỏi game; kéo khung, nhả chuột thì `config::save` | Đúng ý muốn | Cần các hook của overlay chung, xem rủi ro bên dưới |
| **C. Kéo khi menu game mở** | Chế độ chỉnh vẽ khung trong cả lúc menu game mở (khi đó con trỏ của game hiện); ImGui nhận chuột khi con trỏ trên khung | Không hook thêm, không chặn chuột game; `souls_vision` làm đúng kiểu này (xem dưới) | Người chơi phải tự mở một menu (vd. Equipment) để có con trỏ; cú nhấp vẫn tới menu game bên dưới; minimap đang ẩn ở menu nên chế độ chỉnh phải vẽ khung dù HUD ẩn |

## Tham khảo: souls_vision (`tmp/repos/souls_vision`, C++, commit `6443cac`)

Mod này có kéo thả (hướng C), đã đọc `overlay.cpp`, `config.cpp`, `README.MD`:

- Khoá cấu hình `general.dragOverlay` (mặc định `false`, **không có phím**: sửa file rồi tải lại); bật thì cửa sổ ImGui "Stat Window" bỏ cờ `NoInputs`, `NoDecoration`, `NoBackground` (còn `NoTitleBar`, `NoResize`) nên ImGui tự lo việc kéo cửa sổ.
- **Không bắt chuột:** `WndProc` chỉ chuyển thông điệp cho `ImGui_ImplWin32_WndProcHandler` rồi gọi `WndProc` gốc, game vẫn nhận mọi cú nhấp. README (v0.2.0) khuyên "vào một menu (như Equipment) để hiện con trỏ cho dễ kéo (đang tìm cách tốt hơn)", tức tác giả chưa giải việc con trỏ.
- **Lưu:** mỗi khi vị trí cửa sổ đổi (mỗi frame lúc kéo) gọi `Config::SaveConfig`, ghi đĩa liên tục; nên chỉ lưu khi nhả chuột (như `shared/overlay` làm với `MenuX/MenuY`).
- Từng có lỗi cửa sổ trôi dần sang trái khi bật `dragOverlay` (sửa ở bản sau): coi chừng cộng và trừ độ lệch lặp mỗi frame khi đổi giữa toạ độ cửa sổ và độ lệch cấu hình.

Kết luận: hướng C chạy được với chi phí rất thấp nhưng chỉ là giải pháp tạm; nó xác nhận rằng con trỏ tự do là phần mà `souls_vision` chưa giải.

## Thiết kế đề xuất cho B

- **Vào chế độ chỉnh bằng phím** (như `ToggleKey`), không bật mặc định. **Cài hook lười**: chỉ cài `ClipCursor`/`SetCursorPos`/chuột game lần đầu bấm phím chỉnh, để người không dùng tính năng không phải chịu hook nào (mod này public, nhiều người dùng ERSS có overlay riêng).
- **Mô hình vị trí:**
  - Kéo cả cột: độ dời chuột (px) chia `res` cộng vào `Layout.OffsetX/OffsetY` (dấu của X ngược chiều vì tính từ mép phải), kẹp trong khung hình; X âm cho phép kéo ra vùng hai bên của màn ultrawide (đã hỗ trợ từ 1.1.0).
  - Kéo riêng thanh mục tiêu: thêm khoá cộng thêm (vd. `Target.OffsetX/OffsetY`, mặc định 0, tính từ chỗ mặc định dưới minimap) để cấu hình cũ không đổi; khi minimap ẩn, thanh vẫn giữ độ lệch tương đối khi trượt lên.
- **Giao diện chỉnh:** khung viền quanh từng phần, tên ("Minimap", "Target"), dòng gợi ý phím; nhấp đúp vào khung để về mặc định; gần mép màn hình thì hít vào mép.
- **Lưu:** khi nhả chuột gọi `config::save` như menu (giữ comment), và `settings::set_offset_*` để áp dụng ngay; đây là chỗ giống hệt đường slider.

## Rủi ro cần kiểm trước khi viết

1. **Hai DLL cùng hook một hàm.** Mỗi DLL link bản `common` và MinHook riêng; `shared/docs/overlay.md` mục 7 nói một hàm chỉ hook được một lần mỗi DLL, và mục 6 nói hai DLL `hudhook` cùng móc DX12 một lúc từng ghi đè nhau. `ClipCursor`/`SetCursorPos`/DirectInput bị hook bởi nhiều DLL (Toolbox, soulsteleport, và ERSS: `ERSS.log` có "Installing overlay input hooks", "Hooking WNDPROC"). Phải thử chạy chung với ERSS 5.1.0 và với Toolbox.
2. **AOB của `DLUID::MouseDevice::Poll`** phụ thuộc bản exe: theo quy tắc dự án phải kiểm duy nhất trên mọi bản (`scripts/ida/run.ps1 aob -Version all`); `shared` đã có, JourneyHUD dùng lại thì thừa hưởng.
3. **`LoadDelay`:** hook chuột cài muộn theo `LoadDelay`; vì cài lười lúc bấm phím nên không ảnh hưởng.
4. **Nhánh git:** phần trích "bắt chuột" thành API dùng chung (`overlay::capture_mouse(bool)` + đọc toạ độ tuyệt đối) đụng `shared/` và ảnh hưởng `soulsteleport`, `toolbox`, nên theo quy tắc nhiều mod làm trên nhánh `shared/overlay-mouse-capture` (từ `main`), JourneyHUD chỉ dùng API đó.

## Việc tiếp theo đề xuất

1. Giai đoạn 1 (hướng A) trên `mod/journeyhud`: phím chỉnh, vẽ khung, mũi tên, lưu; thêm `Target.OffsetX/OffsetY` nếu muốn kéo riêng thanh mục tiêu.
2. Thử nghiệm nhỏ cho giai đoạn 2: bật `shared::overlay::install_input_hooks()` trong một bản dựng thử của JourneyHUD cùng ERSS 5.1.0 để xem con trỏ có thả ra và camera có đứng yên không, trước khi trích API.
3. Giai đoạn 2 (hướng B) sau khi thử nghiệm đạt.
