# AnimCancel

[English](README.md)

Cắt animation đang chạy bằng một hành động khác, không phải đợi nó kết thúc.

## Tóm tắt

Trong Elden Ring, một animation chỉ cho phép chuyển sang hành động khác ở "cửa sổ cancel" do chính
nó mở, thường ở nửa sau của đòn. AnimCancel mở cửa sổ đó sớm hơn, để guard, lăn, nhảy hay đổi vũ khí
có hiệu lực ngay khi bạn bấm.

Bốn hành động có thể cắt animation, mỗi hành động bật tắt độc lập:

| Hành động | Cắt animation bằng |
|---|---|
| **Guard** | đỡ đòn (L1) |
| **Dodge** | lăn, lùi |
| **Jump** | nhảy |
| **Weapon swap** | đổi vũ khí |

Với mỗi hành động, bạn chọn loại animation nào bị cắt:

| Loại animation | Ví dụ |
|---|---|
| **Attack** | đòn đánh thường |
| **Skill** | weapon art |
| **Cast** | sorcery, incantation |
| **Item** | uống flask, dùng item |

Guard chỉ cắt khi L1 thật sự là đỡ đòn, nên không làm đứt chuỗi đòn:

- **Có cắt:** cầm hai tay một vũ khí, hoặc một tay với khiên ở tay trái.
- **Không cắt:** cầm hai vũ khí (dual wield), Twin Blades, hoặc giữ guard từ trước khi đòn bắt đầu.

## Cài đặt

1. Cài một mod loader nạp được DLL (xem [README của repo](../../README.vi.md)).
2. Đặt `AnimCancel.dll` vào thư mục mod của loader và khai báo nó.
3. Chạy game. `AnimCancel.toml` được tạo cạnh DLL ở lần chạy đầu.

## Cấu hình

Cấu hình nằm trong `AnimCancel.toml` cạnh DLL, đặt tên theo DLL. Tên key, giá trị mặc định và ý nghĩa
nằm ở chú thích trong [AnimCancel.toml](AnimCancel.toml), file mẫu nhúng sẵn trong DLL. Sửa file rồi
bấm phím nạp lại (mặc định `F5`) để áp dụng, không cần khởi động lại game. File cấu hình của bản 1.2.0
(`AnimCancelV1.toml`) được chép sang tên mới ở lần chạy đầu.

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục **Anim Cancel**
xuất hiện trong System > Game Options: một trang cho mỗi hành động và công tắc ghi log. Thay đổi được
ghi vào file cấu hình, giữ nguyên comment. Không có ERNativeUI thì mod chạy bình thường từ file.

## Cách hoạt động

Mỗi frame, nếu animation đang chạy thuộc nhóm được bật, mod mở sẵn quyền cancel của nó trong hàng đợi
hành động của `CSChrActionRequestModule`, đúng chỗ script hành vi của game đọc, thay vì đợi animation
tự mở. Không cần mẫu byte hay offset: chỉ dùng struct của
[fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs). Cơ chế của game và kết quả các lần thử:
[docs/animation_cancel.md](docs/animation_cancel.md).

## Tương thích và giới hạn

- Chỉ áp dụng cho người chơi local; chưa kiểm tra với Seamless Co-op.
- Đỡ đòn khi đang ở trên không không làm được chỉ bằng DLL, xem [docs/air_guard.md](docs/air_guard.md).

## Tài liệu

- [CHANGELOG.md](CHANGELOG.md): ghi chú phát hành.
- [docs/animation_cancel.md](docs/animation_cancel.md): cách cancel hoạt động và các lần thử.
- [docs/air_guard.md](docs/air_guard.md): vì sao bỏ đỡ đòn trên không.
- [docs/antivirus.md](docs/antivirus.md): cờ antivirus và những gì đã thay đổi chúng.
