# memscan: quét AOB và cache địa chỉ

**Status 2026-10-13: hoạt động, mọi mod hook bằng AOB đều dùng. Code ở `shared/src/memscan.rs`, có test cho việc parse pattern, tìm khớp và cache.**

## Quét AOB

`find_pattern_in_module` quét **chỉ các section thực thi** của module chính (exe
game) và trả về địa chỉ tuyệt đối của khớp đầu tiên. Pattern là chuỗi hex theo
kiểu IDA, `?` hoặc `??` là byte đại diện. Dùng cho các call site mà
`fromsoftware-rs` không có API phản chiếu (vd. `hit_hook` của autoregen, tìm
hàm hit-resolution bằng prologue của nó chứ không từ struct engine).

Token không phải `?`, `??` hay hex bị coi là byte `00`: một lỗi gõ trong pattern
cho ra "không tìm thấy" chứ không panic trong game.

Đọc `SizeOfOptionalHeader` từ header để tìm bảng section, không giả định
`0xF0`.

## `CachedAddr`

Địa chỉ code của exe không đổi sau khi đã tìm thấy (ASLR chỉ chọn base module
một lần lúc process khởi động), nên quét lại cùng pattern (mỗi lần đăng ký task,
mỗi lần hiện banner reload...) chỉ tốn công.

- **Chỉ cache lần resolve thành công.** Một lần trượt (vd. đua với Arxan đang giải
  nén lúc khởi động) để cache trống, người gọi sau quét lại thay vì bị khoá vào
  thất bại.
- **`Mutex` thay vì `OnceLock`:** khi nhiều thread hỏi cùng lúc (vd. thread
  `common::reload` và thread tính năng của mod cùng đăng ký task lúc khởi động),
  những thread đến sau chờ thread đầu xong rồi dùng kết quả của nó, thay vì tất cả
  cùng quét song song.
- `resolve` chỉ chạy khi chưa có gì trong cache, nên log "found" đặt bên trong nó
  chỉ xuất hiện một dòng mỗi process.

## `wait_for_pattern_in_module`

Thử lại mỗi `retry_interval` đến khi khớp hoặc hết `timeout`. Dành cho quét một
lần lúc khởi động có thể đua với cơ chế chống can thiệp của game (Arxan) trên máy
chậm hoặc đang tải, khi người gọi không có đường thử lại nào khác (hook chỉ cài
một lần, không thử lại mỗi tick). Hạn chót tính bằng đồng hồ thật, nên thời gian
mỗi lần quét cũng được tính vào `timeout`.

## `find_data_pointers` và `find_jumps_to`

Hai phép quét khác với AOB, cho hook mà mẫu byte không dùng được.

- **`find_data_pointers(value)`** tìm mọi ô 8 byte trong các section **không thực
  thi** của exe chứa `value`: chỗ một địa chỉ hàm nằm trong vtable hoặc bảng gọi.
  Chỉ đọc vùng đọc được: section có trang guard / no-access (bảo vệ của game), chạm
  vào là crash (log 2026-10-23, `access violation` ngay lúc quét), nên đi theo từng
  vùng `VirtualQuery` thay vì theo kích thước khai báo của section. `gfx_hook` dùng
  nó để đổi slot của hàm mở tệp.
- **`find_jumps_to(target)`** tìm mọi `jmp rel32` trong các section thực thi rơi
  đúng vào `target`: lệnh nhảy cuối của hàm bọc nhỏ gọi vào một hàm. Dùng khi hàm
  cần hook chỉ được gọi qua một wrapper không có mẫu byte duy nhất; người gọi tự lọc
  thêm bằng byte đứng trước lệnh nhảy.

