<div align="center">

# RiseArcher

**Buff cho cung, nỏ, ballista, mũi tên và tên nỏ.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-5807-orange)](https://www.nexusmods.com/eldenring/mods/5807)

</div>

---

## Giới thiệu

RiseArcher làm cho vũ khí tầm xa đáng dùng hơn: tăng sát thương và scaling, mở khoá Ash of War cho
mọi cung, giảm trọng lượng, tăng giá bán, mang được nhiều đạn hơn, và Rain of Arrows nhanh hơn, xa
hơn, nhiều mũi tên hơn. Mọi hệ số đều chỉnh được. Mod sửa dữ liệu của game khi đang chạy và không
ghi đè `regulation.bin`, nên không xung đột với các mod khác làm việc đó.

`RiseArcher.MASSEDIT` là script Mass Edit của bản phát hành gốc, để sửa thẳng `regulation.bin` trong
một param editor thay cho dùng DLL.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/5807).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `RiseArcher.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `RiseArcher.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [RiseArcher.toml](RiseArcher.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Rise Archer**
cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `RiseArcher.ini`. Nếu chưa có `RiseArcher.toml`, cài đặt trong file ini được
chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/5807): tải về và bình luận.
- [docs/param_patching.md](docs/param_patching.md): những dòng nào được patch, cách giữ giá trị gốc, lúc khởi động.
