# Thanh trượt native: số âm và số thập phân

**Status 2026-10-13: nghiên cứu tĩnh xong (IDA, exe 2.7.1.0, và đọc `mod-settings-menu.dll`), CHƯA thử nghiệm trong game. Kết luận: số âm và thập phân làm được bằng ánh xạ hiển thị
(`min + chỉ_số x step`) với một hook ghi đè chữ của slider (`0x74FA20`), đã có tiền lệ; cũng có đường sửa 1 byte nếu muốn lưu thẳng số âm trong byte. Hiện tại menu dùng ô nhập số cho số âm/thập phân, nên chưa cần làm.**

**Cập nhật 2026-10-15: `NumberSetting::slider(from, to, step)` (ERNativeUI API 1.2) đã dùng ở Drop/Rune/Weight/Spirit/Speed/RiseArcher/AutoRegen; host API 1.1 tự lùi về ô gõ số (`shared/src/erui.rs`, chưa thử với host cũ trong game). AnimCancel chỉ có công tắc; InfiniteAilment và PassiveRunes giữ ô gõ số vì khoảng giá trị quá rộng (vd. 0..3.600.000 ms, -1000..1000) hoặc cần số lẻ mịn (0.18, 0.25), không vừa 256 mức.**

Bối cảnh: ERNativeUI (xem [ingame_menu.md](ingame_menu.md)) giới hạn slider ở một byte 0..255. Câu hỏi: giới hạn đó nằm ở game hay ở thư viện?

## Kết luận

Giới hạn **không nằm ở widget slider của game** mà ở các lớp bọc phía trên:

| Thành phần (RVA 2.7.1.0) | Kiểu | Hệ quả |
|---|---|---|
| `CS::SliderCtrl` (`0x74EBC0` dựng, `0x74FA20` cập nhật chữ) | **int32**: min/max/step ở `+3376..`, giá trị hiện tại/gốc ở `+3368`/`+3372` | Vị trí thanh tính bằng số nguyên có dấu (`0x74FB30`: `(giá trị - min) / (max - min)`), nên min âm được |
| Vẽ số bên phải (`0x74AD20`, gọi từ `0x74FA20`) | `printf(L"%d", int)` vào `Text_0` (giá trị) và `Text_1` (giá trị gốc) | Số âm hiện đúng. Số thập phân thì không in được |
| `CS::PropertySliderController<unsigned char>` (`0x94D280` dựng) | **byte**, ép kiểu khi ghi | Giá trị âm ghi thành byte (-5 thành 251); đọc lại bằng `int8` ra -5 |
| Đọc giá trị ban đầu (`0x94D340`: `44 0F B6 06` = `movzx r8d, byte ptr [rsi]`) | không dấu | Giá trị ban đầu -5 hiện thành 251 |

Hàm thêm hàng slider là `0x94A590(page, textRefs, uint8_t* value, SliderRange* range, uint8_t current, enabled)`;
`SliderRange` = `{int32 min, int32 max, int32 step, int32 reserved0, uint32 metadata = 0x22308, uint32 reserved1}` (0x18 byte).
`reserved0` và `metadata` chưa được host dùng.

## Số âm (rủi ro thấp)

1. Game: đổi 1 byte tại RVA **`0x94D342`** (`B6` thành `BE`): `movzx` thành `movsx` (`44 0F BE 06`), để giá trị ban đầu được mở rộng dấu.
   Cùng độ dài lệnh, nên chỉ cần ghi 1 byte (kiểm lại AOB/độ duy nhất qua `scripts/ida/aob.py -Version all` trước; RVA có thể lệch giữa các bản exe).
2. Host (fork ERNativeUI): bỏ chặn `minimum < 0` và `maximum > 255` ở `Registry::add_slider` (`src/host_registry.cpp:1193-1196`),
   coi byte của hàng là `int8` (-128..127) ở chỗ lưu, so sánh, gọi callback, `set_row_value`.
3. Client (`common::erui`): `add_slider` nhận `i32` min/max và callback đưa `i8`.

