<div align="center">

# SpeedMultiplier

**Tăng hoặc giảm tốc độ của người chơi, Torrent, spirit và kẻ địch, riêng cho từng loại hành động.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11173-orange)](https://www.nexusmods.com/eldenring/mods/11173)

</div>

---

## Giới thiệu

SpeedMultiplier đổi tốc độ animation của nhân vật theo hệ số bạn chọn, riêng cho từng loại hành
động: chạy nhanh hơn mà không đánh nhanh hơn, hay niệm phép nhanh hơn mà không lăn nhanh hơn. Có thể
đặt tốc độ khác khi đang có một hiệu ứng hoặc ở một mức tải trọng.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11173).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `SpeedMultiplier.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `SpeedMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [SpeedMultiplier.toml](SpeedMultiplier.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Speed Multiplier** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Khi nâng cấp mod, key mới được thêm vào file của bạn mà không đụng tới giá trị đã chỉnh; key không còn
dùng trở thành comment ở cuối file. Key gõ sai hoặc sai kiểu được báo kèm số dòng, và cấu hình đang
chạy được giữ nguyên thay vì bị thay bằng file lỗi.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11173): tải về và bình luận.
- [docs/animation_speed.md](docs/animation_speed.md): trường tốc độ, các trường đã thử, chỗ đọc bị hook.
- [docs/action_groups.md](docs/action_groups.md): animation nào thuộc nhóm nào.
- [docs/tae_ids.md](docs/tae_ids.md): các tiền tố animation `aNNN` theo vũ khí, phép và kỹ năng vũ khí.
- [docs/override_conditions.md](docs/override_conditions.md): `[[Override]]` theo SpEffect hoặc mức tải.
- [docs/enemy_speed.md](docs/enemy_speed.md): cách phân biệt kẻ địch và boss.
- [docs/critical_victim.md](docs/critical_victim.md): nạn nhân của đòn chí mạng.
