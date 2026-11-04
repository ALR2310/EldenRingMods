<div align="center">

# InfiniteAilment

**Chỉnh thời lượng và sát thương của Poison, Scarlet Rot, Black Flame và Destined Death.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11087-orange)](https://www.nexusmods.com/eldenring/mods/11087)

</div>

---

## Giới thiệu

Poison và Scarlet Rot trong Elden Ring chỉ kéo dài 90 giây, còn Black Flame và Destined Death chỉ
cháy vài giây. InfiniteAilment cho bạn chỉnh thời lượng, kể cả vô hạn, và sát thương mỗi nhịp của
bốn hiệu ứng này khi chính **bạn** gây ra cho kẻ địch. Mod sửa dữ liệu của game khi đang chạy và
không ghi đè `regulation.bin`, nên dùng được cả với các bản mod lớn như Convergence.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11087).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `InfiniteAilment.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `InfiniteAilment.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key, kèm giá trị vanilla, nằm ở chú thích trong [InfiniteAilment.toml](InfiniteAilment.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Infinite
Ailment** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `InfiniteAilment.ini`. Nếu chưa có `InfiniteAilment.toml`, cài đặt trong file ini
được chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11087): tải về và bình luận.
- [docs/row_discovery.md](docs/row_discovery.md): cách tìm các dòng Poison và Scarlet Rot.
- [docs/black_flame.md](docs/black_flame.md): Black Flame và Destined Death.
