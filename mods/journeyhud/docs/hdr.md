# Overlay quá sáng trên màn hình HDR

**Status 2026-11-02: đã ghi nhận báo cáo và hướng điều tra, chưa kiểm trên màn hình HDR hay sửa code.**

Báo cáo: liziai996 (Nexus, 2026-11-01), Convergence 3.0.4.1, ERSS-FG 4.14.1 với `IsHDR = true`, `Force10Bit = true`.

## Hiện tượng

- Swap chain là `DXGI_FORMAT_R10G10B10A2_UNORM` (theo log ERSS). Toàn bộ overlay sáng và đậm màu hơn HUD gốc của game; icon và chữ trên minimap hoà vào nền bản đồ sáng.
- Đo trên ảnh chụp 4K: luma trung bình vùng minimap 59.8 so với 41.8 của cả khung hình; 5.2% pixel có luma ≥ 240; độ bão hoà 178/255 so với 148/255.
- Cách tránh duy nhất của họ: hạ `HDRBrightnessHUD` của ERSS, nhưng nó làm tối cả HUD gốc.

## Nguyên nhân (người báo đọc DLL, chưa kiểm)

hudhook vẽ ImGui thẳng vào backbuffer; JourneyHUD không xử lý HDR, tonemap, sRGB hay gamma, nên màu ImGui bị bộ ánh xạ đầu ra HDR nâng lên.

## Hướng sửa (họ đề xuất)

- `[Rendering] HdrMode = "Auto" | "SDR"` cùng `HdrBrightness` hoặc `HdrGamma`.
- Hoặc tự nhận định dạng swap chain rồi bù màu cho màu đỉnh (vertex colour) trước khi vẽ.

## Cần làm trước khi sửa

Cần máy HDR (hoặc ảnh chụp từ họ) để kiểm; ảnh chụp minimap và khung hình đầy đủ đang nằm trong bình luận của họ trên trang Nexus.

## Liên quan

Cùng người báo, crash khi bật DLSS-G: nạp DLL mặc định chết tại `ERSS-FG.dll+0x13d87c` lúc ERSS tạo proxy Streamline cho `IDXGIFactory`; trì hoãn nạp khoảng 20 giây (`delay,20000`) hết lỗi. Cùng họ với `questpath.md` và `overlay_swapchain.md`: hook DXGI/DX12 của hudhook đụng mod khác.
