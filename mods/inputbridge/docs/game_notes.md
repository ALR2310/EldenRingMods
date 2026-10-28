# Ghi chú cho agent khi điều khiển Elden Ring

**Status 2026-10-27: sổ ghi chú sống. Agent đọc bằng tool `notes()` của MCP `er-input` trước khi chơi, và ghi thêm bằng `add_note(topic, note, source)` khi người dùng sửa một điều sai hoặc khi thử nghiệm chứng minh được điều mới. Mỗi dòng có ngày và nguồn: `user` (người dùng nói) hoặc `observed` (agent tự thấy). Điều đã bị bác thì sửa hoặc xoá thẳng trong file, không để lại dòng sai.**

## Phím

- 2026-10-27 [user] W/A/S/D: di chuyển.
- 2026-10-27 [user] Space: **nhảy** (không phải lăn).
- 2026-10-27 [user] Shift: **lăn** (giữ để chạy nhanh).
- 2026-10-27 [user] M: mở và đóng bản đồ.
- 2026-10-27 [observed] E: tương tác và xác nhận (đóng hộp thoại, chọn Continue).
- 2026-10-27 [observed] Chuột trái: tấn công.
- 2026-10-27 [observed] Bảng phím của bridge ở `mods/inputbridge/src/keys.txt` (có `-`, `=`/`plus`, phím số, ngoặc, Home/End...). Từ 2026-10-27 bridge cũng hook `GetAsyncKeyState`, nên phím giả tới được cả mod đọc phím kiểu Win32 (phím zoom JourneyHud đã thử: `-` đổi tầm minimap). MCP phải khởi động lại (/mcp) mới nhận danh sách phím và tool mới.

## Quy trình

- 2026-10-27 [observed] Từ màn hình tiêu đề vào world: `press e` (phím bất kỳ, ra hộp thoại cảnh báo), `press e` (đóng hộp thoại, ra menu chính với Continue), `press e` (Continue).
- 2026-10-27 [user] Tải nhân vật vào world mất khoảng 5 đến 10 giây sau khi chọn Continue.
- 2026-10-27 [observed] Game phải do người dùng bật offline (không Easy Anti-Cheat), và `InputBridge.dll` phải có trong `external_dlls` của `config_eldenring.toml`. 2026-10-27 [user, observed] Agent tự bật được game: PowerShell `Start-Process "E:\SteamLibrary\steamapps\common\ELDEN RING\Game\modengine2_launcher.exe" -ArgumentList "-t","er","-c",".\config_eldenring.toml" -WorkingDirectory <thư mục Game>` (đợi khoảng 25 giây cho cửa sổ), rồi vào world như trên (khoảng 25 giây sau Continue). Bash `cmd start` không chạy được.
- 2026-10-28 [observed] Tiến tới grace bằng WASD rất dễ lệch (di chuyển theo camera, 0.3 s khoảng 1 bước): chụp ảnh sau mỗi 0.3-0.5 s, grace phải ở ngay dưới chân nhân vật rồi mới bấm E. Mỗi screenshot trả cả ảnh lẫn base64 trong văn bản, nên dùng max_width nhỏ (320) để tiết kiệm. Bấm E gần grace lần đầu (2026-10-28) chưa mở được menu nghỉ, chưa rõ vì sao.

## Thời gian

- 2026-10-27 [observed] Bản đồ và menu hiện sau khoảng 3 đến 5 giây; chụp ngay sau lệnh thì chưa thấy gì.

## Camera và chuột

- 2026-10-27 [observed] `camera(6, 0, 1)` xoay khoảng 25 đến 30 độ sang phải.
- 2026-10-27 [observed] mouse_move chỉ nhận dịch chuyển tương đối (dx, dy). Để trỏ vào một điểm dùng `mouse_to(x, y)` / `click_at(x, y)` (toạ độ theo ảnh chụp gần nhất; cần MCP đã khởi động lại), trả con trỏ bằng `cursor_off()`. wheel(notches=-5) thu nhỏ bản đồ. Ở bản đồ, F mở danh sách grace, Z/X trong danh sách đổi vùng (chỉ thấy các vùng Lands Between, không có tab DLC ở save này).

## Màn hình

- 2026-10-27 [observed] Kích thước cửa sổ game thay đổi (320x168, 800x450); `screenshot(max_width=...)` thu nhỏ khung. Cửa sổ nhỏ thì chữ trong menu rất khó đọc.
