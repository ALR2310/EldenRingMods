# toml_config: cấu hình TOML cho mod có danh sách và bảng

**Status 2026-10-13: hoạt động, dùng bởi speedmultiplier và animcancel (các mod còn lại dùng `.ini`, xem `shared/docs/config.md`). Code ở `shared/src/toml_config.rs`, có test cho migration, gộp mẫu và lưu giá trị. Phần riêng của SpeedMultiplier: `mods/speedmultiplier/docs/config_toml.md`.**

## Vì sao có (2026-10-02)

Cho mod cần cấu hình vượt khỏi ini phẳng: danh sách quy tắc, bảng theo từng mục
(quy tắc SpEffect của SpeedMultiplier). Bộ đọc ini (`common::config`) gộp mọi
`[Section]` vào một namespace và không có mảng, nên hình dạng đó phải đánh số key
(`Speed01`...).

## Khai báo cấu hình

Mỗi mod khai báo một struct serde `T` với `#[derive(Default)]` +
`#[serde(default, deny_unknown_fields)]` trên **mọi** struct. **Giá trị mặc định
chỉ sống trong mẫu nhúng của mod** (2026-10-02; trước đó mỗi giá trị bị viết hai
lần, ở mẫu và ở một `Default` viết tay, giữ bằng nhau bằng một test).

File được đọc hai lượt (`parse`):

1. Chỉ văn bản của người dùng, để báo lỗi kèm số dòng (khoá thiếu nhận giá trị
   không của `Default` ở lượt này).
2. Giá trị của người dùng đặt chồng lên mẫu, ra config thật: khoá thiếu nhận giá
   trị của mẫu.

`T::default()` (toàn số không) không bao giờ là config đang dùng; `defaults` là giá
trị của mẫu. Test của mod nên khẳng định `template_missing_keys` rỗng, nếu không
một trường mà mẫu không có sẽ âm thầm bằng không.

- Khoá thiếu rơi về mẫu, nên file cũ vẫn nạp được.
- Khoá lạ/gõ sai và sai kiểu là **lỗi kèm dòng** (thông báo của chính toml), không
  bị bỏ qua, lúc reload khi người dùng đang sửa. Lúc khởi động chúng bị đưa đi
  trước (bên dưới).
- File lỗi parse **không bao giờ** thay config đang dùng: lúc khởi động mod chạy
  bằng giá trị của mẫu, lúc reload giữ config tốt gần nhất.

## Migration có phiên bản (2026-10-02)

Thay cho danh sách đổi tên phẳng, vốn phụ thuộc thứ tự của chính nó, không tách
hay chuyển đổi được một khoá, và một mục bị bỏ làm mất giá trị âm thầm. File mang
phiên bản định dạng ở `Migration::version_key` (vd. `General.ConfigVersion`); giá
trị của mẫu ở đó là phiên bản hiện tại. File không có là phiên bản 1, định dạng
phát hành đầu tiên. `Migration::steps[i]` đưa file từ phiên bản `i + 1` lên `i + 2`,
nên mỗi bước chạy đúng một lần cho mỗi file, theo thứ tự.

**Một bước đã phát hành không bao giờ được đổi:** file đã qua nó sẽ không chạy lại;
sửa lỗi bằng một bước mới.

## Đưa file về khớp mẫu lúc khởi động

Qua `toml_edit` để comment, giá trị và bố cục của người dùng được giữ nguyên:

1. Các bước migration file chưa chạy, rồi đặt phiên bản về hiện tại.
2. Thêm khoá và bảng mà mẫu có còn file thiếu, kèm comment của mẫu (khoá mới sau
   cập nhật).
3. Khoá mà mẫu không có bị lấy ra và ghi lại thành comment trong một khối ở cuối
   file: tương đương `[Legacy]` của ini nhưng là comment để file vẫn parse được.
   Tên cấp cao nhất trong `Migration::keep` được để yên (danh sách mà mẫu không có
   mục, vd. `[[Override]]`).

File từ phiên bản **mới hơn** (mod bị hạ cấp) không bị đụng tới, nếu không bước 3
sẽ xoá các khoá mới. File không phải TOML hợp lệ cũng để yên (bước parse sẽ báo
lỗi). Không có file thì ghi mẫu ra. Mọi lần ghi đi qua `config::write_atomic`.

## Dùng

Cấu hình đã parse là một ảnh chụp `Arc<T>`: gọi `TomlConfig::get` một lần mỗi tick
rồi đọc trường thường, không tra chuỗi theo khoá mỗi giá trị. `save_values` ghi
`("Section.Key", "chữ")` vào file mà giữ nguyên comment và bố cục (mỗi chữ thành
bool, số nguyên, số thực hoặc chuỗi) rồi đọc lại.

`key_name` là `deserialize_with` cho trường phím nóng đọc bằng
`input::parse_virtual_key`: nhận cả chuỗi (`"F5"`, `"0x74"`) lẫn số trần (`116`),
vì ini nhận cả hai còn TOML sẽ từ chối số ở chỗ chuỗi.

Ghi log và phím reload vẫn lấy từ `logger` / `reload` dựa trên ini theo mặc định;
mod TOML điều khiển chúng bằng `logger::set_enabled` và `reload::run_with`.
