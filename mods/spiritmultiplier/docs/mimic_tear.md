# Mimic Tear: Blob và số lượng

**Status 2026-10-22: `InstantMimic` hoạt động (test trong game 2.7.2.0: Mimic ra ngay, đi theo, đánh, có thanh HP trên HUD). Gồm ghi đè dòng Blob + cờ + animation 1830, và stub HUD; xem các mục cuối file. Fade-in giữ nguyên.**

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

## `InstantMimic`: bỏ Blob và fade-in (2026-10-22, chưa test trong game)

Port từ `er10x.dll` ("10x Spirit Summons", `tmp/erx10/`): các chuỗi `drop_mimic_blob`, `remove_mimic_fade`, `mimic_blob_row=20700000`, `mimic_npc_row=100000010`, `mimic_fade_value=16316`; log của nó: "row was the blob stage; rewritten with row ... so both links are the real mimic. The row id stays, which is what makes the game copy your build". Đã giải mã er10x (171 hàm, `dumps/er10x/export.c`, tạo bằng `scripts/ida/export_all.py` chạy trực tiếp bằng idat): xem mục "er10x làm gì" cuối file.

Cách làm (`src/mimic.rs`, `src/chain.rs`):

- Nhóm `BuddyParam` theo `triggerSpEffectId`; dòng có NPC Blob (`TRANSIENT_NPCS`) nhận các trường spawn của dòng không-Blob cùng nhóm (NPC/think, ridden, offset, doping lv0-10, init, `generateAnimId`). **Giữ nguyên ID dòng**: game dùng ID đó để nhận ra "đây là Mimic Tear" và copy build.
- Fade-in **giữ nguyên** (người dùng chọn, 2026-10-22): `NpcParam` Mimic slot `spEffectID24` = 16316 "Spirit Summon - Fade In" chỉ là hiệu ứng hình (vfx 53202, sống 3 s, không có sát thương/buff); er10x xoá nó (-1). Test lần đầu: Mimic vẫn trồi lên, nghi do `generateAnimId` chứ không phải fade-in.
- Chuỗi: Blob thành Mimic nên 1 link = 1 Mimic; chuỗi `[Blob, Mimic]` rút còn 1 link (`Amount=10` ra 10 Mimic, `Multiplier=5` ra 5). Chỉ rút khi Blob đứng đầu chuỗi (đầu chuỗi là nút của game, mod không ghi), và chỉ sau khi dòng Blob đã được sửa (`mimic::blob_replaced`).
- Mọi thứ chụp một lần rồi áp từ bản chụp nên F5 bật/tắt hai chiều.

**Kết quả test 2026-10-22 (log + quan sát của người dùng):** tính năng chưa đạt mục tiêu "ra tức thì". Thử 2 (blob nhận animation spirit thường 1830 và mọi link chuỗi là dòng Blob): 2 slot NPC 100000010, một con đứng yên mãi, một con trồi lên rồi đi theo, thanh HP chỉ 1. Kết luận tạm: game coi thực thể sinh từ dòng Blob là chỗ giữ chỗ (vanilla nó là NPC Blob tự biến mất sau ~8 s), nên NPC Mimic gắn vào dòng Blob đứng yên; hiệu ứng trồi lên không nằm ở `generateAnimId` (Mimic 60500, spirit thường 1830, Blob 1831). Đã hoàn tác thử 2 (chuỗi `[Blob, Mimic]`, Blob nhận trường Mimic nguyên bản). Muốn ra tức thì thật cần decompile `er10x.dll` xem nó làm gì khác.

Cần test: Mimic ra ngay với vũ khí/giáp/talisman đã copy; nếu build không copy (game cần cả link thứ 2 trong chuỗi) thì đổi sang giữ chuỗi 2 link như er10x (khi đó 1 lần gọi ra 2 Mimic).

## er10x làm gì (giải mã 2026-10-22, `dumps/er10x/export.c`)

Trên mỗi dòng `BuddyParam` (duyệt theo chỉ số):

- Dòng có `generateAnimId` (dword 22, +0x58) = 1831 và ID = `mimic_blob_row` (20700000) là Blob. Nếu `drop_mimic_blob`: **chép nguyên 156 byte của dòng kế tiếp (chỉ số +1, phải cùng `triggerSpEffectId`) đè lên dòng Blob**, trừ dword +0x9C giữ lại. ID dòng không đổi.
- Bước `fix_fields` (mặc định bật), áp cho **mọi** dòng: `appearOnAroundSekihi` = 0, `disablePCTargetShare` = 0, `pcFollowType` = 0, và `generateAnimId` > 0 khác 1830 thì đặt 1830 (kể cả Mimic: 60500 -> 1830). Đây là cái làm Mimic hiện tức thì; log của nó gọi là "sekihi/targetShare/followType/appearAnim".
- Fade: `NpcParam` dòng 100000010, trường chứa 16316 đặt -1 (bản mod này giữ fade theo ý người dùng).
- Chuỗi: dựng lại bằng vòng lặp xoay vòng ID gốc, đầu chuỗi giữ nguyên, số entry thật = số entry không phải Blob (tối thiểu 1); giống `chain.rs`.

Vì sao thử 2 của mod này hỏng (một Mimic đứng yên mãi, một Mimic chạy): `Spawn` chỉ chép NPC/think/offset/doping/anim, **thiếu 3 byte cờ** (+36..38). `pcFollowType` của dòng Blob là kiểu không đi theo người chơi, nên bản sao Mimic trên dòng Blob đứng yên. Bản hiện tại chép thêm 3 byte cờ từ dòng Mimic, và đặt animation phổ biến nhất của dòng không-Blob (1830) cho cả dòng Blob lẫn dòng Mimic thật (chuỗi `[Blob, Mimic]` vẫn xoay vòng cả hai).

## Thanh HP trên HUD (2026-10-22, giải mã eldenring.exe 2.7.2.0)

Quan sát: với `InstantMimic` (Blob đã thành Mimic), 2 Mimic cùng chạy/đánh nhưng HUD chỉ hiện 1 thanh HP; `Multiplier=1` (1 Mimic, từ dòng Blob) thì **không có thanh nào**. Param không giải thích được: sau khi sao chép, dòng `20700000` chỉ khác `20700001` ở ID (CSV BuddyParam/NpcParam đã đối chiếu).

Nguyên nhân (code game, 2.7.2.0): hàm dựng danh sách thanh HP của HUD buddy `sub_140771ED0` (RVA 0x771ED0) duyệt các chr đã triệu hồi và **bỏ qua** chr nào mà `sub_1404B7A10` (RVA 0x4B7A10, "chr này là Blob của Mimic": nhóm của chr có entry với BuddyParam id **20700000**, cờ chr +457 bit 4) trả 1; tối đa 5 thanh. ID `20700000` còn được `sub_1404BBEB0` (hàm sinh buddy) dùng để bật cờ "yêu cầu Mimic Tear". Chỉ có 2 chỗ trong exe chứa hằng này.

Cách sửa (`src/mimic.rs`): đổi đích `call sub_1404B7A10` trong `sub_140771ED0` sang stub trả `al = 0` khi `InstantMimic` đang áp (mọi chr từ dòng Blob giờ là Mimic thật nên đáng có thanh), còn không thì nhảy sang hàm gốc. AOB `84 C0 74 ?? 48 8B D6 49 8B CE E8 ?? ?? ?? ?? 84 C0 75 ?? 83 FF 05`, `call` ở +10, unique trên 2.6.2/2.7.0/2.7.1/2.7.2. Chưa test trong game.
