# AutoPickup

[English](README.md)

Tự nhặt đồ khi đứng trong tầm, không cần bấm nút.

## Tóm tắt

Trong Elden Ring, mỗi lần nhặt cây cỏ, đồ trên đất hay đồ trên xác đều phải đứng đúng chỗ và
bấm nút. AutoPickup bấm nút đó thay bạn. Cần gạt, cửa, grace và hội thoại không bị đụng tới.

Năm nhóm, mỗi nhóm bật tắt và chỉnh tầm độc lập:

| Nhóm | Gồm |
|---|---|
| **Materials** | cây cỏ, nấm, nguyên liệu chế tạo |
| **Items** | vật phẩm nằm trên đất |
| **CorpseLoot** | đồ trên xác và hài cốt |
| **LostRunes** | rune bạn làm rơi khi chết |
| **Chests** | rương kho báu (mặc định tắt, vì rương mimic mở cùng cách) |

Ngoài ra có thể tạm dừng tự nhặt khi đang chiến đấu, nới tầm nhặt (cũng là tầm hiện gợi ý
nhặt) cho từng nhóm, và tắt animation nhặt.

## Cài đặt

1. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
2. Đặt `AutoPickup.dll` vào thư mục mod của loader và khai báo nó.
3. Chạy game. `AutoPickup.toml` được tạo cạnh DLL ở lần chạy đầu.

## Cấu hình

Cấu hình nằm trong `AutoPickup.toml` cạnh DLL. Tên key, giá trị mặc định và ý nghĩa nằm ở
chú thích trong [AutoPickup.toml](AutoPickup.toml), file mẫu nhúng sẵn trong DLL. Sửa file rồi
bấm phím nạp lại (mặc định `F5`) để áp dụng, không cần khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Auto
Pickup** xuất hiện ở System > Game Options với đủ các tuỳ chọn trên. Không có ERNativeUI thì mod
vẫn chạy bình thường từ file TOML.

## Cách hoạt động

Mod hook hàm game dùng để hỏi "tương tác này có được kích hoạt không"
(`CSActionButtonManImp::ExecActionButton`). Với tương tác thuộc nhóm đang bật, hook trả lời
"có" nên game nhặt như thể bạn đã bấm; mọi tương tác khác chạy như cũ. Tầm nhặt là bán kính,
chiều cao và độ lệch chiều cao của dòng `ActionButtonParam`, nhân theo `Range` và luôn tính từ
giá trị gốc của game. Chi tiết: [docs/exec_action_button.md](docs/exec_action_button.md) và
[docs/pickup_animation.md](docs/pickup_animation.md).

## Tương thích và giới hạn

- Không chạy cùng Seamless Co-op.
- Hàm được tìm bằng mẫu byte. Bản game mới có thể làm hỏng việc này; khi đó mod ghi log lỗi và
  để game chạy tiếp mà không nhặt tự động.
- Không dùng cùng DLL khác cũng tự nhặt; tắt tính năng đó ở bên kia.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [docs/exec_action_button.md](docs/exec_action_button.md): cơ chế, điểm hook, rủi ro.
- [docs/pickup_animation.md](docs/pickup_animation.md): cách tắt animation nhặt.
