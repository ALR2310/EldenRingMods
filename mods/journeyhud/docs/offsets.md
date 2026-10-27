# JourneyHUD: offset và kiểm chứng theo bản exe

**Status 2026-10-26: đã kiểm trong IDA trên 2.6.2.0, 2.7.0.0, 2.7.1.0, 2.7.2.0 (tĩnh). Chưa kiểm trong game; +0x14 của poise chưa xác nhận được bằng IDA.**

Panel đọc qua struct của `fromsoftware-rs` (rev của workspace). Không dùng `rva::get()`. Các offset dưới đây
được ép bằng `const` assert trong `src/target.rs`: một bản `fromsoftware-rs` dời field thì build dừng.

| Dữ liệu | Offset | Kiểm trong IDA |
|---|---|---|
| Handle mục tiêu khoá (`PlayerIns::locked_on_enemy`) | +0x6B0 | **Có, cả 4 bản.** |
| Danh sách module (`ChrIns::modules`) | +0x190 | Dùng sẵn ở mod đã phát hành (AutoRegen, SpiritMultiplier). |
| HP / max HP (`CSChrDataModule`) | +0x138 / +0x13C | Dùng sẵn ở AutoRegen (đã phát hành). |
| Container: `data` / `super_armor` | +0x00 / +0x40 | Theo `fromsoftware-rs`, chưa kiểm riêng. |
| Poise hiện tại (`CSChrSuperArmorModule`) | +0x10 | **Có** (hàm cập nhật dùng `a1+16` làm giá trị hiện tại). |
| Poise tối đa | +0x14 | **Chưa.** Xem bên dưới. |
| Hàm tra nhân vật theo handle | slot 3 của vtable ChrSet | **Có, cả 4 bản.** |
| Thanh máu boss (`CSFeManImp::boss_health_displays`) | +0x5BF0, 3 ô x 0x20, handle ở +8 (+0x5BF8), ô trống = -1 | **Có, cả 4 bản** (xem bên dưới). |
| Giờ trong game (`WorldAreaTime::clock`) | +0x00 (`time64`), +0x08 giờ ở bit 34-38 của số đóng gói | **Có, cả 4 bản** (xem bên dưới). |

## Thanh máu boss (`Minimap.Hide.InBossFights`)

Hàm nhỏ tìm một handle trong 3 ô (RVA 76DA70 / 76E8F0 / 76E8F0 / 76EDA0 trên 2.6.2.0 / 2.7.0.0 / 2.7.1.0 / 2.7.2.0, cùng
dạng cả 4 bản): `for (i = a1 + 23544; ...; i += 8 dword) { ... if (++n >= 3) return -1 }`. `23544` là 0x5BF8, tức handle của ô
đầu (ô bắt đầu ở 0x5BF0, `fmg_id` +0, handle +8), bước 32 byte, đúng 3 ô; với handle cần tìm là `0xFFFFFFFF` nó so dword thấp
với `0xFFFFFFFF`, nghĩa là **ô trống = handle -1**, khớp `FieldInsHandle::is_empty`. Chưa thử khi hạ boss để xem ô có
được xoá ngay không (TODO `[verify]`).

## Giờ trong game (`Minimap.Clock`)

Singleton `WorldAreaTime` đăng ký theo tên ở `sub_14009CF30` (cùng RVA cả 4 bản) nên `WorldAreaTime::instance()` tìm theo tên,
không dùng bảng RVA. Hàm `CS::WorldAreaTimeImp::Update` (643BC0 / 644A10 / 644A10 / 644EC0) chép `*a1` và `*(a1+8)` sang
`+16` và `+24` (đồng hồ hiện tại sang đồng hồ tick trước), kiểm giờ-phút-giây mục tiêu ở `+40/+44/+48` (<= 23, 59, 59) và lấy
giờ bằng `(*(a1+8) >> 34) & 0x1F`, đúng với `PackedDate::hours` (bit 34-38) của `fromsoftware-rs`. Phút nằm ngay sau (bit 39-44).

## Handle mục tiêu (+0x6B0)

Trên cả 4 bản exe, `fieldaccess 6b0` cho cùng bộ truy cập: một lần ghi `-1` (qword) và một lần ghi
handle (`mov [r13+6B0h], rcx`) trong cùng một hàm, cộng với một hàm đọc ở `0x344DA0 / 0x344DB0`.

| Bản | Hàm ghi (RVA) |
|---|---|
| 2.6.2.0 | 716260 |
| 2.7.0.0, 2.7.1.0 | 7170B0 |
| 2.7.2.0 | 717560 |

Decompile ở 2.7.2.0: hàm cập nhật camera khoá mục tiêu. Với nhân vật người chơi (`v8`) nó ghi `-1` vào
`+1712` (= 0x6B0) khi bỏ khoá và ghi `*(chr_mục_tiêu + 8)`, tức **`field_ins_handle` của mục tiêu** (ChrIns +0x8),
khi đang khoá. Vậy `locked_on_enemy` là đúng handle cần, không cần ghép theo vị trí.

## Tra nhân vật theo handle (slot 3)

- `CS::ChrSet` slot 3 là hàm tra theo handle: đổi handle sang chỉ số rồi gọi slot 1 (`vtable+8`). Cùng dạng ở 2.6.2.0 (4937A0), 2.7.1.0 (493D00), 2.7.2.0 (493DE0).
- `CS::OpenFieldChrSet` ghi đè slot 3 bằng tìm nhị phân trong bảng handle (2.7.1.0 `4EC850`, 2.7.2.0 `4EC930`, 2.6.2.0 `4EBA80`), cùng chữ ký.
- Gọi qua vtable của chính đối tượng nên đúng cho cả hai lớp.

## Poise (chưa kiểm hết)

Hàm của game dùng `+0x10` làm giá trị hiện tại (float), `+0x18` là phần cộng thêm vào mức tối đa và `+0x1C` là
bộ đếm hồi. Mức tối đa hiệu dụng là `vtable[2]() + [+0x18]` (ở lớp cơ sở `vtable[2]` trả 0.0). Tôi chưa tìm
được chỗ nào trong IDA ghi `+0x14`; `souls_vision` và `fromsoftware-rs` đều đặt "max" ở đó nên có thể lớp con
đặt giá trị này. Cách xử lý: panel chỉ hiện hàng Poise khi max > 0 và kẹp tỉ lệ 0..1; log
`Target: ... poise (cur, max)` khi thử trong game sẽ cho biết giá trị thật, nếu sai thì đổi cách tính max.

## Việc còn lại

- Thử trong game, so log poise với thanh stagger thật (đánh trúng, chờ hồi).
- Nếu max ở +0x14 không đúng, tìm lớp con thật của CSChrSuperArmorModule.
