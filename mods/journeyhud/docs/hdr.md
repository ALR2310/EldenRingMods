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
| `Auto` (mặc định) | theo colour space mà game hoặc mod khác vừa đặt cho swap chain bằng `SetColorSpace1`: G2084 (HDR10) chuyển như `HDR10`, G10 Rec.709 (scRGB) chuyển như `scRGB`, còn lại không chuyển. Đổi theo ngay khi bật/tắt HDR trong game (vd. menu ERSS) |
| `SDR` | không chuyển, như bản 1.0.0 |
| `HDR10` / `scRGB` | ép kiểu đó |

**Cách `Auto` biết colour space.** DXGI không có hàm đọc colour space của swap chain, nên bản hudhook chép (`color.rs`) đổi con trỏ slot `SetColorSpace1` (38) trong vtable của swap chain tạm: chỉ sửa dữ liệu, không vá mã nên không xung đột với hook của Steam overlay, QuestPath hay ERSS. Vtable nằm trong `dxgi.dll` và dùng chung, nên mọi swap chain cùng lớp đi qua hàm ghi lại giá trị rồi gọi hàm gốc. Hai giới hạn: (1) lần gọi xảy ra **trước** khi hook cài thì ta không thấy (khi đó `Auto` ở `SDR` tới lần gọi sau; hoặc đặt tay `HDR10`); (2) nếu mod như ERSS gọi trên lớp swap chain khác (proxy riêng) thì ta không thấy, cần kiểm bằng log.

Mode, độ sáng và gamma không còn viết cứng vào shader: renderer đặt chúng vào hàng thứ ba của ma trận chiếu (vertex shader không dùng, vì z = 0), pixel shader đọc từ đó mỗi frame. Nên đổi `HDRMode`, `HDRBrightness`, `HDRGamma` rồi bấm F5 là có hiệu lực, không cần khởi động lại. Đã từng thử cách đọc `IsHDR` trong `ERSS-FG.toml`: **bỏ**, vì tên file và tên khoá có thể bị tác giả đổi (người dùng, 2026-11-02).

Log ghi `Overlay: colour output <kiểu> (swap chain colour space N, set M time(s), HDRMode ...)` mỗi khi giá trị đổi.

## Chưa làm / chưa kiểm

- **Chưa thử** `Auto` theo `SetColorSpace1`: cần bật/tắt HDR trong menu ERSS khi game chạy và xem log `colour output`, để biết ERSS có gọi qua lớp ta bắt được không (nếu `set 0 time(s)` thì không). Ta mới thử trên một máy HDR, chưa đối chiếu độ sáng với HUD gốc bằng ảnh. Cần liziai996 thử bản `Auto`, gửi dòng log `colour output`, và nói trắng/màu đã khớp HUD chưa; chỉnh `HDRBrightness` rồi `HDRGamma` nếu lệch.
- Ngoài ra ERSS có thể tự áp bước ánh xạ riêng sau lớp phủ của ta (người báo nói "lifted by the output mapping"); nếu vậy số 203 nit cần hệ số khác, chỉnh bằng `HDRBrightness`.
- Mod khác dùng hudhook (SoulsTeleport, `common` overlay) cũng dùng bản chép nhưng không gọi `color::set` nên mặc định `Sdr`, vẽ y như trước.

## ERSS-FG 4.14.1: HDR của nó hoạt động thế nào (phân tích bản dump, 2026-11-02)

`ERSS-FG.dll` bị pack nên không đọc tĩnh từ file được; dump từ game đang chạy bằng `scripts/dump_module.py ERSS-FG.dll --proc eldenring.exe --out tmp/scratchpad/erss` (bản dump nằm ở `tmp/scratchpad/erss/`, gitignore), rồi phân tích tĩnh. RVA là của bản 4.14.1, **đổi theo bản**. Dưới đây [XN] = xác nhận từ code, [SD] = suy đoán.

- ERSS là một ReShade addon, tự nhúng swap chain kiểu FidelityFX (log `InitSwapChain`, `UpdateSwapChainColorSpace`). Cấu hình là struct toàn cục: `OverrideColorSpace` (0x312c33), `Force10Bit` (0x312c36), `IsHDR` (0x312c39) [XN].
- **Ô HDR của ERSS** (`IsHDR`) báo cho ERSS rằng tín hiệu là HDR; tooltip của nó nói cần bật HDR trong Windows. Khi bạn bấm ô này, hàm 0xbc930 dựng lại pipeline rồi gọi 0x192750, hàm này gọi `SetColorSpace1(12)` (G2084 P2020, tức HDR10) trên một đối tượng toàn cục (0x327268) [XN]. Khi tắt thì đặt lại colour space gốc.
- `SetColorSpace1(12)` ở lúc khởi tạo swap chain (0x16ed30) chỉ chạy khi `OverrideColorSpace` bật [XN].
- ERSS tự chuyển đổi tín hiệu HDR cho Frame Generation và tự vẽ UI/HUD vào swap chain của nó (`HDRBrightnessHUD`) [XN về tồn tại, chưa giải công thức]. Overlay của ta vẽ ngay trước `Present` thật nên nhiều khả năng nằm **sau** các pass này [SD].

**Hệ quả:** đối tượng 0x327268 là swap chain của ERSS (proxy Streamline hoặc swap chain FidelityFX) [SD], khác lớp mà bản `SetColorSpace1` của ta vá, nên ta không thấy lời gọi nào khi bạn bật tắt ô HDR. Không có tín hiệu DXGI chung nào để suy ra HDR của ERSS (format luôn `R10G10B10A2` do `Force10Bit`, Windows HDR vẫn tắt). Cách duy nhất là đọc cờ nội bộ của ERSS (byte `IsHDR` ở RVA cố định của bản 4.14.1), mong manh và gắn với một bản; **không làm** (người dùng, 2026-11-02: tên và địa chỉ có thể bị tác giả đổi, để thủ công còn hơn). Vì vậy với ERSS đặt tay `HDRMode` (`HDR10` khi ô HDR của ERSS bật), `Auto` chỉ theo được game hoặc mod gọi `SetColorSpace1` qua lớp ta vá.

## Liên quan

Cùng người báo, crash khi bật DLSS-G: nạp DLL mặc định chết tại `ERSS-FG.dll+0x13d87c` lúc ERSS tạo proxy Streamline cho `IDXGIFactory`; trì hoãn nạp khoảng 20 giây (`delay,20000` trong `.me3`) hết lỗi. Không tái hiện được ở máy ta; cùng họ với `questpath.md` và `overlay_swapchain.md` (hook DXGI/DX12 của hudhook đụng mod khác). Tạm thời dùng `delay` ở `.me3`.
