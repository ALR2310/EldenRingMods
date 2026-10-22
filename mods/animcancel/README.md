<div align="center">

# AnimCancel

**Cắt animation đang chạy bằng một hành động khác, không phải đợi nó kết thúc.**

![Rust](https://img.shields.io/badge/Rust-cdylib-orange?logo=rust)
![Platform](https://img.shields.io/badge/platform-Windows_x64-blue)
![Game](https://img.shields.io/badge/game-Elden_Ring-8a6d3b)

</div>

---

## Giới thiệu

Trong Elden Ring, một animation chỉ cho phép chuyển sang hành động khác ở "cửa sổ
cancel" do chính nó mở, thường ở nửa sau của đòn. AnimCancel mở cửa sổ đó sớm hơn,
để guard, lăn, nhảy hay đổi vũ khí có hiệu lực ngay khi bạn bấm, không phải chờ hết
animation.

## Tính năng

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
- **Không cắt:** cầm hai vũ khí (dual wield), Twin Blades, hoặc giữ guard từ trước
  khi đòn bắt đầu.

## Cấu hình

Cấu hình nằm trong `AnimCancelV1.toml` cạnh DLL, tự tạo ở lần chạy đầu. Tên key,
giá trị mặc định và ý nghĩa nằm ở chú thích trong [AnimCancelV1.toml](AnimCancelV1.toml),
file mẫu nhúng sẵn trong DLL. Sửa file rồi bấm phím reload để áp dụng, không cần
khởi động lại game.

Tên file có chữ `V1` để tránh một chuỗi bị antivirus gắn cờ nhầm, xem
[docs/antivirus.md](docs/antivirus.md).

### Menu trong game (tùy chọn)

Nếu [ERNativeUI](https://github.com/Flammrock/ERNativeUI) được nạp trước mod, mục
**Anim Cancel** xuất hiện trong System > Game Options: một trang cho mỗi hành động
và công tắc ghi log. Thay đổi trong menu được ghi vào file cấu hình (giữ nguyên
comment). Không có ERNativeUI thì mod chạy bằng file như bình thường.

## Cách hoạt động

```
animation đang chạy ──► thuộc nhóm được bật? ──► ghi quyền cancel vào hàng đợi
                                                  của CSChrActionRequestModule
                                                  (đúng chỗ HKS đọc)
```

Mỗi frame, nếu animation đang chạy thuộc nhóm được bật, mod mở sẵn quyền cancel của
nó thay vì đợi animation tự mở. Không cần AOB hay RVA: chỉ dùng struct của
[fromsoftware-rs](https://github.com/vswarte/fromsoftware-rs) và crate dùng chung
`common`. Cơ chế của game và kết quả các lần thử:
[docs/animation_cancel.md](docs/animation_cancel.md).

## Giới hạn

- Chỉ áp dụng cho người chơi local; chưa xét tương thích với Seamless Co-op.
- Đỡ đòn khi đang ở trên không không làm được chỉ bằng DLL, xem
  [docs/air_guard.md](docs/air_guard.md).

## Tài liệu

| Tài liệu | Nội dung |
|---|---|
| [docs/animation_cancel.md](docs/animation_cancel.md) | Cơ chế cancel của game, vì sao ghi đúng chỗ đó, kết quả các lần thử |
| [docs/air_guard.md](docs/air_guard.md) | Thử đỡ đòn trên không và vì sao dừng |
| [docs/antivirus.md](docs/antivirus.md) | Cờ antivirus và việc đổi tên file cấu hình |
| [HISTORY.md](HISTORY.md) | Dòng thời gian thay đổi |
| [CHANGELOG.md](CHANGELOG.md) | Ghi chú phát hành cho người dùng |

Mã nguồn: [src/cancel.rs](src/cancel.rs) (logic cắt animation),
[src/weapon.rs](src/weapon.rs) (luật guard theo vũ khí), [src/config.rs](src/config.rs),
[src/menu.rs](src/menu.rs). Phân loại animation dùng `common::anim`.
