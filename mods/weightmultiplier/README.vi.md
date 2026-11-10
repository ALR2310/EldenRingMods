<div align="center">

# WeightMultiplier

**Nhân Trọng Tải với một hệ số, hoặc đặt cứng nó bằng một số.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-10549-orange)](https://www.nexusmods.com/eldenring/mods/10549)

</div>

---

## Giới thiệu

WeightMultiplier đổi tổng Trọng Tải (equip load) của bạn theo hệ số tùy chọn, hoặc đặt nó bằng đúng
một số bất kể bạn mặc gì. Một hook duy nhất áp cho cả số hiển thị trên giao diện lẫn hành vi thật,
kiểu lăn và tốc độ chạy. Hệ số nhỏ hơn 1 làm bạn nhẹ hơn, lớn hơn 1 nặng hơn. Mod trước đây tên là
ReductionWeight.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/10549).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `WeightMultiplier.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `WeightMultiplier.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [WeightMultiplier.toml](WeightMultiplier.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game. Mod đợi vài giây sau khi game khởi động rồi mới tự áp dụng (`LoadDelay`).

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Weight
Multiplier** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

Các bản trước dùng `WeightMultiplier.ini`. Nếu chưa có `WeightMultiplier.toml`, cài đặt trong file
ini được chuyển sang ở lần chạy đầu; bản thân file ini được để nguyên.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/10549): tải về và bình luận.
- [docs/hook_point.md](docs/hook_point.md): vị trí hook, cách tìm ra nó, chuyện gì xảy ra khi mod khác hook cùng chỗ.
