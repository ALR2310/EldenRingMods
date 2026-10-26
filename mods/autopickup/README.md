<div align="center">

# AutoPickup

**Tự nhặt đồ khi đứng trong tầm, không cần bấm nút.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

Trong Elden Ring, mỗi lần nhặt cây cỏ, đồ trên đất hay đồ trên xác đều phải đứng
đúng chỗ và bấm nút. AutoPickup làm bước bấm đó thay bạn: đứng trong tầm là nhặt.
Cần gạt, cửa, grace và hội thoại không bị đụng tới.

## Tính năng

Năm nhóm, mỗi nhóm bật tắt và chỉnh tầm độc lập:

| Nhóm | Gồm |
|---|---|
| **Materials** | cây cỏ, nấm, nguyên liệu chế tạo |
| **Items** | vật phẩm nằm trên đất |
| **CorpseLoot** | đồ trên xác và hài cốt |
| **LostRunes** | rune bạn làm rơi khi chết |
| **Chests** | rương kho báu (mặc định tắt, vì rương mimic mở cùng cách) |

Ngoài ra có thể tạm dừng tự nhặt khi đang chiến đấu, và nới tầm nhặt (cũng là tầm
hiện gợi ý nhặt) cho từng nhóm.

## Cấu hình

Cấu hình nằm trong `AutoPickup.toml` cạnh DLL, tự tạo ở lần chạy đầu. Tên key,
giá trị mặc định và ý nghĩa nằm ở chú thích trong [AutoPickup.toml](AutoPickup.toml),
file mẫu nhúng sẵn trong DLL. Sửa file rồi bấm phím reload để áp dụng, không cần
khởi động lại game.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Auto Pickup** xuất hiện ở System > Game Options với đủ các tuỳ chọn trên. Không
có ERNativeUI thì mod vẫn chạy bình thường từ file toml.

## Cách hoạt động

Mod hook hàm game dùng để hỏi "tương tác này có được kích hoạt không"
(`CSActionButtonManImp::ExecActionButton`). Với tương tác thuộc nhóm đang bật, hook
trả lời "có" nên game nhặt như thể bạn đã bấm; mọi tương tác khác chạy như cũ. Tầm
nhặt là bán kính, chiều cao và độ lệch của dòng ActionButtonParam, nhân theo `Range`
và luôn tính từ giá trị gốc của game. Chi tiết: [docs/exec_action_button.md](docs/exec_action_button.md).

## Giới hạn

- Không chạy cùng Seamless Co-op.
- Dùng mẫu byte để tìm hàm; bản game mới có thể cần cập nhật mod (mod ghi log lỗi).
- Không dùng chung với DLL khác cũng tự nhặt (Solid Uncapper, ErdTools của Convergence): tắt tính năng đó ở bên kia.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [HISTORY.md](HISTORY.md): dòng thời gian phát triển.
- [docs/exec_action_button.md](docs/exec_action_button.md): cơ chế, điểm hook, nguồn tham khảo, rủi ro.
