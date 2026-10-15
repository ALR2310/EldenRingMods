# MenuProbe (thử nghiệm, không phải mod)

Mod dò thuần quan sát: móc 5 hàm menu của game 2.7.1.0, **chỉ ghi log rồi chạy tiếp hàm gốc**, không đổi hành vi. Không đăng Nexus, không có CHANGELOG/HISTORY.
Nghiên cứu và câu hỏi nó trả lời: [docs/ingame_menu_newpage.md](../../docs/ingame_menu_newpage.md) (mục "`GridControl` và hàng widget").

Mỗi vị trí hook được so từng byte với bản 2.7.1.0 trước khi ghi; sai byte (bản game khác, hoặc mod khác đã hook) thì bỏ qua vị trí đó và ghi log.

## Hook

| Tên | RVA | Ghi log |
|---|---|---|
| `grid_update` | `0x73A140` | mỗi `GridControl` mới và mỗi lần số mục / chỉ số chọn / hàng cuộn đổi (vtable của grid giúp nhận ra loại) |
| `grid_select` | `0x73C240` | mỗi lần chọn mục đổi |
| `grid_refresh` | `0x73D0D0` | một phần các lần gọi callback "làm mới hàng đang hiển thị" |
| `keycfg_draw` | `0x8684D0` | một phần các lần vẽ hàng của màn gán phím |
| `prop_push` | `0x928630` | số thuộc tính (hàng) đã có trước mỗi lần thêm; cảnh báo ở hàng thứ 16 |

## Chạy
1. Build: `cargo build --release -p menuprobe` (ra `target/release/MenuProbe.dll`).
2. Chép `MenuProbe.dll` vào thư mục mod của game như mọi DLL khác. Tốt nhất để một mình, hoặc cùng ERNativeUI cũng được (log sẽ có thêm các trang của nó).
3. Vào game, nạp nhân vật, rồi **lần lượt**:
   - Mở System > Game Options, đi qua từng tab (Camera, Display, Sound, Network, Keyboard/Mouse, Graphics), di chuyển lên xuống từng trang.
   - Vào Keyboard/Mouse > gán phím (màn có danh sách phím): cuộn bằng phím lên/xuống, con lăn chuột và kéo thanh cuộn, chọn vài hàng, thoát bằng Esc.
   - Nếu dùng ERNativeUI: mở trang của một mod có nhiều hàng (ví dụ Auto Regen) và bấm Next/Previous.
4. Thoát game, gửi `MenuProbe.log` (cạnh DLL).

## Cách đọc log
- `grid_update new/changed: ... count=N index=I scroll=S`: `count` là số mục trong grid, `scroll` là hàng đầu đang hiển thị. Màn gán phím cuộn thì `scroll` phải đổi; trang Options có đổi không?
- `prop_push ... count_before=K`: trang Options có bao nhiêu hàng; hàng thứ 16 thì lần thêm tiếp theo là lỗi "out of memory".
- `keycfg_draw`: số lần vẽ cho mỗi lần cuộn cho biết mỗi lần chỉ vẽ lại hàng mới hay vẽ lại cả 11 vị trí.
- Dòng `skipped ...` nghĩa là byte tại RVA đó khác kỳ vọng.

Không có chức năng gỡ nóng: tắt game để gỡ.
