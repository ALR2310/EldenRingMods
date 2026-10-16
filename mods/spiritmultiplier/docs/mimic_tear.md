# Mimic Tear: Blob và số lượng

**Status 2026-10-16: sửa chế độ `Amount` (test trong game: 10 ra đủ 10 Mimic). Crash khi gọi nhiều Mimic Tear (Seamless) là lỗi của game/GPU, xem cuối file.**

## Hiện tượng

Nexus (LordNick9911, 2026-10-16) và test của người dùng: `Amount=10` chỉ ra 5 Mimic Tear. Log (`slots changed`): 10 spirit xen kẽ NPC `133201000` (`[Spirit Summon] Mimic Tear - Blob`) và `100000010` (`[Spirit Summon] Mimic Tear`); khoảng 7 giây sau 5 con Blob biến mất, còn 5 Mimic.

## Nguyên nhân

Chuỗi SpEffect 207010 -> BuddyParam của Mimic Tear có 2 phần tử: Blob (xuất hiện tạm rồi mất) và Mimic thật. `Amount` đếm phần tử chuỗi nên 10 phần tử = 5 Blob + 5 Mimic. `Multiplier` nhân độ dài chuỗi gốc (2) nên đúng sẵn: nhân 5 ra 5 Blob + 5 Mimic.

## Cách sửa (`src/chain.rs`)

`TRANSIENT_NPCS` (hiện chỉ `133201000`) -> tập BuddyParam ID có NPC đó, dựng 1 lần từ `BuddyParam` qua `common::params`. Chế độ `Amount`: số phần tử = `Amount x độ dài chuỗi gốc / số phần tử thật` (làm tròn lên). Mimic Tear: 10 -> 20 phần tử (10 Blob + 10 Mimic). Chuỗi không có phần tử tạm không đổi. Dữ liệu `NpcParam` export chỉ có đúng 1 dòng "Blob".

## Crash khi gọi nhiều Mimic Tear (kết luận 2026-10-16: lỗi của game/GPU, không phải mod)

Báo cáo Nexus (LordNick9911) và tái hiện: `Multiplier=100` (chuỗi bị cắt ở `MaxSpirits=60` = 30 Blob + 30 Mimic) trong Seamless Co-op thì game thoát khoảng 7 giây sau khi gọi, đúng lúc Blob bắt đầu biến mất. Không có dump WER, không có sự kiện `Application Error`; `common::crash` (vectored handler) ghi được:

- **Có Seamless** (2/2 lần, cùng chữ ký): `access violation, read 0x0, at nvwgf2umx.dll+0x7E3868`, stack `D3D12Core.dll+0x22A38D < 0x2395D4 < eldenring.exe+0x19E4344 < 0x19EC3ED < 0x1C4AC70 < 0x1C4A793`. `eldenring.exe+0x19EC310` là hàm đặt render target (lặp qua mảng con trỏ tài nguyên đồ hoạ) gọi D3D12; con trỏ tới tài nguyên đã hỏng/NULL.
- **Không Seamless**: cùng cấu hình chạy ổn khi Blob biến mất (30 Mimic còn lại); nhưng về sau gặp một crash khác: `write 0x0` trong `memmove` (`eldenring.exe+0x251E7FB`, vòng sao chép theo bước `sub_1411B1700`).

Giả thuyết: mỗi Mimic sao chép model và trang bị người chơi nên tốn nhiều tài nguyên đồ hoạ/bộ nhớ; cấp phát thất bại bị engine bỏ qua rồi truy cập NULL (cùng kiểu crash render 2026-10-04). Chưa đo VRAM/RAM. `Amount=10` ổn nhiều lần, `Amount=30` có sửa ổn một lần.

Kết luận: gọi Mimic Tear số lượng lớn (khoảng 30 con) với Seamless có thể làm game thoát; không sửa được ở mod. Mỗi Mimic chiếm 2 slot lúc mới ra nên `MaxSpirits=60` chỉ ra tối đa 30 Mimic thật.

## Các báo cáo khác của cùng người dùng (không tái hiện được trên 1.2.2)

- "Không gọi lại được sau khi cho về": thực ra triệu hồi lại **trước khi** game dọn xong spirit cũ (60 spirit mất khoảng 5-6 giây) thì lần gọi đó bị mất; chờ vài giây là gọi lại bình thường.
- "`Amount`/`Multiplier` không vượt số lượng gốc": không tái hiện; nghi sửa ini mà không bấm `ReloadKey`, hoặc không khởi động lại sau khi đổi `MaxSpirits`.
