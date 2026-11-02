# Kim người chơi dễ nhìn hơn / tương thích Blue Markers

**Status 2026-11-02: đã xác nhận trong game với ModEngine 2 và me3 rằng JourneyHUD nhận asset ghi đè; UXM chưa thử.**

Yêu cầu: WalrusLife (Nexus, 2026-11-02): dùng cùng mod Blue Markers (Nexus 11179) để nhìn hướng nhân vật; kim mặc định hoà vào icon grace và nền bản đồ.

## Blue Markers là gì (xem `Downloads\Blue Markers 11179`)

Một file duy nhất: `menu/hi/01_common.tpf.dcx` (~67 MB nén), file thay thế cho sprite sheet menu chung của game (cùng file JourneyHUD đọc sprite, xem `map.md`). Đã giải nén và so từng sprite: chỉ sheet `SB_MapCursor` đổi, 63 sprite khác (kim `MENU_MAP_Player_01/02`, kẻ địch, bạn, host, memo, marker...), `SB_MapCursor_02` và `_03_dlc` giống hệt. Kim `Player_01` cùng khung và kích thước, chỉ đổi màu vàng sang xanh lơ. Không có `01_common.sblytbnd.dcx` đi kèm nên toạ độ sprite giữ nguyên.

## Vì sao JourneyHUD không thấy bản xanh

`src/map/sprites.rs` đọc `/menu/hi/01_common.tpf.dcx` qua `archive::open(game_dir, ...)`, tức chỉ trong archive gốc `Data0..3` / `DLC` của game. File rời mà ModEngine nạp từ thư mục mod không được xét, nên kim vẫn là kim vàng gốc. (Nếu người dùng đang chạy mod soarqin thì mod đó chắc đọc qua đường khác; chưa kiểm.)

## Cách ModEngine trả file ghi đè

Cả hai loader đều làm việc ở lúc chương trình mở file, nên DLL không cần tự tìm profile hay đoán thư mục mod:

- **ModEngine 2:** hook `CreateFileW`. Khi đường dẫn nằm dưới thư mục game, nó tìm lần lượt các `mods` trong `config_eldenring.toml` rồi mở file cùng đường dẫn trong thư mục mod ([mã nguồn](https://github.com/soulsmods/ModEngine2/blob/main/src/modengine/ext/mod_loader/archive_file_overrides.cpp#L53-L64), [hook](https://github.com/soulsmods/ModEngine2/blob/main/src/modengine/ext/mod_loader/archive_file_overrides.cpp#L136-L185)).
- **me3:** profile tách `[[packages]]` (asset) với `[[natives]]` (DLL) ([tài liệu](https://github.com/garyttierney/me3/blob/main/docs/configuration-reference.md#dissecting-the-example-configuration)). Bản 0.8.1 cài hook `CreateFileW` trước khi nạp native để file được tìm bên trong thư mục game có thể lấy từ package ([changelog](https://github.com/garyttierney/me3/blob/main/CHANGELOG.md#me3---v081---2025-09-15)).

Vì vậy không có API VFS công khai nào JourneyHUD cần gọi. Không nên tự parse `config_eldenring.toml` hay `.me3`: ME2 dùng biến môi trường `MODENGINE_CONFIG`, còn me3 truyền danh sách package đã biên dịch cho mod host và có thể dùng profile ở bất cứ đâu. Cả hai cách sẽ sai thứ tự ưu tiên hoặc bỏ sót cấu hình launcher.

## Khả thi

- **Nhận file ghi đè:** `sprites.rs::read_menu_file` thử `std::fs::read(<game_dir>/menu/hi/01_common.tpf.dcx)` trước, cho cả texture lẫn layout. Với UXM, đây là file thật; với ME2/me3, hook `CreateFileW` sẽ tự trả file đang thắng theo thứ tự package. Nếu file không có, hàm fallback sang `archive::open` như trước. Như vậy JourneyHUD dùng đúng asset mà game đang dùng, không phụ thuộc loader nào hay tên/thư mục mod.
- **Việc rẻ hơn, không phụ thuộc mod ngoài:** thêm tuỳ chọn cho kim người chơi (phóng to, tô màu hoặc viền sáng), vẽ tại `radar_ui.rs` (`Icon::Player`, vẽ ở dòng ~598-607). Đáp ứng đúng lời xin "làm marker dễ thấy hơn" và nên làm trước.

## Chưa kiểm

Người dùng đã thử Blue Player Map Marker trong game với ModEngine 2 và me3: kim ghi đè hiện đúng. UXM và trường hợp không có override chưa thử lại.
