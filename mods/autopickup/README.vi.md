<div align="center">

# AutoPickup

**Tự nhặt đồ khi bạn đứng trong tầm, không cần bấm nút tương tác.**

[English](README.md)

![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
[![Nexus Mods](https://img.shields.io/badge/Nexus_Mods-11470-orange)](https://www.nexusmods.com/eldenring/mods/11470)

</div>

---

## Giới thiệu

Trong Elden Ring, mỗi cây cỏ, vật phẩm dưới đất hay xác chết đều cần bạn đứng đúng chỗ và bấm nút.
AutoPickup bấm giúp: nguyên liệu, vật phẩm, đồ trên xác và rune bạn đã đánh rơi. Cần gạt, cửa, grace
và hội thoại được để yên. Mỗi nhóm bật tắt riêng và có tầm nhặt riêng.

Không chạy được với Seamless Co-op, hoặc cùng một DLL khác cũng tự nhặt đồ.

## Cài đặt

1. Tải mod từ [trang Nexus](https://www.nexusmods.com/eldenring/mods/11470).
2. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
3. Đặt `AutoPickup.dll` vào thư mục mod của loader và khai báo nó.
4. Chạy game. `AutoPickup.toml` được tạo cạnh DLL ở lần chạy đầu. Ý nghĩa từng key nằm ở chú thích trong [AutoPickup.toml](AutoPickup.toml); sửa file rồi bấm `F5` (mặc định) để áp dụng, không cần khởi động lại game.

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Auto Pickup** cũng xuất hiện trong System > Game Options. Đây là tùy chọn.

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [Trang Nexus](https://www.nexusmods.com/eldenring/mods/11470): tải về và bình luận.
- [docs/exec_action_button.md](docs/exec_action_button.md): cơ chế, điểm hook, rủi ro.
- [docs/pickup_animation.md](docs/pickup_animation.md): cách tắt animation nhặt.
