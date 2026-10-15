# TODO

Chỉ liệt kê việc **còn mở**, mỗi việc 1 dòng. Việc xong xoá đi (lịch sử ở
`mods/<mod>/HISTORY.md`); lý do / chi tiết kỹ thuật ở `mods/<mod>/docs/`.
Quy ước đầy đủ: `CLAUDE.md`, mục "Viết `TODO.md` gọn".

## Menu trong game (ERNativeUI)

Chi tiết: `docs/ingame_menu.md`

- [ ] [research] Slider số âm/thập phân: hook `0x74FA20` ánh xạ `min + chỉ_số x step` (tiền lệ `mod-settings-menu.dll`), thử bản fork ERNativeUI (`docs/ingame_menu_slider.md`)
- [ ] [research] Menu cuộn / vượt giới hạn 15 hàng của trang con (`docs/ingame_menu_scroll.md`)
- [ ] Phát hành lần lượt các mod đã có menu (bullet có sẵn ở `[Unreleased]`); ghi trên trang Nexus rằng ERNativeUI là phụ thuộc tùy chọn (exe 2.7.0.0 / 2.7.1.0)

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

- [ ] Test trong game reload qua `common::reload` + banner (2026-10-07), rồi phát hành bullet ở `[Unreleased]`
- [ ] Nghiên cứu hệ số rune riêng theo nguồn (giết địch / bán đồ...; julianpratt, 2026-08-31)

## AnimCancel

- [ ] Trả lời DAM127 (Nexus, 2026-10-12): bản 1.0.0 đã phát hành (đòn đánh, skill, phép, item huỷ bằng guard/lăn/đổi vũ khí); hỏi muốn cancel gì thêm `[chờ phản hồi]`
- [ ] Kiểm tra tác dụng phụ (i-frame, hitbox, FP khi cắt phép) và Seamless Co-op; xem `mods/animcancel/docs/animation_cancel.md`

## Chung

- [ ] `ReloadKey` lọt sang instance game khác: `common::reload` (`reload.rs`) và autoregen (`regen.rs`) còn dùng `eldenring::util::input`, đổi sang `common::input::is_key_pressed` (xem `mods/soulsteleport/docs/position_exchange.md`, `shared/docs/input.md`)
- [ ] `risearcher` (`weapon.rs`, `bullet.rs`), `infiniteailment` (`status_effect.rs`): chuyển sang `common::params::for_each_row_mut` (xem `mods/dropmultiplier/docs/convergence_panic.md`)
- [ ] `ReloadBanner` cho `weightmultiplier` - cần quyết định (phải bỏ thiết kế không dùng task game)
- [ ] Dọn tham chiếu `.docs/...` lỗi thời (~17 dòng / 13 file, phần lớn ở `sometweaks` đã bỏ; còn `autoregen/hit_hook.rs`, `spiritmultiplier/chain.rs`, `probe.rs`)
- [ ] `mods/risearcher/project.json` chứa đường dẫn máy: git-ignore hoặc đổi thành file mẫu

## Đã dừng

- AnimCancel: đỡ đòn trên không (như Sekiro) - không làm được chỉ bằng DLL, xem [mods/animcancel/docs/air_guard.md](mods/animcancel/docs/air_guard.md) (OKLICKMAN, 2026-10-15)
