# AnimCancel: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-12 | Nghiên cứu tĩnh: cancel do TAE bật qua `CSChrActionRequestModule`, hướng đi là nới `possible_action_inputs/cancels`; chưa biết HKS có chuyển state | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Tạo crate `animcancel` (probe): log cửa sổ cancel theo anim và phím F7 ép các hành động chọn được; chưa test trong game | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Test 1-3: ép bitmask không đủ (HKS đọc queue theo id TAE); ghi vào `input_entries` làm lệnh giả | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Ép vào `cancel_entries[id]` (tạo entry nếu thiếu), gỡ `disabled_action_inputs`, bit 37 cho đổi vũ khí: guard, lăn, đổi vũ khí huỷ được đòn đánh, đã test trong game | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Từ probe thành mod thật: gỡ log `WINDOW`/`FLAG` và phím F7, cấu hình `AnimCancel.toml` (`[Cancel]`, `[Animations]`, hot reload), chỉ ép khi anim thuộc nhóm bật và hàng đợi đã theo anim; chưa test trong game | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Tách `Group`/`group_of`/`current_anim_id` từ SpeedMultiplier sang `common::anim` để dùng chung | |
| 2026-10-12 | Thêm lại probe làm tuỳ chọn (`CancelProbe`, `FlagProbe` trong `[Logging]`, mặc định tắt) để chẩn đoán phép | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Phép không huỷ được: dùng id hàng đợi thay vì anim đang đọc (lúc niệm phép khi đi, `anim_queue` hiện lớp chân `a000_020010`) và coi mọi anim prefix 400-599 là Cast (tia Placidusax's Ruin bị `group_of` xếp vào Other); đã test trong game | [docs](docs/animation_cancel.md) |
| 2026-10-12 | Weapon skill và phép test ổn, `Skill` và `Cast` bật mặc định | |
| 2026-10-12 | Thêm `nexus_page.bbcode` và `CHANGELOG.md` (chuẩn bị đăng, chưa đăng) | |
| 2026-10-12 | Thêm `Item` (huỷ khi uống flask và dùng item, mặc định tắt) và ảnh minh hoạ trên trang Nexus; có mod ID 11328 | |
| 2026-10-13 | Thêm menu trong game tùy chọn (ERNativeUI, `common::erui`): 7 công tắc chia 2 trang (Cancel With / Cancel What) + Log file, ghi về TOML qua `TomlConfig::save_values`; probe vẫn chỉ ở file | [docs](../../docs/ingame_menu.md) |
| 2026-10-15 | Mỗi hành động (Guard/Dodge/WeaponSwap) có section và bộ animation riêng thay cho `[Cancel]` + `[Animations]` dùng chung, vì người chơi Reforged bị guard huỷ luôn phép khi cast bằng weapon catalyst (giữ guard + đánh nhẹ); `ConfigVersion` 2 kèm migrate file cũ, menu thành 3 trang con | [docs](docs/animation_cancel.md) |
| 2026-10-15 | Thử đỡ đòn trên không (nhóm `Jump` + `EventProbe`), bị kéo xuống đất và reset trạng thái liên tục nên dừng: không làm được chỉ bằng DLL; code nằm trong git stash | [docs](docs/air_guard.md) |
| 2026-10-15 | Chữ của menu trong game đưa sang hệ đa ngôn ngữ (en.json, vi.json trong `locales/`), có bản tiếng Việt | [docs](../../shared/docs/i18n.md) |
| 2026-10-15 | Người dùng đã chơi thử nhiều ngày, không thấy tác dụng phụ (i-frame, hitbox, FP khi cắt phép); đóng việc kiểm tra | [docs](docs/animation_cancel.md) |
| 2026-10-17 | Guard (L1) chỉ huỷ khi L1 thật sự là đỡ: cầm hai tay một vũ khí (trừ Twin Blades) hoặc một tay với khiên ở tay trái (`wepmotionCategory` 47/48/49/57); một vũ khí mỗi tay (dual wield) và Twin Blades thì L1 là đòn đánh, huỷ làm cụt combo (báo cáo người dùng, log `STYLE`); thêm dòng probe `STYLE` | [docs](docs/animation_cancel.md) |
| 2026-10-17 | Sửa lỗi người dùng báo (wlpo1266): giữ đỡ đòn rồi bấm tấn công thì đòn bị cắt ở 0,2 giây (guard còn giữ từ trước khi đòn bắt đầu bị coi là yêu cầu guard); Guard chỉ huỷ khi bấm guard mới sau khi đòn đã bắt đầu | [docs](docs/animation_cancel.md) |
| 2026-10-19 | Cấu hình đổi tên `AnimCancel.toml` thành `AnimCancelV1.toml`: chuỗi đường dẫn `AnimCancel.toml` trong DLL làm họ Doina (BitDefender) gắn cờ, đổi tên thì 14 xuống 6 cờ; người dùng bản cũ mất cấu hình (mod tạo file mới) | [docs](docs/antivirus.md) |
| 2026-10-19 | Dựng lại logic cắt animation (`cancel.rs`, `weapon.rs`, `probe.rs`): ghi bitfield qua kiểu `ChrActions` / `TaeCancelFlags` (crate `bitfield`) thay cho ép kiểu con trỏ và `mem::zeroed`, tách `plan()` (thuần, có test) khỏi `open()`, `GuardTracker`; hành vi giữ nguyên, đã test trong game | |
| 2026-10-19 | Sửa lỗi phím reload lọt ra ngoài game: đọc bằng `common::input` (`GetAsyncKeyState`, chỉ khi cửa sổ game có focus) thay cho `GetKeyState` của `eldenring`; nhóm 4 hãng antivirus (Avast, AVG, Avira, WithSecure) cũng không còn gắn cờ | [docs](../../shared/docs/reload.md) |
| 2026-10-19 | Thêm hành động Jump (`[Jump]`, trang menu): nút nhảy là bit 6 (`ACTION_ARM_CHANGE_STYLE`), mở trong `cancel_entries` là đủ; đã test trong game | [docs](docs/animation_cancel.md) |
| 2026-10-22 | Log gọn lại theo kiểu ngắn, kỹ thuật cho dev: bỏ dòng lặp ý, địa chỉ patch, câu dài; chỉ ghi 1 lần khi chờ thế giới | |
