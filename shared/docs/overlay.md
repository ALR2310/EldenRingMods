# overlay: khung menu ImGui DX12 dùng chung

**Status 2026-10-31: trích từ `soulsteleport` (đã chạy ổn định trong game) lên `shared/src/overlay/`, bật qua feature `overlay` của `common`. Dùng ở `soulsteleport` và `toolbox`.**

## Mục đích

Mọi mod cần cửa sổ Dear ImGui vẽ đè lên DX12 của game (qua `hudhook`) đều cần cùng một lớp hạ tầng:

1. **Font**: nhúng `shared/assets/NotoSans-Subset.ttf` (~90 KB, đủ chữ Việt/Latin) và tự gộp thêm font CJK đầu tiên tìm thấy trong thư mục Fonts thật của Windows (`GetWindowsDirectoryW`).
2. **Co giãn theo độ phân giải + `MenuScale`**: chuẩn 1080p (`display_size[1] / 1080.0`, kẹp `0.6..=3.0`), nhân thêm `MenuScale` từ ini. Sau `scale_all_sizes`, đặt lại `mouse_cursor_scale = base * scale` không làm tròn để con trỏ không biến mất ở cửa sổ nhỏ hơn 1080p.
3. **Con trỏ chuột và chặn chuột vào game**:
   - Đọc `GetCursorPos` + `ScreenToClient` mỗi frame khi menu mở, quy đổi sang `display_size` (hỗ trợ cả Windows DPI scaling), đọc nút trái/phải qua `GetAsyncKeyState`.
   - Chặn `WM_INPUT` và `WM_MOUSE*` trong `before_wnd_proc` khi menu mở để sự kiện tương đối của hudhook không ghi đè tọa độ tuyệt đối.
   - Hook `SetCursorPos` và `ClipCursor` (MinHook) để game không kéo con trỏ về giữa màn hình khi menu mở.
   - Hook cả `DLUID::MouseDevice::Poll` của `eldenring.exe` (quét AOB, xoá `DIMOUSESTATE2` tại `+0x7E8` và 3 trục float tại `+0x800`) lẫn `IDirectInputDevice8::GetDeviceState` (slot 9) và `GetDeviceData` (slot 10) trên `System32\dinput8.dll` và `dinput8.dll` cục bộ trong `overlay/input_block.rs`: trả trạng thái chuột rỗng cho game khi menu mở (kể cả khi thư mục game có `dinput8.dll` proxy tự dỡ sau khi chuyển tiếp sang `System32`), vẫn giữ bàn phím và tay cầm để nhân vật di chuyển được.
4. **Theme & khung cửa sổ**: nền đen ấm hơi trong suốt, chữ kem, viền/điểm nhấn vàng đồng; thanh đầu có nút `Reset` (góc trái), tiêu đề căn giữa, nút `X` đóng (góc phải).
5. **Lưu vị trí & kích thước (`MenuX`, `MenuY`, `MenuWidth`, `MenuHeight`)**: tính theo đơn vị 1080p, chỉ ghi vào ini khi người chơi nhả chuột trái sau khi kéo/đổi cỡ. Hạng mục `save_ini_values` hỗ trợ lưu trực tiếp các thiết lập khác từ GUI vào ini và nạp lại `common::config`.
6. **Không tranh chấp hook DX12 với các DLL `hudhook` khác (vd. `JourneyHUD.dll`)**: khi hai DLL cùng gọi `ImguiDx12Hooks::new()` (`MH_CreateHook`) đồng thời trước khi một bên kịp `MH_ApplyQueued()`, cả hai cùng tạo trampoline từ prologue gốc chưa hook nên DLL gọi `MH_ApplyQueued()` sau sẽ ghi đè nhảy qua DLL trước. `overlay::install` đợi `CSTaskImp` + 800 ms (sau khi `JourneyHUD` đã hook xong ở mốc `CSTaskImp`, nhưng vẫn trước khi cửa sổ DX12 của game tạo ở ~+3 s) và giữ mutex toàn tiến trình `Local\EldenRingMod_OverlayInit_<pid>`.

Chi tiết quá trình tìm ra từng lỗi (mất con trỏ ở 1680x945, raw input, DirectInput): xem `mods/soulsteleport/docs/menu_ui.md`.
