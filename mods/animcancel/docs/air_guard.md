# Đỡ đòn trên không (guard khi đang nhảy): đã thử, dừng

**Status 2026-10-15: ĐÃ DỪNG, không làm bằng DLL.** Thử ngày 2026-10-14, thấy nhiều vấn đề nên bỏ; người dùng Nexus (OKLICKMAN, 2026-10-15) cũng hỏi tính năng này ("air blocking like Sekiro") và đã được trả lời là không làm được chỉ bằng DLL.

## Kết luận

Game không có trạng thái đỡ đòn khi đang trên không. Ép hành động guard giữa không trung thì game kéo nhân vật xuống đất và reset trạng thái liên tục, không ổn định. Làm cho đúng cần animation mới và sửa file hành vi (HKS/TAE), ngoài khả năng của một DLL.

## Đã thử

- Thêm nhóm animation `Jump` vào `[Animations]` (`Jump = true`, ghi chú "EXPERIMENTAL: only Guard works in the air"): khi đang nhảy chỉ mở bit Guard, không mở lăn và đổi vũ khí (`cancel.rs`, `allow_from` với `dodge`/`weapon_swap` = false).
- `EventProbe` (`src/eventprobe.rs`, 200 dòng): log các sự kiện hành vi (behavior event) phát ra khi giữ L1 lúc ở trên không, kèm `standing_on_solid_ground`.

## Còn lại

Code thử nằm trong stash git `stash@{0}` ("!!GitHub_Desktop<main>", 6 file trong `mods/animcancel/`), chưa đưa vào nhánh nào. Không áp dụng lại trừ khi có hướng mới (ví dụ sửa hành vi bằng file ngoài DLL).
