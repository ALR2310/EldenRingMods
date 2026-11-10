<div align="center">

# DropMultiplier

**Chỉnh tỉ lệ rơi đồ của quái và số lượng nguyên liệu nhận được.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11075-orange)](https://www.nexusmods.com/eldenring/mods/11075)

</div>

---

## Giới thiệu

DropMultiplier chỉnh hai thứ độc lập: tần suất quái rơi đồ và số lượng nguyên liệu nhận được mỗi lần
nhặt, để bạn bớt thời gian cày. Tỉ lệ rơi có thể nhân hệ số hoặc đặt cố định, còn nguyên liệu chế
tạo, nâng cấp và nguyên liệu chỉ nhặt một lần mỗi nhóm có hệ số riêng. Mod sửa dữ liệu của game khi
đang chạy và không ghi đè `regulation.bin`, nên dùng được cả với các bản mod lớn như Convergence.

Tỉ lệ cố định có thể biến một món vốn chắc chắn rơi thành rơi theo xác suất.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11075).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `DropMultiplier.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `DropMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [DropMultiplier.toml](DropMultiplier.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Drop Multiplier** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `DropMultiplier.ini`. Nếu chưa có `DropMultiplier.toml`, cài đặt trong file ini được chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11075): tải về và bình luận.
- [docs/drop_rate.md](docs/drop_rate.md): mô hình bốc thăm của game, hai chế độ, công thức tỉ lệ cố định.
- [docs/materials.md](docs/materials.md): cách phân loại và nhân nguyên liệu.
- [shared/docs/params.md](../../shared/docs/params.md): đọc dòng param an toàn, và lỗi panic với Convergence.