Phạm vi thanh trượt có dấu: -128 … 127.

## Số thập phân và số âm bằng ánh xạ hiển thị (đã có tiền lệ: `mod-settings-menu.dll`)

Đọc code của `tmp/repos/menu_in_game/mod-settings-menu.dll` (mã đóng, chỉ để hiểu kỹ thuật) cho thấy cách làm **không cần sửa dấu byte**:

- Nó cài 5 detour (MinHook) trên exe 2.7.1.0: `0x959DF0` (trang Game Options), `0x95B770` (trang con kiểu Advanced Settings), `0x764220` (tra chữ),
  **`0x74FA20`** (cập nhật chữ của `SliderCtrl`) và **`0x74F5D0`** (xử lý chuột kéo của `SliderCtrl`).
- Thanh trượt vẫn là **chỉ số nguyên** 0..N (N <= 255): với min/max/step kiểu `double` (vd. 0..10 bước 0,05 = 201 mức), giá trị thật
  = `min + chỉ_số x step`. Mỗi setting nằm trong một bảng nội bộ (mỗi mục 0x610 byte; byte lưu chỉ số ở `+0x40`, giá trị double hiện tại ở `+0x5C8`).
- Detour `0x74FA20`: nhận ra slider của nó bằng cách so con trỏ giá trị mà `SliderCtrl` giữ (`+0xD90`) với `+0x40` của từng mục; đọc chỉ số hiện tại ở `SliderCtrl+0xD28`
  (= `a1+3368` trong decompile của game), đổi sang `double`, báo cho luồng worker gọi callback; sau khi gọi hàm gốc nó gọi hàm vẽ chữ riêng (`0x180007380` trong DLL).
- Hàm vẽ chữ định dạng `double` bằng `printf("%.6g")` rồi ghi đè **`Text_0` / `Text_1`** của slider (tìm widget bằng `0x74B140`, đặt chữ bằng `0xD86050`);
  còn có chữ đặc biệt ("Use global", màu `<font color=...>`).
- Hook `0x74F5D0` để kéo chuột/hiệu chỉnh thanh (`Bar calibration`, `slider hit-area`).

**Kết luận:** số thập phân và số âm cho slider làm được chỉ bằng (1) hook `0x74FA20` ghi đè `Text_0`/`Text_1` theo `min + chỉ_số x step`, (2) giữ chỉ số là byte (mức tối đa 256),
(3) host/client nhận min, max, step kiểu số thực và trả giá trị thật cho callback. **Không cần vá `movzx`→`movsx`** (mục "Số âm" ở trên chỉ cần nếu muốn slider
không ánh xạ mà lưu thẳng số âm trong byte). Chuột có thể cần hook `0x74F5D0` nếu cách tính vị trí kéo của game không khớp với dải chỉ số.

## Chưa kiểm chứng (cần chạy game)

- Các phần khác của `SliderCtrl` (xử lý bấm trái/phải, kéo chuột, so sánh giá trị gốc `+3372`) có giả định không âm ở đâu không; mới đọc 2 hàm.
- Hook chữ có xung đột với Solid Uncapper (cũng hook hàm menu) hay không.
- Có tương thích 2.6.2.0 / 2.7.0.0 không (RVA khác; ERNativeUI chỉ hỗ trợ 2.7.0.0 và 2.7.1.0).

## Đường đi đề xuất khi thử

1. Dựng fork `tmp/repos/ERNativeUI` (MIT; máy có Visual Studio 2026 Community + CMake, xem `build.bat`).
2. Thử nhỏ nhất: chỉ số âm (3 việc ở trên), 1 slider -10 … 10 trong RuneMultiplier; xem hiển thị ban đầu, kéo, lưu, đọc lại.
3. Nếu tốt: làm số thập phân; sau đó đề nghị tác giả gộp thay đổi (hoặc phát hành bản fork, người dùng thay DLL host).

Hiện không cần: ô nhập số (`NumberSetting`, xem [ingame_menu.md](ingame_menu.md)) đã đáp ứng số âm và số lẻ.
