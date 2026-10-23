# gfx, gfx_hook: sửa movie Scaleform trong bộ nhớ

**Status 2026-10-23: hoạt động trong game 2.7.2.0 (đã thử với `HubHpBars` của spiritmultiplier); `gfx` có test, kể cả tệp thật qua test `#[ignore]`. Code ở `shared/src/gfx.rs`, `shared/src/gfx_hook.rs`.**

## Vì sao sửa trong bộ nhớ

Menu của game là movie Scaleform (`menu/*.gfx`, dạng SWF không nén). Đính kèm
tệp `.gfx` đã sửa thì đụng bản dịch và mod giao diện khác. ERNativeUI (nguồn
`tmp/ERNativeUI`, `docs/guides/presentation-and-gfx.md`) sửa movie ngay khi game
mở tệp, trước khi Scaleform đọc; `gfx_hook` làm cùng việc cho một tệp.

## `gfx`: bộ chỉnh

`extend_numbered_placements(movie, path, prefix, count)` đi theo tên instance từ
sân khấu xuống (`["Buddy", "Anim", "GaugeList"]`) tới một sprite rồi thêm
`prefix{n}` cho tới `count` bằng cách sao `PlaceObject2` của mục đánh số cuối:
cùng nhân vật và scale, vị trí lùi tiếp theo hiệu của hai mục cuối, depth
`+4`. Thêm trước `ShowFrame` đầu để có ở frame 1. Cập nhật độ dài `DefineSprite`
(đổi sang dạng tag dài khi cần) và độ dài khai báo của tệp. Trả `Ok(None)` khi đã
đủ, lỗi có tên (`NotGfx`, `PathNotFound`, `NoTemplate`, ...) thay vì panic.

Chỉ đọc/ghi dạng placement game dùng cho các hàng này (cờ `0x26`: tên, ma trận,
nhân vật); dạng khác trả `Unsupported`. Ma trận ghi lại với độ rộng bit tối thiểu.
Test: dựng movie tổng hợp từ đúng byte của `GaugeList` thật, kiểm vòng đọc/ghi
byte đúng, khoảng cách, idempotent; test `real_hud_movie` (`#[ignore]`, cần
`ER_HUD_GFX` và `ER_HUD_GFX_OUT`) chạy trên `01_000_fe.gfx` thật.

## `gfx_hook`: thay bộ đệm khi game mở tệp

- Hàm mở tệp của Scaleform (`sub_140D6E970` trên 2.7.2.0) chỉ được gọi qua một slot con trỏ trong dữ liệu exe, nên
  **đổi slot** (`memscan::find_data_pointers`) thay vì hook đầu hàm. ERNativeUI
  hook đầu hàm bằng SafetyHook; hai bên chồng nhau vẫn chạy vì ta gọi qua địa
  chỉ hàm, kể cả khi đầu hàm đã bị họ hook.
- AOB bắt đầu từ byte thứ 17 của hàm (duy nhất trên 2.6.2 - 2.7.2), **không** từ
  đầu hàm: jump của SafetyHook (ERNativeUI nạp trước) ghi đè tới 14 byte đầu nên
  mẫu tính từ prologue không khớp (log 2026-10-23: `file opener pattern not found`).
- Detour gọi hàm gốc, nếu đường dẫn kết thúc bằng tên tệp đích thì kiểm tra
  đối tượng file trong bộ nhớ (`+0x08` refcount > 0, `+0x18` con trỏ byte,
  `+0x20` độ dài, `+0x24` con trỏ đọc = 0, `+0x28` hợp lệ = 1; đọc được, kích
  thước hợp lý), tạo bản sửa **một lần** và trỏ `+0x18`/`+0x20` sang đó. Bản sửa
  không bao giờ giải phóng (Scaleform có thể giữ con trỏ theo vòng đời movie).
- Không kiểm vtable của đối tượng (ERNativeUI kiểm bằng RVA cố định theo bản);
  thay vào đó kiểm các trường trên. Nếu thấy đối tượng lạ, log và bỏ qua.
- Nếu game đã mở tệp trước khi mod cài hook (movie đã nạp sẵn), không có gì
  thay đổi; log `Movie: ... patched` không xuất hiện.
