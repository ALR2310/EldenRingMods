<div align="center">

# SoulsTeleport

**Dịch chuyển tới đúng chỗ đồng đội đang đứng trong Seamless Co-op.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11119-orange)](https://www.nexusmods.com/eldenring/mods/11119)

</div>

---

## Giới thiệu

SoulsTeleport là mod độc lập cho **Seamless Co-op** (cần cài Seamless Co-op). Bấm phím mở menu, chọn một người
trong phiên (kèm vai trò: Host, Co-op, Hunter...) và bạn được dịch chuyển tới đúng chỗ họ đang đứng, qua màn hình
tải, dù ở xa đến đâu. Vị trí được hỏi trực tiếp người đó qua Steam, nên **người được tới cũng phải cài mod**.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11119).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `SoulsTeleport.dll` vào thư mục mod của loader và đăng ký nó.
4. Chạy game. `SoulsTeleport.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [SoulsTeleport.toml](SoulsTeleport.toml). Menu mở bằng `Tab` (mặc định); đổi `Scale` cần khởi động lại.

Các bản trước dùng `SoulsTeleport.ini`. Nếu chưa có `SoulsTeleport.toml`, cài đặt trong file ini được mang sang
ở lần chạy đầu tiên; file ini không bị đụng tới.

Dùng ở chế độ offline với EAC tắt, giống Seamless Co-op.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11119): tải về và bình luận.
- [docs/warp_mechanism.md](docs/warp_mechanism.md): cách warp hoạt động.
- [docs/position_exchange.md](docs/position_exchange.md): lấy vị trí đồng đội qua Steam.
- [docs/roles_and_invaders.md](docs/roles_and_invaders.md): vai trò và chặn teleport giữa người thù địch.
- [docs/menu_ui.md](docs/menu_ui.md): menu, hook chuột và hiển thị.
