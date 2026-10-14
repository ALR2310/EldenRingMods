# logger: file log gated bằng `LogFile`

**Status 2026-10-13: hoạt động, mọi mod dùng. Code ở `shared/src/logger.rs`, có test cho việc bật/tắt và tạo file.**

## Quy tắc bật/tắt (2026-09-23)

Khoá `LogFile` trong ini quyết định tất cả: `LogFile=false` thì không tạo cũng
không ghi file nào; `LogFile=true` thì file được tạo (cắt bỏ log của lần chạy
trước) ở dòng đầu tiên cần ghi. Khoá được kiểm tra ở **mỗi lần ghi**, nên đổi
khoá rồi bấm `ReloadKey` có hiệu lực ngay: bật giữa chừng thì bắt đầu ghi lúc
đó, tắt thì ngừng ghi.

Trước đó mọi mod trừ RiseArcher/RuneMultiplier gọi `init` vô điều kiện nên file
luôn được tạo, còn `LogFile` chỉ chặn các dòng chi tiết của riêng mod: không
đúng với tên của khoá. Mod muốn có log ngay từ đầu thì để mặc định
`LogFile=true` trong ini.

Mod dùng `toml_config` không có khoá ini để đọc: nó gọi `set_enabled` với giá trị
`LogFile` của mình (sau khi nạp và sau mỗi lần reload), và từ đó giá trị này ghi
đè việc tra ini.

## Định dạng và mức log

`[thời gian] [MỨC] nội dung`. Mức được ghi nguyên văn, **không** đệm cho đủ 5 ký
tự: bản đầu dùng `[INFO ]` để thẳng cột nhưng khoảng trắng trong ngoặc bị người
dùng đọc thành lỗi gõ (2026-09-26).

| Hàm | Dùng khi |
|---|---|
| `log` | thông tin thường (INFO); tên `log` giữ nguyên để không phải sửa mọi nơi gọi |
| `warn` | bất ngờ nhưng mod đã phục hồi hoặc còn đang thử lại |
| `error` | một tính năng hỏng và tắt hẳn đến hết phiên; đừng lặp chữ "ERROR" trong nội dung |
| `debug` | chi tiết khi chẩn đoán một tính năng; nơi gọi tự đặt sau cờ ini của riêng nó |

Log được người dùng đăng công khai, xem `shared/docs/diag.md` về quyền riêng tư.

## Mở file ở chế độ append

File được cắt về 0 byte bằng một lần mở riêng, rồi giữ handle ở chế độ **append**.
Append xác định lại cuối file thật ở mỗi lần ghi, nên nếu có thứ khác (người
dùng xoá nội dung file trong trình soạn thảo) đổi kích thước file khi handle còn
mở, dòng tiếp theo vẫn rơi đúng cuối file, thay vì vị trí ghi cũ đã lưu trong
handle (với handle ghi thường, vùng trống sẽ bị lấp bằng byte 0).

## Panic hook

`install_panic_hook` ghi một dòng ERROR (tên thread, vị trí, nội dung) trước khi
chạy hook cũ. Workspace dùng `panic = "unwind"`, nên một panic ở thread nền (như
`std::thread::spawn` trong `DllMain`) làm thread chết im lặng và log dừng đột ngột,
không có dòng lỗi nào. Đã từng gặp khi offset struct của `fromsoftware-rs` lệch
sau bản cập nhật game (các `.expect()` của `SoloParamRepository`).

Khoá của log không bao giờ gây panic: nếu khoá bị poison do một panic trước đó
thì vẫn dùng tiếp, vì hook panic cũng gọi vào logger.
