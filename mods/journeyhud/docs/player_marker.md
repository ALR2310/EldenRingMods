# Kim người chơi dễ nhìn hơn / tương thích Blue Markers

**Status 2026-11-02: đã xác định JourneyHUD chỉ đọc sprite gốc; chưa thử nhận file ghi đè hoặc đổi cách vẽ kim.**

Yêu cầu: WalrusLife (Nexus, 2026-11-02): dùng cùng mod Blue Markers (Nexus 11179) để nhìn hướng nhân vật; kim mặc định hoà vào icon grace và nền bản đồ.

## Blue Markers là gì (xem `Downloads\Blue Markers 11179`)

Một file duy nhất: `menu/hi/01_common.tpf.dcx` (~67 MB nén), file thay thế cho sprite sheet menu chung của game (cùng file JourneyHUD đọc sprite, xem `map.md`). Đã giải nén và so từng sprite: chỉ sheet `SB_MapCursor` đổi, 63 sprite khác (kim `MENU_MAP_Player_01/02`, kẻ địch, bạn, host, memo, marker...), `SB_MapCursor_02` và `_03_dlc` giống hệt. Kim `Player_01` cùng khung và kích thước, chỉ đổi màu vàng sang xanh lơ. Không có `01_common.sblytbnd.dcx` đi kèm nên toạ độ sprite giữ nguyên.

## Vì sao JourneyHUD không thấy bản xanh

`src/map/sprites.rs` đọc `/menu/hi/01_common.tpf.dcx` qua `archive::open(game_dir, ...)`, tức chỉ trong archive gốc `Data0..3` / `DLC` của game. File rời mà ModEngine nạp từ thư mục mod không được xét, nên kim vẫn là kim vàng gốc. (Nếu người dùng đang chạy mod soarqin thì mod đó chắc đọc qua đường khác; chưa kiểm.)

## Khả thi

- **Nhận file ghi đè:** khả thi nhưng có chỗ khó. Phải biết thư mục mod đang bật (ME2 `modengine.toml`, ME3 `packages` của profile, UXM). Hướng nhẹ: thử `<game_dir>\menu\hi\01_common.tpf.dcx` (UXM), thêm tuỳ chọn thư mục ghi đè; bản rời ghi đè không qua RSA/Data nên đọc thẳng `std::fs`, vẫn giải DCX/TPF bằng code có sẵn. Rủi ro: file ghi đè của mod khác (Convergence, Reforged, UI) cũng thay cả sheet này, nên phải đọc đúng bản đang hiệu lực.
- **Việc rẻ hơn, không phụ thuộc mod ngoài:** thêm tuỳ chọn cho kim người chơi (phóng to, tô màu hoặc viền sáng), vẽ tại `radar_ui.rs` (`Icon::Player`, vẽ ở dòng ~598-607). Đáp ứng đúng lời xin "làm marker dễ thấy hơn" và nên làm trước.

## Chưa kiểm

Chưa thử nhận file ghi đè trong game, ME2/ME3. Tô màu kim bằng code (ImGui `col`) cho cùng hiệu ứng mà không cần file ngoài.
