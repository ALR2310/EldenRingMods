# AutoRegen

[English](README.md)

Tự động hồi HP, FP và Stamina, theo thời gian hoặc khi đánh trúng.

## Tóm tắt

AutoRegen hồi HP, FP và Stamina theo hai cách độc lập, có thể bật cùng lúc: **theo thời gian** (mỗi chu
kỳ, theo điều kiện bạn chọn) và **khi đánh trúng**. Mọi con số đều chỉnh được, và đổi xong bấm phím nạp
lại là áp dụng ngay.

### Theo thời gian (`[PerTick]`)

Mỗi chu kỳ, mod hồi một lượng cho HP, FP và Stamina khi điều kiện được chọn thoả:

| Điều kiện | Hồi khi |
|---|---|
| **Luôn luôn** | mọi lúc |
| **Ngoài giao tranh** | không đánh và không bị đánh trong 15 giây gần nhất |
| **Trong giao tranh** | vừa đánh hoặc bị đánh trong 15 giây gần nhất |
| **Đứng yên** | không di chuyển, không giữ hành động nào trong 5 giây |
| **Đang làm gesture** | đang ngồi (hoặc gesture khác trong danh sách của bạn) |

Lượng hồi tính theo **điểm cố định**, **% tối đa** hoặc **% phần đã mất** (nhanh khi thấp, chậm dần khi
gần đầy). **Trần hồi phục** giữ mỗi tick không hồi quá một mức phần trăm cho trước.

### Khi đánh trúng (`[PerHit]`)

Mỗi đòn bạn đánh trúng hồi một lượng, tính theo **điểm cố định**, **% tối đa** hoặc **% sát thương gây
ra**. Bạn chọn loại đòn được tính (cận chiến, phép và tầm xa, hoặc cả hai) và có thể loại trừ weapon art.

## Cài đặt

1. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
2. Đặt `AutoRegen.dll` vào thư mục mod của loader và khai báo nó.
3. Chạy game. `AutoRegen.toml` được tạo cạnh DLL ở lần chạy đầu.

## Cấu hình

Cấu hình nằm trong `AutoRegen.toml` cạnh DLL, đặt tên theo DLL. Tên key, giá trị mặc định và ý nghĩa
nằm ở chú thích trong [AutoRegen.toml](AutoRegen.toml), file mẫu nhúng sẵn trong DLL. Bấm phím nạp lại
(mặc định `F5`) để áp dụng, không cần khởi động lại game.

Các bản trước dùng `AutoRegen.ini`. Nếu chưa có `AutoRegen.toml`, cài đặt trong file ini được chuyển sang
ở lần chạy đầu; bản thân file ini được để nguyên. Key gõ sai hoặc sai kiểu được báo kèm số dòng, và cấu
hình đang chạy được giữ nguyên.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Auto Regen** xuất
hiện trong System > Game Options, gồm trang **Per Tick**, trang **Per Hit** và công tắc ghi log. Lượng
hồi và chu kỳ là ô nhập số (gõ được số lẻ). Danh sách gesture chỉ sửa trong file. Không có ERNativeUI thì
mod chạy bình thường từ file.

## Cách hoạt động

- **Theo thời gian:** một task trên nhóm `FrameBegin` của game đọc và ghi HP, FP, Stamina qua
  `CSChrDataModule` của [fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs).
- **Khi đánh trúng:** một hook ở đầu hàm hit-resolution của game chỉ quan sát ai đánh ai và sát thương
  bao nhiêu; lượng hồi do mod tự ghi. Chi tiết: [docs/per_hit.md](docs/per_hit.md).
- **Gesture:** chỉ tính là đang ngồi khi animation thật sự đổi và giữ ổn định, nên gesture bị chặn giữa
  lúc cast phép không làm mod hồi nhầm. Chi tiết: [docs/gesture_trigger.md](docs/gesture_trigger.md).

## Tương thích và giới hạn

- "Đang giao tranh" chỉ là xấp xỉ: đứng yên né đòn boss không tính là giao tranh.
- Phép có động tác cận chiến (vd. Carian Greatsword) bị tính là đòn cận chiến.
- Loại trừ weapon art theo nút bấm gần nhất, kèm kiểm tra animation cho Ash dạng stance. Ash của vài vũ
  khí có bộ chiêu riêng vẫn có thể bị tính là đòn thường.
- Hook hit mới được xác minh trên một bản game (2.7.1.0). Nếu không tìm thấy thì chỉ tắt phần hồi khi
  đánh trúng, hồi theo thời gian vẫn chạy.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [docs/per_hit.md](docs/per_hit.md): hook hồi khi đánh trúng, phân loại đòn, loại trừ weapon art, các lỗi đã gặp.
- [docs/gesture_trigger.md](docs/gesture_trigger.md): điều kiện đứng yên và gesture, các hướng đã thử.
