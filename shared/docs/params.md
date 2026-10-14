# params: đọc và ghi param không qua `rows()` / `rows_mut()`

**Status 2026-10-13: hoạt động, dùng bởi dropmultiplier, fasterrevival, speedmultiplier, spiritmultiplier (`for_each_row_mut`, `row_ids`, `check`). `risearcher` và `infiniteailment` chưa chuyển (xem TODO.md). Code ở `shared/src/params.rs`. Không có test tự động: mọi hàm cần `SoloParamRepository` thật của game, nên kiểm bằng log trong game.**

## Vì sao không dùng `rows()` / `rows_mut()`

Nguyên nhân gốc và bằng chứng (Convergence, `ItemLotParam_enemy` có header 4631
dòng nhưng bảng lookup của game chỉ 4630, fromsoftware-rs đọc lố một mục và
panic) nằm ở `mods/dropmultiplier/docs/convergence_panic.md`. Tóm lại: bảng
lookup của game do game tự dựng và có kích thước theo số dòng runtime, nhỏ hơn
header; vanilla không bị.

Hướng thay thế: `for_each_row_mut` duyệt theo **index** qua row descriptor của
chính file param (cùng dữ liệu `get_row_by_index_mut` đã đọc), không đụng bảng
lookup và dừng ở `row_count`. Đổi lại người gọi nhận chỉ số dòng thay vì param
ID. Cần tra theo ID thì dùng `row_ids`, dựng bản đồ ID → index một lần.

## `check`: khoá slot cứng của fromsoftware-rs

fromsoftware-rs cứng hoá slot của mỗi param và chỉ `debug_assert!` rằng slot
đó đúng, tức không bao giờ kiểm trong bản release. Trên phiên bản game có thứ tự
param khác, ghi qua slot sai sẽ âm thầm làm hỏng một param khác. `check` so tên
resource và tên struct trước khi ghi.

## `row_ids`: bố cục row descriptor

fromsoftware-rs để `row_data_offset` ở chế độ private, nên mã tự đọc descriptor
nằm ngay sau header. Bố cục (phản chiếu `RowDescriptor<T>` của nó):

- `id: u32`, rồi `data_offset: T`, rồi `name_offset: T`.
- `T = u64` (descriptor 24 byte) khi bit 2 của `format_2d` (byte `0x2D` của header)
  được đặt, ngược lại `T = u32` (12 byte).
- Descriptor bắt đầu sau header, cộng thêm `0x10` byte khi có header mở rộng
  (`T = u64`, hoặc bit 0 và bit 1 của `format_2d` cùng bật).

Mỗi `data_offset` được đối chiếu với con trỏ dòng mà `get_row_by_index` trả về.
Lệch ở bất kỳ dòng nào (giả định bố cục sai trên một phiên bản game tương lai)
thì hàm trả `None`, không trả ID sai.

## Số dòng runtime

`describe` in cả số dòng ở header (`u16`, trong file) lẫn số dòng runtime (`u32`
trong metadata `0x10` byte trước file, ngay sau một `u32` offset tên); bảng
lookup được dựng theo số thứ hai. Hai số này lệch nhau là dấu hiệu của đúng lỗi
Convergence nêu trên, nên có sẵn trong log báo lỗi.
