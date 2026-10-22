# TODO

Chỉ liệt kê việc **còn mở**, mỗi việc 1 dòng. Việc xong xoá đi (lịch sử ở
`mods/<mod>/HISTORY.md`); lý do / chi tiết kỹ thuật ở `mods/<mod>/docs/`.
Quy ước đầy đủ: `CLAUDE.md`, mục "Viết `TODO.md` gọn".

## Menu trong game (ERNativeUI)

Chi tiết: `docs/ingame_menu.md`

- [ ] [research] Menu cuộn / vượt giới hạn 15 hàng của trang con (`docs/ingame_menu_scroll.md`)
- [ ] Phát hành lần lượt các mod đã có menu (bullet có sẵn ở `[Unreleased]`); ghi trên trang Nexus rằng ERNativeUI là phụ thuộc tùy chọn (exe 2.7.0.0 / 2.7.1.0)

## SpiritMultiplier

Chi tiết: `mods/spiritmultiplier/docs/open_issues.md`, `docs/warp_research.md`

- [ ] [research] Convergence: triệu hồi thêm Ash sau Cemetery Shade làm mọi spirit vô hình (người dùng, 2026-10-10); xem SpEffect 295921 trong Smithbox, `docs/open_issues.md`
- [ ] Reforged: spirit gọi ra mặc định ở trạng thái đỏ (Fury) thay vì trắng
- [ ] Reforged + MultiSpirit: kích Fury riêng cho từng Ash (hiện Fury 1 nhóm thì mọi Ash đều xám)
- [ ] Reforged: NoRestResummon không có tác dụng sau Spirit-Severing Blade (nghi luật Reforged, không phải mod)
- [ ] Seamless Co-op: spirit mất màu ma (tạm dừng 2026-10-02)
- [ ] [chờ phản hồi] Seamless Co-op 2 người crash khi gọi Mimic số lượng lớn (Spixtea, 2026-10-20, Multiplier=25); thử lại với `InstantMimic`, `docs/mimic_tear.md`

## SpeedMultiplier

- [ ] Seamless Co-op: đồng bộ hình ảnh tốc độ giữa các máy bằng Steam P2P (hướng B, chốt 2026-10-03; xem `docs/seamless.md`)
- [ ] Skin ngựa khác (không phải Torrent gốc) không nhận tăng tốc của ngựa (jiuxin0117, 2026-10-15)
- [ ] Hệ số tốc độ đánh theo loại vũ khí (InvertedButt, 2026-10-01; gộp cả cung/nỏ riêng, tạm không làm)
- [ ] Tách hệ số tốc độ kẻ địch thường / boss (Nickademus84, 2026-10-13; tách theo nhóm hành động như player thì chưa khả thi)
- [ ] Tuỳ chọn: Seamless cho Spirit / Enemy / nạn nhân critical; chia nhóm boss / spirit

## RuneMultiplier

- [ ] Nghiên cứu hệ số rune riêng theo nguồn (giết địch / bán đồ...; julianpratt, 2026-08-31)

## Chung

- [ ] Gắn `common::conflict` (cảnh báo hook trùng mod khác) vào các mod hook byte còn lại: autoregen, runemultiplier; xem `shared/docs/conflict.md`
