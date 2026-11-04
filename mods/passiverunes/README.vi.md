<div align="center">

# PassiveRunes

**Nhận (hoặc mất) rune theo thời gian chơi.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10528-orange)](https://www.nexusmods.com/eldenring/mods/10528)

</div>

---

## Giới thiệu

PassiveRunes cộng rune cho bạn đều đặn khi đang chơi, kèm bonus ở các mốc thời gian chơi từ 30 phút
đến 24 giờ. Mọi giá trị đều có thể âm để trừ rune thay vì cộng. Mod đọc và ghi số rune trực tiếp,
không cần patch code.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/10528).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `PassiveRunes.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `PassiveRunes.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [PassiveRunes.toml](PassiveRunes.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game. Danh sách mốc thời gian chỉ được đọc một lần lúc khởi động.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Passive Runes**
cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `PassiveRunes.ini`. Nếu chưa có `PassiveRunes.toml`, cài đặt trong file ini được
chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/10528): tải về và bình luận.
- [docs/rune_math.md](docs/rune_math.md): ba thành phần của mỗi lần thưởng, giá trị âm và các mốc thời gian.
