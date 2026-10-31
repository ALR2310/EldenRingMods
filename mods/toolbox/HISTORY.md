# Toolbox: dòng thời gian

Mỗi dòng một thay đổi, mới nhất ở dưới cùng.

| Ngày | Thay đổi | Chi tiết |
|---|---|---|
| 2026-10-31 | Tạo mod: chuyển MapAnywhere và TorrentAnywhere từ SomeTweaks (`ApplySpEffect` gọi qua AOB thay `rva::get()`), thêm công cụ Buildup (kích qua `ChrIns::ApplySpEffect` `6400..6800` và `101..109` để hiện thanh HUD và nổ hiệu ứng thật) và GUI ImGui trong game (`F8`, nút riêng cho từng loại buildup + thanh trượt Amount + nút xoá, trích `common::overlay` từ SoulsTeleport kèm chống tranh chấp hook DX12 với `JourneyHUD` và hook `DLUID::MouseDevice::Poll` + `System32\dinput8.dll` để chặn chuột vào game khi mở menu). | [README](README.md), [overlay](../../shared/docs/overlay.md) |
