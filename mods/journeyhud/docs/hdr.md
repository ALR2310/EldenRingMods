# Overlay quá sáng trên màn hình HDR

**Status 2026-11-02: đã có code (`[Rendering] HDRMode`, hudhook chép vào `vendor/`), người dùng đã thử với ERSS-FG (`Auto` chọn HDR10, minimap hết cháy màu cam); chờ liziai996 xác nhận trên máy họ.**

Báo cáo: liziai996 (Nexus, 2026-11-01), Convergence 3.0.4.1 + ERSS-FG 4.14.1 (`IsHDR = true`, `Force10Bit = true`): swap chain `DXGI_FORMAT_R10G10B10A2_UNORM`, overlay sáng và bão hoà hơn HUD của game (đo trên ảnh 4K: luma trung bình vùng minimap 59,8 so với 41,8 của cảnh; bão hoà 178 so với 148). Cách tránh duy nhất của họ là giảm `HDRBrightnessHUD` của ERSS, làm tối cả HUD của game.

## Nguyên nhân

Màu ImGui và texel icon là sRGB 8 bit; hudhook vẽ thẳng vào back buffer, không đổi màu. Trên swap chain SDR đúng như vậy. Trên swap chain HDR10 (ST2084, Rec.2020) cùng con số được đọc như PQ nên trắng 1.0 thành 10.000 nit, và sRGB đọc như Rec.2020 nên màu bão hoà hơn. Với scRGB (FP16 tuyến tính, 1.0 = 80 nit) cũng sáng hơn nhiều. Suy luận từ số đo và cấu hình ERSS, **chưa kiểm** bằng ảnh chụp HDR của ta.

## Cách sửa

hudhook dùng pixel shader cố định nên không đổi được từ ngoài. Ta chép hudhook 0.9.3 vào `vendor/hudhook` (xem `vendor/README.md`) và đổi pixel shader DX12 (`src/color.rs` trong bản chép): sau khi tính `màu đỉnh * texel`, chuyển

- **HDR10:** sRGB sang tuyến tính, ma trận Rec.709 sang Rec.2020, nhân `203 nit * HDRBrightness`, mã hoá PQ;
- **scRGB:** sRGB sang tuyến tính, nhân `203 / 80 * HDRBrightness`.

203 nit là trắng tham chiếu HDR (BT.2408): trắng của overlay ngang trắng giấy của HUD tiêu chuẩn. `HDRGamma` là số mũ áp lên giá trị tuyến tính trước khi mã hoá. Pha màu (blend alpha) vẫn làm trong không gian đã mã hoá, như khi SDR.

Chế độ (`[Rendering] HDRMode`):

| Giá trị | Ý nghĩa |
|---|---|
| `Auto` (mặc định) | chuyển nếu format là `R10G10B10A2_UNORM` (HDR10) hoặc `R16G16B16A16_FLOAT` (scRGB) **và** màn hình chứa cửa sổ đang ở HDR (`IDXGIOutput6::GetDesc1().ColorSpace` là G2084) |
| `SDR` | không chuyển, như bản 1.0.0 |
| `HDR10` / `scRGB` | ép kiểu đó, bỏ qua dò |

Dò là đoán: DXGI không cho đọc colour space của swap chain, chỉ của màn hình. Người dùng Windows HDR bật nhưng game SDR (`R8G8B8A8`) không bị ảnh hưởng vì format không khớp; chỉ khi game dùng swap chain 10-bit mà colour space là sRGB thì `Auto` đoán sai, khi đó đặt `SDR`.

Đọc khi pipeline dựng (frame đầu và mỗi lần game tạo lại swap chain), nên không áp dụng khi hot reload; log ghi `Overlay: ImGui initialized, colour output <kiểu> (mode, format, monitor HDR)`.

## Chưa làm / chưa kiểm

- Ta mới thử trên một máy HDR, chưa đối chiếu độ sáng với HUD gốc bằng ảnh. Cần liziai996 thử bản `Auto`, gửi dòng log `colour output`, và nói trắng/màu đã khớp HUD chưa; chỉnh `HDRBrightness` rồi `HDRGamma` nếu lệch.
- Ngoài ra ERSS có thể tự áp bước ánh xạ riêng sau lớp phủ của ta (người báo nói "lifted by the output mapping"); nếu vậy số 203 nit cần hệ số khác, chỉnh bằng `HDRBrightness`.
- Mod khác dùng hudhook (SoulsTeleport, `common` overlay) cũng dùng bản chép nhưng không gọi `color::set` nên mặc định `Sdr`, vẽ y như trước.

## Liên quan

Cùng người báo, crash khi bật DLSS-G: nạp DLL mặc định chết tại `ERSS-FG.dll+0x13d87c` lúc ERSS tạo proxy Streamline cho `IDXGIFactory`; trì hoãn nạp khoảng 20 giây (`delay,20000` trong `.me3`) hết lỗi. Không tái hiện được ở máy ta; cùng họ với `questpath.md` và `overlay_swapchain.md` (hook DXGI/DX12 của hudhook đụng mod khác). Tạm thời dùng `delay` ở `.me3`.
