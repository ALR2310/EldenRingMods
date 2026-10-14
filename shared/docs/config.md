# config: đọc ini và gộp mẫu mặc định vào file của người dùng

**Status 2026-10-13: hoạt động, dùng bởi mọi mod cấu hình bằng `.ini`. Mod cần list/bảng dùng `shared/src/toml_config.rs` (`speedmultiplier`, `animcancel`). Code ở `shared/src/config.rs`, có test cho từng nhánh gộp.**

## Định dạng

`key=value` mỗi dòng, `#` hoặc `;` mở đầu comment. Tiêu đề `[Section]` được chấp
nhận nhưng bị bỏ qua: mọi key dùng chung một namespace, nên tên key phải duy
nhất trong 1 file. Giá trị đọc qua `get_string` / `get_int` / `get_double` /
`get_bool` kèm mặc định; không parse được thì trả về mặc định.

**Comment cuối dòng:** `;` hoặc `#` đứng sau khoảng trắng kết thúc giá trị
(`WarpWhenFar=true   ; ghi chú` đọc ra `true`). Trước đây toàn bộ phần chữ đó
bị coi là giá trị, `get_bool`/`get_double` không parse được và mod âm thầm rơi về
mặc định. `;` hoặc `#` dính liền giá trị (`a;b`, `a#b`) vẫn thuộc về giá trị.

Crate `common` không có ini riêng, nên mọi hàm cần mặc định nhận mẫu
`include_str!` của mod làm tham số.

## Gộp mẫu vào file có sẵn (`load_or_create_default`)

- **File chưa có:** ghi nguyên mẫu ra đĩa, để người dùng mới có file thật để sửa.
- **File đã có:** viết lại theo bố cục của mẫu. Comment và thứ tự lấy từ mẫu,
  mọi key người dùng đã có giữ nguyên giá trị, key còn thiếu nhận mặc định của
  mẫu. Không ghi đĩa nếu không có gì để gộp, đổi tên hay chuyển.
- **Key mẫu không còn định nghĩa** (đổi tên hoặc bỏ ở bản mới): chuyển xuống
  khối `[Legacy]` cuối file thay vì xoá, để không mất cấu hình của người dùng.
  Lỗi từng gặp: kiểm tra Legacy bị câu lệnh "không thiếu gì thì thoát" chặn, nên
  key cũ nằm lại mãi khi mẫu không thêm key mới; đã có test hồi quy.
- **Đổi tên key** (`load_or_create_default_with_renames`): file còn `old` mà
  chưa có `new` thì `new` nhận giá trị của `old`, và `old` bị bỏ chứ không chuyển
  sang Legacy. Không có cặp này, đổi tên key ở bản mới sẽ âm thầm đưa mọi giá trị
  người dùng đã chỉnh về mặc định.

Hàm trả về số key mới được gộp (key đổi tên không tính) để mod log một lần
khi logger đã sẵn sàng.

Khi viết lại, file luôn dùng xuống dòng `\n`, kể cả khi file cũ là CRLF.

## Ghi giá trị ngược (`set_values`)

Thêm 2026-09-25 cho SoulsTeleport (lưu vị trí menu); trước đó không mod nào ghi
giá trị ra ini. Dòng `key=value` đã có được viết lại tại chỗ nên comment, section
và thụt đầu dòng giữ nguyên; key chưa có dòng thì nối vào cuối file. Giữ kiểu
xuống dòng (LF hoặc CRLF) của file hiện có. Bản đồ giá trị trong bộ nhớ luôn
được cập nhật, kể cả khi ghi file thất bại.

Ghi file đi qua `write_atomic`: ghi ra `<file>.tmp` cạnh đó rồi đổi tên đè lên
file thật, để sập giữa chừng hoặc có người ghi thứ hai không để lại config ghi
dở. `toml_config` dùng chung hàm này.

## Tham chiếu

- `shared/src/config.rs`: cài đặt và test.
- `shared/src/reload.rs`: nạp lại config theo phím `ReloadKey`.
