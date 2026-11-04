<div align="center">

# FasterRevival

**Hồi sinh sớm hơn sau khi chết: không phải đợi animation ngã hay màn "YOU DIED".**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11160-orange)](https://www.nexusmods.com/eldenring/mods/11160)

</div>

---

## Giới thiệu

Khi chết, Elden Ring bắt bạn xem hết animation ngã, đợi màn "YOU DIED" hiện, rồi đợi thêm một lần
nữa trước khi hồi sinh. FasterRevival cắt cả hai khoảng chờ: chết thường hồi sinh sau khoảng 4 đến 5
giây thay vì 12 đến 14. Mod luôn bật và không có gì phải chỉnh. Mod không sửa file animation nên
không xung đột với các mod animation.

Rơi khỏi bản đồ thì không đổi, và chưa kiểm tra khi đang cưỡi Torrent.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11160).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `FasterRevival.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `FasterRevival.toml` được tạo cạnh DLL ở lần chạy đầu; file chỉ có công tắc ghi log và một tùy chọn chẩn đoán, giải thích ở chú thích trong file.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11160): tải về và bình luận.
- [docs/death_flow.md](docs/death_flow.md): game ghi nhận cái chết thế nào, và mod kéo mốc đó lên sớm ra sao.
- [docs/respawn_fade.md](docs/respawn_fade.md): khoảng dừng sau "YOU DIED".
