<div align="center">

# RuneMultiplier

**Nhân hệ số lên số rune nhận được từ mọi nguồn.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10630-orange)](https://www.nexusmods.com/eldenring/mods/10630)

</div>

---

## Giới thiệu

RuneMultiplier nhân số rune bạn nhận được lên một hệ số tùy chọn: giết quái, dùng hoặc bán vật phẩm
và các nguồn khác. Hệ số lớn hơn 1 cho nhiều rune hơn, nhỏ hơn 1 cho ít hơn, và số âm thì trừ rune.
Rune chi tiêu, khi lên cấp hay mua đồ, không bị ảnh hưởng.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/10630).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `RuneMultiplier.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `RuneMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [RuneMultiplier.toml](RuneMultiplier.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Rune
Multiplier** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `RuneMultiplier.ini`. Nếu chưa có `RuneMultiplier.toml`, cài đặt trong file ini
được chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/10630): tải về và bình luận.
- [docs/addsoul_hook.md](docs/addsoul_hook.md): hook, vì sao khoản chi tiêu không bị nhân, kiểm tra tương thích.
