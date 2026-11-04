# Ghi chú cho agent khi điều khiển Elden Ring

**Status 2026-10-27: sổ ghi chú sống. Agent đọc bằng tool `notes()` của MCP `er-input` trước khi chơi, và ghi thêm bằng `add_note(topic, note, source)` khi người dùng sửa một điều sai hoặc khi thử nghiệm chứng minh được điều mới. Mỗi dòng có ngày và nguồn: `user` (người dùng nói) hoặc `observed` (agent tự thấy). Điều đã bị bác thì sửa hoặc xoá thẳng trong file, không để lại dòng sai.**

## Phím

- 2026-10-27 [user] W/A/S/D: di chuyển.
- 2026-10-27 [user] Space: **nhảy** (không phải lăn).
- 2026-10-27 [user] Shift: **lăn** (giữ để chạy nhanh).
- 2026-10-27 [user] M: mở và đóng bản đồ.
- 2026-10-27 [observed] E: tương tác và xác nhận (đóng hộp thoại, chọn Continue).
- 2026-10-27 [observed] Chuột trái: tấn công.
- 2026-10-27 [observed] Bảng phím của bridge ở `mods/toolbox/src/input/keys.txt` (có `-`, `=`/`plus`, phím số, ngoặc, Home/End...). Từ 2026-10-27 bridge cũng hook `GetAsyncKeyState`, nên phím giả tới được cả mod đọc phím kiểu Win32 (phím zoom JourneyHud đã thử: `-` đổi tầm minimap). MCP phải khởi động lại (/mcp) mới nhận danh sách phím và tool mới.
- 2026-11-03 [observed] Phím F1..F12 và mọi hotkey mod đọc bằng GetAsyncKeyState (ReloadKey F5, DragKey F7...) chỉ tới mod khi Toolbox.dll đủ mới: log Toolbox.log phải có dòng "Win32 input: hooks applied (keys true, cursor true)". Bản Toolbox.dll build trước 2026-11-03 12:12 không có dòng này nên press f6/f7 "ok" nhưng mod không thấy; build lại Toolbox (scripts/build-mod.ps1 -Mod Toolbox) và chép vào thư mục mod. Thử 2026-11-03: sau khi chép bản mới, F7 và F5 đều chạy, kéo thả JourneyHUD bằng mouse_to + mouse_hold + mouse_to + mouse_release chạy được.

## Quy trình

- 2026-10-27 [observed] Từ màn hình tiêu đề vào world: `press e` (phím bất kỳ, ra hộp thoại cảnh báo), `press e` (đóng hộp thoại, ra menu chính với Continue), `press e` (Continue).
- 2026-10-27 [user] Tải nhân vật vào world mất khoảng 5 đến 10 giây sau khi chọn Continue.
- 2026-10-27 [observed] Game phải do người dùng bật offline (không Easy Anti-Cheat), và `Toolbox.dll` phải có trong `external_dlls` của `config_eldenring.toml`. 2026-10-27 [user, observed] Agent tự bật được game: PowerShell `Start-Process "E:\SteamLibrary\steamapps\common\ELDEN RING\Game\modengine2_launcher.exe" -ArgumentList "-t","er","-c",".\config_eldenring.toml" -WorkingDirectory <thư mục Game>` (đợi khoảng 25 giây cho cửa sổ), rồi vào world như trên (khoảng 25 giây sau Continue). Bash `cmd start` không chạy được.
- 2026-10-28 [observed] Tiến tới grace bằng WASD rất dễ lệch (di chuyển theo camera, 0.3 s khoảng 1 bước): chụp ảnh sau mỗi 0.3-0.5 s, grace phải ở ngay dưới chân nhân vật rồi mới bấm E. Mỗi screenshot trả cả ảnh lẫn base64 trong văn bản, nên dùng max_width nhỏ (320) để tiết kiệm. Bấm E gần grace lần đầu (2026-10-28) chưa mở được menu nghỉ, chưa rõ vì sao.

## Thời gian

- 2026-10-27 [observed] Bản đồ và menu hiện sau khoảng 3 đến 5 giây; chụp ngay sau lệnh thì chưa thấy gì.

## Camera và chuột

- 2026-10-27 [observed] `camera(6, 0, 1)` xoay khoảng 25 đến 30 độ sang phải.
- 2026-10-27 [observed] mouse_move chỉ nhận dịch chuyển tương đối (dx, dy). Để trỏ vào một điểm dùng `mouse_to(x, y)` / `click_at(x, y)` (toạ độ theo ảnh chụp gần nhất; cần MCP đã khởi động lại), trả con trỏ bằng `cursor_off()`. wheel(notches=-5) thu nhỏ bản đồ. Ở bản đồ, F mở danh sách grace, Z/X trong danh sách đổi vùng (chỉ thấy các vùng Lands Between, không có tab DLC ở save này).

## Màn hình

- 2026-10-27 [observed] Kích thước cửa sổ game thay đổi (320x168, 800x450); `screenshot(max_width=...)` thu nhỏ khung. Cửa sổ nhỏ thì chữ trong menu rất khó đọc.
- 2026-11-02 [observed] 2026-11-02: MCP chọn cửa sổ theo tiêu đề có chữ "Elden Ring": một tab Chrome tên "Journey HUD at Elden Ring Nexus" bị chụp thay cho game và phím E có thể đã gửi nhầm vào Chrome. Trước khi bấm phím, kiểm tra dòng "Window:" của screenshot là "ELDEN RING™"; nếu là Chrome thì dừng, báo người dùng đóng hoặc đổi tên tab.

## Bản đồ

- 2026-11-03 [user] Đổi bản đồ Lands Between sang Shadow Realm (DLC): mở bản đồ bằng M, đang ở bản đồ thì giữ X khoảng 0.6 giây (press x 0.6); bấm 0.2 giây không ăn. Trong danh sách grace (F) thì Z/X chỉ đổi vùng của danh sách, không đổi bản đồ: phải bấm Q để thoát danh sách về bản đồ rồi mới giữ X. Footer của bản đồ có dòng "X Show Shadow Realm map" / "Show Lands Between map". Danh sách grace chỉ có vùng của bản đồ đang hiển thị.
- 2026-11-04 [observed] 2026-11-04 [observed] Dịch chuyển (fast travel) qua MCP: M mở bản đồ, F mở danh sách grace, di chuyển hàng bằng phím mũi tên up/down (rê chuột không đổi hàng được chọn, chỉ click_at không chọn hàng), E chọn, hộp thoại "Is that okay?" bấm E lần nữa để OK, đợi khoảng 12 giây tải. Danh sách mở lại thì con trỏ không nhớ hàng cũ nên chụp ảnh kiểm hàng sáng trước khi bấm E.
