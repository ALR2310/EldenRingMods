# input: phím nóng và vì sao có `is_key_pressed` riêng

**Status 2026-10-13: `common::input::is_key_pressed` được `soulsteleport` và `weightmultiplier` dùng. `common::reload` và `autoregen` vẫn dùng `eldenring::util::input::is_key_pressed` (còn mở, xem TODO.md). Code ở `shared/src/input.rs`.**

## Hai hàm cùng tên

| | `eldenring::util::input::is_key_pressed` | `common::input::is_key_pressed` |
|---|---|---|
| Cơ chế | `GetKeyState`, debounce theo VK trong một bản đồ dùng chung (`DEBOUNCE_MAP`) | `GetAsyncKeyState` + trạng thái riêng từng phím |
| Gọi từ | task của game (đăng ký trên scheduler của game) | thread thường (`Sleep`) hoặc task đều được |
| Chỉ tính khi cửa sổ game có focus | không | có |
| Cần `fromsoftware-rs` | có | không |

`weightmultiplier` cố tình không phụ thuộc `fromsoftware-rs` nên không có
scheduler của game; nó dùng bản của `common`.

## Vì sao có bản riêng (2026-09-24)

Bản đầu chỉ là `GetAsyncKeyState(vk) & 1`. Bit thấp đó ("đã bấm kể từ lần gọi
trước") là **một cờ chung toàn hệ thống**, ai gọi trước thì xoá cờ. Khi chạy 2
instance game trên 1 máy (thử Seamless Co-op), poll của máy chủ cứ nuốt lượt bấm
dành cho cửa sổ kia, và hotkey còn nổ khi gõ trong ứng dụng khác.

`GetKeyState` (bản của `eldenring`) gọi từ thread task của game cũng không đủ: thread đó
không có hàng đợi input riêng nên nó phản ánh phím của cả desktop. Kiểm tra
co-op số 4 (2026-09-24): bấm một phím nóng ở 1 cửa sổ làm **cả hai** instance
game cùng warp (xem `mods/soulsteleport/docs/position_exchange.md`).

Bản hiện tại:

- Không poll gì cả khi cửa sổ foreground thuộc process khác, nên không nuốt
  lượt bấm của cửa sổ đó.
- Khi lấy lại focus, lần poll đầu chỉ ghi nhận trạng thái, để một lượt bấm ở app
  khác trước lúc alt-tab về không bị tính.
- Nổ khi thấy bit "đã bấm" (bắt được cú chạm ngắn hơn chu kỳ poll) hoặc khi thấy
  chuyển từ nhả sang nhấn.

## Chỉ một người gọi cho mỗi phím

Trạng thái lưu theo phím, không theo người gọi. Hai nơi cùng poll một phím thì
chỉ nơi gọi trước thấy `true`. Vì vậy `common::reload::run` phải là nơi **duy
nhất** poll `ReloadKey` trong một process; các module khác theo dõi
`RELOAD_GENERATION` thay vì tự poll. (Điều này đúng với cả hai bản của hàm.)

## parse_virtual_key

Nhận, theo thứ tự ưu tiên: mã VK dạng hex (`0x2D`) hoặc thập phân (`112`),
`F1`..`F24`, hoặc một chữ cái/chữ số. Mọi thứ khác (rỗng, gõ sai, ngoài
khoảng 1..=0xFF) trả về `fallback`.
