# TODO

Chỉ liệt kê việc **còn mở**, mỗi việc 1 dòng. Việc xong xoá đi (lịch sử ở
`mods/<mod>/HISTORY.md`); lý do / chi tiết kỹ thuật ở `mods/<mod>/docs/`.
Quy ước đầy đủ: `CLAUDE.md`, mục "Viết `TODO.md` gọn".

## Menu trong game (ERNativeUI)

Chi tiết: `docs/ingame_menu.md`

- [ ] Đưa menu tùy chọn lên các mod khác (ứng viên chọn 2026-10-13); mỗi mod 1 hàng trang con, tên chứa tên mod
- [ ] [research] Slider số âm/thập phân: thử bản fork ERNativeUI + sửa 1 byte game (`docs/ingame_menu_slider.md`)

## InfiniteAilment

- [ ] [research] Thêm Black Flame (DoT theo % máu, giống Poison/Rot) làm ailment chỉnh được; Destined Death (giảm máu tối đa) khó hơn (long41, 2026-10-13)

## SpiritMultiplier

Chi tiết: `mods/spiritmultiplier/docs/open_issues.md`, `docs/warp_research.md`

- [ ] [research] Convergence: triệu hồi thêm Ash sau Cemetery Shade làm mọi spirit vô hình (người dùng, 2026-10-10); xem SpEffect 295921 trong Smithbox, `docs/open_issues.md`
- [ ] Reforged: spirit gọi ra mặc định ở trạng thái đỏ (Fury) thay vì trắng
- [ ] Reforged + MultiSpirit: kích Fury riêng cho từng Ash (hiện Fury 1 nhóm thì mọi Ash đều xám)
- [ ] Reforged: NoRestResummon không có tác dụng sau Spirit-Severing Blade (nghi luật Reforged, không phải mod)
- [ ] Seamless Co-op: spirit mất màu ma (tạm dừng 2026-10-02)
- [ ] Spirit dịch chuyển tới người chơi khi bị bỏ lại (hoãn)
- [ ] Cho triệu hồi cùng 1 Ash nhiều lần, tối đa X con, quá X thì con cũ nhất biến mất (almasakmal123, 2026-10-08)
- [ ] [research] Spirit cỡ lớn (mod Elden Lord Army of Summons) đứng im đến khi địch lại gần người chơi; muốn chủ động tấn công (almasakmal123, 2026-10-08); thử bằng `SpiritThinkParam.ini`, xem `mods/spiritmultiplier/docs/think_override.md`
- [ ] Spirit không bị sát thương khi rơi từ chỗ cao (almasakmal123, 2026-10-08); phần đưa về chỗ người chơi khi rơi khỏi map gộp vào warp hoãn ở trên

## SpeedMultiplier

- [ ] Seamless Co-op: đồng bộ hình ảnh tốc độ giữa các máy bằng Steam P2P (hướng B, chốt 2026-10-03; xem `docs/seamless.md`)
- [ ] Test `Critical` khác 1 với nạn nhân backstab/riposte (`docs/critical_victim.md`)
- [ ] Hệ số tốc độ đánh theo loại vũ khí (InvertedButt, 2026-10-01; gộp cả cung/nỏ riêng, tạm không làm)
- [ ] Tách hệ số tốc độ kẻ địch thường / boss (Nickademus84, 2026-10-13; tách theo nhóm hành động như player thì chưa khả thi)
- [ ] Tuỳ chọn: Seamless cho Spirit / Enemy / nạn nhân critical; chia nhóm boss / spirit

## DropMultiplier

- [ ] Nhân số lượng đồ hái trên map (Erdleaf Flower, Trina's Lily...; `ItemLotParam_map`, LordSoulOfNito, 2026-10-02)

## RuneMultiplier

- [ ] Test trong game bản có menu (ERNativeUI): slider áp ngay, ini ghi lại sau khi dừng, F5 đồng bộ, công tắc Log file; rồi phát hành bullet ở `[Unreleased]` (docs/ingame_menu.md)
- [ ] Test trong game reload qua `common::reload` + banner (2026-10-07), rồi phát hành bullet ở `[Unreleased]`
- [ ] Nghiên cứu hệ số rune riêng theo nguồn (giết địch / bán đồ...; julianpratt, 2026-08-31)

## AnimCancel

- [ ] Trả lời DAM127 (Nexus, 2026-10-12): đã có bản chạy (đòn đánh, skill, phép huỷ bằng guard/lăn/đổi vũ khí); hỏi muốn cancel gì thêm `[chờ phản hồi]`
- [ ] Kiểm tra tác dụng phụ (i-frame, hitbox, FP khi cắt phép) và Seamless Co-op; xem `mods/animcancel/docs/animation_cancel.md`

## Chung

- [ ] `ReloadKey` lọt sang instance game khác: `common::reload` và autoregen nên dùng `common::input::is_key_pressed` (xem `mods/soulsteleport/docs/position_exchange.md`)
- [ ] `risearcher` (`weapon.rs`, `bullet.rs`), `infiniteailment` (`status_effect.rs`): chuyển sang `common::params::for_each_row_mut` (xem `mods/dropmultiplier/docs/convergence_panic.md`)
- [ ] `ReloadBanner` cho `weightmultiplier` - cần quyết định (phải bỏ thiết kế không dùng task game)
- [ ] Dọn tham chiếu `.docs/...` lỗi thời (~47 dòng / 15 file)
- [ ] `mods/risearcher/project.json` chứa đường dẫn máy: git-ignore hoặc đổi thành file mẫu
