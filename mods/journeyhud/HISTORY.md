# JourneyHud: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng. Chuyện nào có phân tích dài thì
link sang `docs/`.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-26 | Tạo mod: panel máu và poise của kẻ địch đang khoá, vẽ bằng ImGui/hudhook. Mục tiêu đọc từ `locked_on_enemy` của người chơi thay cho cách ghép theo vị trí. Chưa test trong game. | [design.md](docs/design.md) |
| 2026-10-26 | Kiểm tĩnh trong IDA trên 2.6.2.0 đến 2.7.2.0: handle khoá +0x6B0 và slot 3 tra theo handle đúng ở cả 4 bản; thêm assert offset lúc build. +0x14 (poise max) chưa xác nhận được. | [offsets.md](docs/offsets.md) |
| 2026-10-26 | Thử game đầu tiên: panic vì lấy 2 draw list foreground cùng lúc (sửa). Màn 16:10 làm panel quá to và đè dải đen: neo theo khung 16:9 của game. Đổi màu và thanh mảnh hơn cho giống HUD gốc. | [design.md](docs/design.md) |
| 2026-10-26 | Vẽ lại thanh theo số đo từ ảnh game: lõi đỏ sậm phẳng, mép dưới ngả hồng, viền be mảnh có nắp hai đầu, nền gần đen, bỏ hộp nền và số (số bật bằng ShowNumbers). | [design.md](docs/design.md) |
| 2026-10-26 | Theo thanh boss của game: bỏ viền quanh thanh (chỉ còn đường be dưới đáy và nắp hai đầu), thêm khối vàng cho máu vừa mất (giữ rồi tụt dần), thanh poise dính sát dưới thanh máu. | [design.md](docs/design.md) |
| 2026-10-26 | Thanh vẽ bằng 4 texture HUD của game (Bar, BarBG, BarEdge, BarEdge2) nhúng trong DLL: thanh tô màu và cắt theo máu, vạch ngăn và vệt sáng ở đầu máu khi vừa trúng đòn. Tải texture lỗi thì rơi về thanh phẳng. | [design.md](docs/design.md) |
| 2026-10-26 | Thử game: thanh đẹp nhưng nhỏ. Đo thanh HP người chơi (khoảng 712x12 ở 1080p, mép trên cách đỉnh khung 47) rồi đặt cỡ và vị trí mặc định theo đó; thêm khoá chỉnh độ dài. | [design.md](docs/design.md) |
| 2026-10-26 | Độ dài thanh về 220 như trước (700 quá to); giữ độ dày 12 theo thanh người chơi. | [design.md](docs/design.md) |
| 2026-10-26 | Bỏ tên kẻ địch (không cần, tra ra `None` cho Tree Sentinel, souls_vision gốc cũng không có). Thêm menu ERNativeUI (bật tắt, số máu, độ dài, cỡ, lề), áp dụng ngay và ghi ini. Thanh máu và poise đã thử đúng trong game. | [names.md](docs/names.md) |
