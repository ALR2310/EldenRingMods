<div align="center">

# AutoRegen

**Tự động hồi HP, FP và Stamina, theo thời gian hoặc khi đánh trúng.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10548-orange)](https://www.nexusmods.com/eldenring/mods/10548)

</div>

---

## Giới thiệu

AutoRegen hồi HP, FP và Stamina theo hai cách độc lập, có thể bật cùng lúc: theo thời gian (có thể
chỉ khi ngoài giao tranh, trong giao tranh, đứng yên hoặc đang ngồi) và khi đánh trúng. Mọi lượng
hồi đều chỉnh được.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/10548).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `AutoRegen.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `AutoRegen.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [AutoRegen.toml](AutoRegen.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Auto Regen** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `AutoRegen.ini`. Nếu chưa có `AutoRegen.toml`, cài đặt trong file ini được chuyển sang
ở lần chạy đầu; bản thân file ini được để nguyên. Key gõ sai hoặc sai kiểu được báo kèm số dòng, và cấu
hình đang chạy được giữ nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/10548): tải về và bình luận.
- [docs/per_hit.md](docs/per_hit.md): hook hồi khi đánh trúng, phân loại đòn, loại trừ weapon art, các lỗi đã gặp.
- [docs/gesture_trigger.md](docs/gesture_trigger.md): điều kiện đứng yên và gesture, các hướng đã thử.
