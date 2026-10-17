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
