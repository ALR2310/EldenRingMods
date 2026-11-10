<div align="center">

# AnimCancel

**Cắt animation đang chạy bằng một hành động khác, không phải đợi nó kết thúc.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11328-orange)](https://www.nexusmods.com/eldenring/mods/11328)

</div>

---

## Giới thiệu

Trong Elden Ring, một animation chỉ cho phép hành động khác chen vào ở thời điểm nó tự chọn, thường
là nửa sau của đòn. AnimCancel mở thời điểm đó sớm hơn, để guard, lăn, nhảy và đổi vũ khí có hiệu
lực ngay khi bạn bấm. Bạn chọn hành động nào được cắt animation nào.

Chưa kiểm tra với Seamless Co-op.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11328).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `AnimCancel.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `AnimCancel.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [AnimCancel.toml](AnimCancel.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Anim Cancel** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

File cấu hình của bản 1.2.0 (`AnimCancelV1.toml`) được chép sang tên mới ở lần chạy đầu.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11328): tải về và bình luận.
- [docs/animation_cancel.md](docs/animation_cancel.md): cách cancel hoạt động và các lần thử.
- [docs/air_guard.md](docs/air_guard.md): vì sao bỏ đỡ đòn trên không.
- [docs/antivirus.md](docs/antivirus.md): cờ antivirus và những gì đã thay đổi chúng.
