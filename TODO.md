# TODO

Chỉ liệt kê việc **còn mở**, mỗi việc 1 dòng. Việc xong xoá đi (lịch sử ở
`mods/<mod>/HISTORY.md`); lý do / chi tiết kỹ thuật ở `mods/<mod>/docs/`.
Quy ước đầy đủ: `docs/rules/style.md`.

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
- [ ] [verify] `HubHpBars` chạy thật trên bản exe cũ 2.6.2 / 2.7.0 / 2.7.1: mẫu và quan hệ hàm đã khớp bằng IDA, chưa thử trong game (`docs/buddy_hud.md`)
- [ ] [chờ phản hồi] Seamless Co-op 2 người crash khi gọi Mimic số lượng lớn (Spixtea, 2026-10-20, Multiplier=25); thử lại với `InstantMimic`, `docs/mimic_tear.md`
- [ ] SpiritMultiplier: giữ spirit khi fast travel (người dùng, 2026-10-26); ngồi grace đã xong, `docs/persist_spirits.md`

## SpeedMultiplier

- [ ] Seamless Co-op: đồng bộ hình ảnh tốc độ giữa các máy bằng Steam P2P (hướng B, chốt 2026-10-03; xem `docs/seamless.md`)
- [ ] Skin ngựa khác (không phải Torrent gốc) không nhận tăng tốc của ngựa (jiuxin0117, 2026-10-15)
- [ ] Hệ số tốc độ đánh theo loại vũ khí (InvertedButt, 2026-10-01; gộp cả cung/nỏ riêng, tạm không làm)
- [ ] Tách hệ số tốc độ kẻ địch thường / boss (Nickademus84, 2026-10-13; tách theo nhóm hành động như player thì chưa khả thi)
- [ ] Tuỳ chọn: Seamless cho Spirit / Enemy / nạn nhân critical; chia nhóm boss / spirit

## RuneMultiplier

- [ ] Nghiên cứu hệ số rune riêng theo nguồn (giết địch / bán đồ...; julianpratt, 2026-08-31)

## JourneyHUD

Chi tiết: `mods/journeyhud/docs/design.md`

- [ ] [bug] JourneyHUD: game crash hỏng heap (0xC0000374), thấy 2026-10-27 và 2026-11-03, stack chỉ có `eldenring.exe` / `ntdll`, chưa rõ mod nào (`mods/journeyhud/docs/heap_corruption.md`)
- [ ] [research] JourneyHUD: buildup trong co-op Seamless (2026-10-28, chưa có dịp thử): hai máy cùng đánh một kẻ địch, so dòng log `Buildup:`; máy chủ thấy tổng, máy khách chỉ thấy phần mình? (`docs/buildup.md`)
- [ ] [verify] JourneyHUD: chữ giờ dưới icon trạng thái của game khi đổi cỡ HUD trong tuỳ chọn game (`mods/journeyhud/docs/effects.md`); còn làm: khối sát thương giữa người chơi (co-op)
- [ ] [verify] JourneyHUD: chấm kẻ địch (`Minimap.Layers.Enemies`) trong hầm, dungeon legacy, DLC và quân địch ngoài trời ngoài `chr_sets`; việc còn lại: đưa bộ lọc kẻ địch lên `common` (`mods/journeyhud/docs/radar.md`)
- [ ] [research] JourneyHUD: đầu chuỗi beacon trên 2.6.2.0 đến 2.7.1.0 (`mods/journeyhud/docs/map.md`)
- [ ] [verify] JourneyHUD: nền bản đồ ở Finger Birthing Grounds (DLC) sau quy tắc điểm gốc cho dungeon legacy; Farum Azula và Haligtree đã đúng (`mods/journeyhud/docs/map.md`)
- [ ] Phát hành JourneyHUD bản sửa lỗi bản đồ (3 bullet ở `[Unreleased]`), rồi báo Ver3ion trên Nexus (Mistwood, Farum Azula, InCaves) và dán lại trang mô tả

## NoFallDamage

- [chờ phản hồi] Code và `docs/fall_damage.md` (2026-10-26) chỉ nằm trong `git stash` (`!!GitHub_Desktop<mod/nofalldamage>`), nhánh `mod/nofalldamage` chưa có file mod; khôi phục vào nhánh rồi thử KillGuard trong game, đừng xoá stash.
