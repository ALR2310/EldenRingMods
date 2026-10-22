# DropMultiplier: tỉ lệ rơi đồ (`ItemLotParam_enemy`)

**Status 2026-10-22: PHÁT HÀNH, đã test trong game. Code: `mods/dropmultiplier/src/drop_rate.rs` (`scale_row` có test cho từng trường hợp), `src/lot.rs` (đọc/ghi từng ô của dòng). Tách từ `sometweaks::drop_rate` ngày 2026-09-14, cùng thuật toán.**

## Mô hình bốc thăm của game

Cách game chọn món rơi là bốc thăm có trọng số **tương đối theo tổng của chính dòng
đó**. Mỗi dòng `ItemLotParam_enemy` có tối đa 8 ô, mỗi ô giữ một trọng số
(`lot_item_base_point0N`), và xác suất của ô N là:

`lot_item_base_point0N / tổng(lot_item_base_point01..08)`

Kết quả "không rơi gì", khi dòng có, chỉ là một ô tường minh khác
(`lot_item_id0N == 0`) và tham gia cùng phép tổng. Đã đối chiếu với một dòng thật trong
`ItemLotParam_enemy.csv` xuất ra từ SmithBox: cột "% chance to occur" mỗi ô khớp đúng
công thức. Ví dụ `985 / (985 + 15) = 98,5%`, và sau khi sửa trọng số ô thứ hai
`15 → 30`: `30 / (985 + 30) = 2,96%` và `985 / (985 + 30) = 97,04%`. Xác suất ô đầu
giảm vì nó không được nhân mà tổng của dòng thì tăng, không phải lỗi.

## Vì sao không đụng `cumulate_lot_point0N`

Mod cố ý để nguyên trường này. Cũng bản xuất đó cho thấy nó là `0` ở mọi ô của mọi
dòng, và SmithBox tính "% chance" chỉ từ `lot_item_base_point`, không đọc
`cumulate_lot_point`. Dù tên trường trong `ITEMLOT_PARAM_ST` (và tài liệu các game
Souls cũ về phép bốc thăm dùng tổng cộng dồn) gợi ý khác, trường này không phải thứ
vòng bốc thăm của Elden Ring tra. Ghi giá trị khác vanilla vào đó là dựa vào hành vi
chưa bản game nào từng chạy thật, mà không có lợi ích nào biết được.

## Hai chế độ (`Mode`)

Chỉ một trong hai, không bao giờ cả hai cùng lúc.

- **`Mode=0`, `Multiplier`:** nhân trọng số của mọi ô chứa đồ thật (`lot_item_id0N != 0`)
  với hệ số. Nhân đồng đều **cả ô "không rơi gì"** sẽ không đổi gì (tử và mẫu cùng
  nhân nên tỉ lệ giữ nguyên), nên ô đó được để nguyên.
- **`Mode=1`, `Percentage`:** ép tổng xác suất "có rơi món thật" của mỗi dòng đúng bằng
  tỉ lệ đã chọn, giữ nguyên tỉ lệ tương đối giữa các món cùng dòng.

## Đại số của `Percentage`

Với mỗi dòng, `itemSum` là tổng trọng số các ô đồ thật và `otherSum` là trọng số ô
"không rơi gì" (giữ nguyên). Giải `target = itemSum_new / (itemSum_new + otherSum)` theo
`itemSum_new`:

`itemSum_new = target × otherSum / (1 − target)`

Rồi nhân trọng số từng món thật với `itemSum_new / itemSum`: tổng của nhóm đúng bằng
`target`, phần chia của từng món giữ nguyên. Ba trường hợp biên:

- **`target ≥ 100%`:** công thức chia cho 0, nên thay vào đó trọng số mọi ô "không rơi gì"
  bị đặt về 0 (bảo đảm có đồ rơi, trọng số món thật không đổi vì chúng là trọng số
  khác 0 duy nhất còn lại).
- **Dòng không có ô "không rơi gì" (`otherSum == 0`):** đã luôn rơi 100%, không thể
  giảm xuống dưới 100%, nên `target < 100%` bỏ qua dòng đó (không có chỗ để dồn phần
  xác suất bị bớt đi).
- **Dòng không có món thật (`itemSum == 0`):** không có gì để ép tỉ lệ lên, giữ nguyên.

Hệ quả cần biết (đã ghi ở README): chế độ ép tỉ lệ có thể biến món vốn chắc chắn rơi
(100%) thành rơi theo xác suất.

## Hot reload không cộng dồn

Reload cần trọng số **gốc của game**, chụp một lần trước mọi chỉnh sửa, để tính lại từ
đó mỗi khi bấm `ReloadKey`. Tính lại từ giá trị đã sửa thì cộng dồn: nhân x2 rồi
reload x2 sẽ thành x4 chứ không giữ ở x2. Bản chụp được khoá theo **chỉ số dòng**
(không theo ID, xem `convergence_panic.md`) và bảo vệ bằng `Mutex`, vì lần chụp đầu
chạy trên thread của module còn các lần reload sau chạy trên thread nào mà nhóm task
`FrameBegin` của game chọn. Một `Mutex` bị poison vẫn dùng tiếp để một lần panic không
làm hỏng mọi lần reload sau đó.

## Tham chiếu

- `mods/dropmultiplier/docs/convergence_panic.md`: vì sao duyệt theo chỉ số dòng.
- `mods/dropmultiplier/docs/materials.md`: phần số lượng nguyên liệu (`[Materials]`).
- `shared/docs/params.md`: `for_each_row_mut`, `check`, `row_ids`.
