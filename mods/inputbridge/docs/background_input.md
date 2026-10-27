# Input nền cho Elden Ring: các lớp chặn và cách gỡ

**Status 2026-10-27: bàn phím và chuột chèn được vào game đang ở nền (thử trên 2.7.2.0: từ màn hình tiêu đề vào world, đi bằng W, mở/đóng bản đồ bằng M, xoay camera, chuột trái vung kiếm; bàn phím/chuột thật của người dùng bị lọc khi game không ở trước). Chưa làm: tay cầm (chỉ lọc), kênh pipe/MCP, đọc trạng thái trực tiếp.**

## Kết luận

`PostMessage` (chế độ nền của `game-player` MCP) không bao giờ tới được game: nó không đọc input qua tin nhắn cửa sổ. `eldenring.exe` chỉ import `DINPUT8!DirectInput8Create` và `XINPUT1_4` (`XInputGetState`/`XInputSetState`). Để gửi phím lúc game ở nền phải gỡ **ba lớp** chặn, mỗi lớp thử riêng đều chưa đủ:

| Lớp | Biểu hiện | Cách gỡ (`src/`) |
|---|---|---|
| 1. DirectInput mất thiết bị khi mất focus | `GetDeviceState` bàn phím trả `0x8007001E` (`DIERR_INPUTLOST`) | `dinput.rs`: hook `GetDeviceState`/`GetDeviceData`, lỗi thì trả `DI_OK` với trạng thái rỗng rồi OR các phím giả |
| 2. Cờ "chỉ foreground" của game | Đọc phím thành công nhưng game bỏ qua | `gate.rs`: xoá 3 byte `+0x88E` (pad), `+0x88F` (bàn phím), `+0x890` (chuột) của singleton input |
| 3. So `GetForegroundWindow` với cửa sổ game | Vẫn bỏ qua khi hai lớp trên đã gỡ | `focus.rs`: hook `GetForegroundWindow` trả cửa sổ game; kèm `ClipCursor`/`SetCursorPos` để game không nhốt chuột người dùng |

Bằng chứng: tách từng lớp. Chỉ lớp 1 + 2: bàn phím đọc 0 lỗi ở nền nhưng E không ăn. Khi game được focus thật thì cùng lệnh E ăn ngay (chứng tỏ cách chèn đúng). Thêm lớp 3 thì E ăn ở nền.

## Cổng ở lớp 2 (IDA, 2.7.2.0)

Singleton ở global `qword_144864D48` (RVA `0x4864D48`; mod dò bằng AOB, không hard-code). Byte `+0x88D` = "cửa sổ active", `+0x88E/0x88F/0x890` = "SetForeground" cho pad/bàn phím/chuột, nạp lúc khởi tạo từ các cờ cấu hình `Ext.UserInput.CooperativeLevel.SetForeGround.{Pad,Keyboard,Mouse}` (hàm `0x1F2C7B0`). Hàm đọc tay cầm `0x1F6F030` bỏ qua khi `[+0x88E] && ![+0x88D]`. Hàm `0x1F70C20` là phần dựng thiết bị bàn phím (đọc `+0x88F`). AOB của gate: `scripts/ida/run.ps1 aob -Version all` cho `UNIQUE` ở 2.6.2.0, 2.7.0.0, 2.7.1.0, 2.7.2.0.

`GetForegroundWindow` có hai nơi gọi trong exe: `0x22AC530` (cấu hình âm thanh) và `0x2671EB0` (so với cửa sổ game, 4 hàm gọi: `0x9BEBB0`, `0x9BF350`, `0xE21B80`, `0xE3785F`, `0xE85F20`, `0xE93D80`).

## Lỗi đã gặp: không dùng `GetWindowTextW` trên cửa sổ game

Tìm cửa sổ game bằng `EnumWindows` + `GetWindowTextW` làm game crash lúc khởi động (`eldenring.exe+0x1E937F0`, `a1=0x20`): với cửa sổ cùng process `GetWindowTextW` gửi `WM_GETTEXT` thẳng vào window proc, lúc đó game chưa khởi tạo xong. Dùng `InternalGetWindowText` (đọc trực tiếp, không gửi tin nhắn).

## Giao thức thử

Ghi `InputBridge.cmd` (ghi tên tạm rồi đổi tên) cạnh DLL, mod đọc rồi xoá: `down <phím>`, `up <phím>`, `press <phím> [giây]`, `release_all`; chuột: `mouse <dx> <dy>`, `look <dx> <dy> <giây>` (dịch mỗi khung, `look 6 0 1` xoay camera khoảng 25-30 độ), `mdown`/`mup`/`mclick <left|right|middle> [giây]`, `wheel <n>`. Mã phím là DIK (`src/keys.rs`). Chuột chèn vào `DIMOUSESTATE`/`DIMOUSESTATE2` (lX, lY, lZ rồi từng byte nút) trong cùng hook `GetDeviceState`. Một số thao tác có hiệu ứng chậm vài giây (bản đồ), `look` ảnh sau 3-5 giây. Log `InputBridge.log` có dòng thống kê mỗi 5 giây: `fg` (foreground thật), `gate=active/pad/kb/mouse`, số lần đọc và lỗi của từng thiết bị.

## Chưa biết / việc tiếp

- Tay cầm (`XInputGetState`) mới chỉ bị lọc (xoá nút và cần gạt khi game ở nền), chưa chèn.
- Chưa thử chiến đấu thật (khoá mục tiêu, đỡ, lăn): cần biết phím lăn của người dùng.
- Đường đọc có đệm (`GetDeviceData`) game không dùng ở màn hình tiêu đề (0 lần gọi).
- Mod tự chứa một bản MinHook; không bật cùng `SoulsTeleport.dll` (cùng hook DirectInput).
- `PauseTheGame.dll` đã tắt khi thử; chưa kiểm xem nó có ảnh hưởng không.
