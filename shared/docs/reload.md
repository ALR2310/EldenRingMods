# reload: nạp lại config bằng phím nóng

**Status 2026-10-19: hoạt động, dùng bởi dropmultiplier, infiniteailment, animcancel, runemultiplier và các mod khác. Phím đọc bằng `common::input::is_key_pressed` (`GetAsyncKeyState`, chỉ tính khi cửa sổ game có focus); đến 2026-10-18 `reload.rs` gọi `eldenring::util::input::is_key_pressed` (`GetKeyState`, nhận phím cả khi cửa sổ game không ở nền) và đổi vì lỗi đó, đồng thời bản đó cộng với `common::diag` làm antivirus gắn cờ (RuneMultiplier 6 xuống 2 cờ trên VirusTotal, xem `mods/animcancel/docs/antivirus.md`). Code ở `shared/src/reload.rs`.**

## Vai trò

Theo dõi một khoá ini (`ReloadKey` theo quy ước) và nạp lại config khi bấm, độc
lập với mọi module tính năng, nên các tính năng của một mod không phải tự poll
phím này (và không cần đang chạy thì reload vẫn hoạt động).

`RELOAD_GENERATION` là bộ đếm nguyên tử tăng sau mỗi lần reload thành công, cho
các tính năng cần nhận ra **cạnh** reload (ví dụ để tính lại từ ảnh chụp giá trị
gốc thay vì cộng dồn). Đa số tính năng không cần: `common::config` đã là một bản
đồ dùng chung nên một lệnh `config::get_*` mỗi tick tự thấy giá trị mới.

## Chỉ một người poll mỗi phím

`common::input::is_key_pressed` giữ trạng thái theo mã phím (`KEY_STATE`) chứ không
theo người gọi: nếu hai module cùng gọi cho một phím, chỉ lần gọi đến trước trong
frame mới thấy `true` (lần gọi sau thấy phím vẫn đang giữ). Vì vậy `run` phải là nơi
**duy nhất** poll `ReloadKey` trong một process; các module khác theo dõi
`RELOAD_GENERATION`. Mỗi DLL có bản `KEY_STATE` riêng (mỗi DLL link riêng `common`).

## `run` và `run_with`

- `run(ini_path)`: config ini. Phím đọc từ khoá `ReloadKey` (mặc định `F5`),
  banner từ `ReloadBanner` (mặc định **tắt**).
- `run_with(reload_key, banner, reload)`: cho config không phải ini (TOML).
  `reload_key` đọc lại mỗi frame nên reload đổi phím có hiệu lực ngay; `banner`
  đọc sau khi reload nên reload tắt banner cũng có hiệu lực ngay.
- Khi `reload()` trả `Err` (lỗi cú pháp TOML, config cũ vẫn đang dùng): ghi log
  lỗi và hiện "Config error - see log" trong game, **luôn luôn** (không phụ thuộc
  `banner`), nếu không reload hỏng sẽ im lặng. `RELOAD_GENERATION` không tăng.

Chạy trên nhóm task `FrameBegin` của game, qua `task::run_recurring_safe` nên
panic không lan vào game. Gom từ `sometweaks::reload` và `risearcher::reload`
(hai bản y hệt) khi mod thứ ba cần cùng bộ theo dõi, và nay dùng `task` của crate
này thay vì bản riêng của từng mod.
