# Menu cuộn thay vì phân trang

**Status 2026-10-13: nghiên cứu trên tài liệu ERNativeUI + decompile (2.7.1.0), CHƯA thử trong game. Kết luận: trang cài đặt thường (Game Options và các trang cấu hình)
không có cơ chế cuộn; game chỉ có danh sách cuộn thật ở màn hình gán phím (`KeyConfigDialog`), vốn chuyên cho hàng gán phím. Muốn cuộn cho slider/công tắc/ô số
phải tự "ảo hoá" hàng (fork host) hoặc giảm số hàng gốc. Hiện giữ phân trang.**

Nghiên cứu tiếp (cấu trúc GFX, `GridControl`, callback làm mới, probe trong game): [ingame_menu_newpage.md](ingame_menu_newpage.md). Đính chính: `0x869580` là dispatcher theo byte `[a2+8]`, không phải builder.

Bối cảnh: [ingame_menu.md](ingame_menu.md). Phân trang hiện tại là hàng Next/Previous do ERNativeUI chèn khi hết chỗ.

## Bằng chứng

- **Trang cấu hình thường có số vị trí hàng cố định trong GFX** (6 hàng vanilla; 13 khi vá `02_040_optionsetting.gfx`). Số hàng vật lý của một trang bị chặn thêm bởi mảng hàng
  `DLFixedVector` tối đa **16** phần tử (`0x928630`: `count + 1 > 0x10` là lỗi "out of memory"), nên dù vá GFX cũng không quá 16 hàng/trang.
- **Danh sách cuộn thật duy nhất mà ERNativeUI tìm thấy là màn hình gán phím** (`02_160_KeyConfiguration`, `CS::KeyConfigDialog`): 11 vị trí hàng
  `Item_0_0 … Item_10_0` được tái dùng như một viewport cuộn, cộng `ScrollBarV`/`ScrollBarV2`, mô hình danh sách động
  (builder `0x869580`), có trạng thái ô `PadCategory`/`KeyCategory` làm **tiêu đề nhóm**. ERNativeUI dùng nó cho Input Bindings (hàng gán phím), nhưng đó
  là hàng "nhãn + ô phím", không phải slider/công tắc/ô số, và bấm vào là vào chế độ chờ phím.
- ERNativeUI ghi "virtualization or scrolling" là việc còn thiếu cho tab tuỳ biến (`docs/research/architecture/configuration-class-map.md`), và "Reusable native
  page templates" nằm trong danh sách nghiên cứu tương lai (`docs/features.md`). Chưa có hứa hẹn nào.

## Hướng có thể làm (từ rẻ đến đắt)

1. **Giữ phân trang** nhưng giảm số hàng gốc: mỗi mod đã chỉ có 1 hàng gốc; 10 mod là 10 hàng. Có GFX có 9 chỗ trống nên chỉ vượt 1 hàng.
2. **Rải hàng gốc ra các trang có sẵn**: ERNativeUI cho thêm hàng vào Camera, Display, Sound, Network, Keyboard/Mouse, Graphics (`get_builtin_page`); trang
   Graphics kiểu Advanced Settings có 15 vị trí. Hợp lý về ngữ nghĩa với mod nào gần một mục (ví dụ WindowResize vào Display). Cần client gọi `get_builtin_page`.
3. **Hub chung**: một hàng "Mods" gom mọi mod. API hiện tại không cho nhóm xuyên provider, nên cần sửa host (hoặc một DLL hub đăng ký thay cho các mod).
4. **Ảo hoá hàng (fork host)**: giữ N hàng vật lý, khi tiêu điểm đi quá hàng cuối/đầu thì gán lại nội dung các hàng theo cửa sổ trượt (giống tái dùng 11
   vị trí của màn gán phím). Phải bắt phím lên/xuống và con lăn chuột, gán lại nhãn/bộ điều khiển giá trị (`PropertySliderController`...), giữ trạng thái focus. Công sức lớn,
   rủi ro với Solid Uncapper.
5. **Tái dùng danh sách của `KeyConfigDialog`** làm menu cuộn (hàng = nút): chỉ hợp cho hàng hành động, không cho slider/ô số. Hack, không đề xuất.

## Chưa kiểm chứng

- Số vị trí hàng thực tế của trang con (provider-owned) có bằng trang Game Options không; chưa đo trong game.
- Trang Graphics 15 vị trí có thật sự nhận hàng của mod hiển thị tốt không.

## Bỏ hẳn giới hạn trang con? (nghiên cứu 2026-10-13, IDA 2.7.1.0)

Câu hỏi: viết lại submenu để không còn giới hạn số hàng. Kết quả đọc `0x95B770` (bộ dựng trang con kiểu Advanced Settings), `0x94A590`/`0x94A140`/`0x949DC0`
(thêm hàng), `0x94D280`/`0x74EBC0` (slider):

- **Hàng gắn vào vị trí GFX theo chỉ số**: bộ dựng gọi `AddRow(page, nhãn, chỉ_số = base + N, dữ_liệu...)` (ví dụ `a2 + 402 … 402 + 11`), tức mỗi hàng của trang
  là một vị trí `Item_N_0` đã có sẵn trong movie. Số hàng tối đa = số vị trí trong movie (15 cho `02_042`, 6/13 cho `02_040`) và mảng hàng
  `DLFixedVector` của trang tối đa 16. Sửa GFX chỉ nâng được lên 16 (và chưa rõ lớp trang con có cùng trần 16).
- **Mỗi hàng chứa đủ 5 widget** (`Widgets/Slider`, `ComboBox`, `DropdownList`, `TextInput`, `Button`) và chỉ bật hiển thị widget đúng loại
  (`0x949DC0`/`0x9499A0`). Vì vậy một vị trí có thể đổi loại hàng khi tái dùng.
- **Slider giữ con trỏ giá trị**: `SliderCtrl+3648` = `uint8_t*` giá trị, `+3376..` = min/max/step, `+3368/+3372` = giá trị hiện tại/gốc; nhãn bằng cặp
  mã `GR_MenuText`/`GR_LineHelp`. Các bộ điều khiển khác (combo, toggle) có cấu trúc tương tự nhưng chưa đọc.

**Kết luận:** không có cách "bỏ giới hạn" bằng cách thêm hàng, vì giới hạn là số vị trí vật lý. Cách duy nhất không giới hạn là **tái dùng vị trí (cuộn ảo)**:
giữ N vị trí, khi tiêu điểm đi quá hàng cuối/đầu thì **gán lại** nhãn, con trỏ giá trị, loại widget, min/max/step của các vị trí theo cửa sổ trượt, giống cách
màn gán phím tái dùng 11 vị trí. Việc cần làm (chưa làm): bắt điều hướng lên/xuống/con lăn chuột trong trang, hàm gán lại cho từng loại hàng (slider,
toggle, combo, ô nhập, nút), cập nhật chữ (`0x74FA20`), giữ focus, xử lý Back và phần dịch con chuột.
Ước lượng: lớn (nhiều ngày RE + thử), rủi ro xung đột với Solid Uncapper, và phải nằm trong bản fork của host.

**Bước rẻ hơn có thể làm trước:** "chuyển trang liền mạch" - đi xuống ở hàng cuối thì tự sang trang kế, đi lên ở hàng đầu thì về trang trước, bỏ hai hàng Next/Previous (thêm 2 chỗ mỗi
trang) và cảm giác gần như cuộn. Vẫn là phân trang bên dưới.
