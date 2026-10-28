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
- [ ] [verify] `HubHpBars` chạy thật trên bản exe cũ 2.6.2 / 2.7.0 / 2.7.1: mẫu và quan hệ hàm đã khớp bằng IDA, chưa thử trong game (`docs/buddy_hud.md`)
- [ ] [chờ phản hồi] Seamless Co-op 2 người crash khi gọi Mimic số lượng lớn (Spixtea, 2026-10-20, Multiplier=25); thử lại với `InstantMimic`, `docs/mimic_tear.md`
- [ ] [research] Giữ spirit qua ngồi grace và dịch chuyển (người dùng, 2026-10-26); bước tiếp theo là probe `DisappearAll` / unload, `docs/persist_spirits.md`

## SpeedMultiplier

- [ ] Seamless Co-op: đồng bộ hình ảnh tốc độ giữa các máy bằng Steam P2P (hướng B, chốt 2026-10-03; xem `docs/seamless.md`)
- [ ] Skin ngựa khác (không phải Torrent gốc) không nhận tăng tốc của ngựa (jiuxin0117, 2026-10-15)
- [ ] Hệ số tốc độ đánh theo loại vũ khí (InvertedButt, 2026-10-01; gộp cả cung/nỏ riêng, tạm không làm)
- [ ] Tách hệ số tốc độ kẻ địch thường / boss (Nickademus84, 2026-10-13; tách theo nhóm hành động như player thì chưa khả thi)
- [ ] Tuỳ chọn: Seamless cho Spirit / Enemy / nạn nhân critical; chia nhóm boss / spirit

## RuneMultiplier

- [ ] Nghiên cứu hệ số rune riêng theo nguồn (giết địch / bán đồ...; julianpratt, 2026-08-31)

## Kiểm tra lại sau đợt dọn code

Chi tiết từng mục: `docs/cleanup_verify.md`. Chỉ build và unit test, chưa chạy trong game.

- [ ] [verify] `shared/` (codepatch, diag, memscan, config/toml, task/logger): weightmultiplier, spiritmultiplier, speedmultiplier + Seamless, mọi hook AOB
- [ ] [verify] AutoRegen: tick `Trigger` 0-4, gesture ngồi/cast/đứng dậy, hồi khi đánh trúng (`Mode`, `DamageType`, `ExcludeAow`), hook cài lại khi reload
- [ ] [verify] AnimCancel: guard/dodge/jump/weapon swap cắt như cũ, luật guard (dual wield, Twin Blades), probe và log mới
- [ ] [verify] DropMultiplier: `Mode` 0 và 1, `Percentage` 100, `[Materials]`, reload không cộng dồn, log mới
- [ ] [verify] FasterRevival: chết thường, giữa đòn, đòn tóm, rơi vực; hồi sinh vẫn ~4-5s, log `Death:` / `Fade:` mới
- [ ] [verify] InfiniteAilment: Scarlet Rot, Poison, Black Flame, Destined Death (cháy và giảm máu tối đa) áp đúng, reload không cộng dồn, log mới
- [ ] [verify] PassiveRunes: cộng cố định / theo cấp / lãi kép mỗi chu kỳ, mốc thời gian, đồng hồ dừng ngoài thế giới, giá trị âm, log mới
- [ ] [verify] RiseArcher: buff Bow / Crossbow / Ballista / đạn áp đúng, reload và menu không cộng dồn, cảnh báo ID đạn không tìm thấy, log mới
- [ ] [verify] RuneMultiplier: hook cài được (log `Hook: installed`), nhân đúng rune nhận được, không nhân khoản trừ, reload đổi hệ số, mod khác dùng AddSoul không xung đột
- [ ] [verify] WeightMultiplier: hook cài được, Mode 0 (nhân) và Mode 1 (cố định), đổi Mode khi reload và từ menu, phát hiện hook của mod trọng lượng khác
- [ ] [verify] WindowResize: viền kéo, mũi tên kéo, khoá tỉ lệ, kích thước nhỏ nhất, kích thước khởi động, kéo cạnh và góc
- [ ] [verify] SpeedMultiplier: tốc độ người chơi theo nhóm, Torrent, spirit, kẻ địch, nạn nhân chí mạng, [[Override]], Seamless (stub), probe, log mới
- [ ] [verify] SoulsTeleport: WHERE / HERE giữa hai máy, teleport tới đồng đội, từ chối khi thù địch, chặn gói giả người gửi, menu và chặn chuột
- [ ] [verify] SpiritMultiplier: [verify] spiritmultiplier: sau dọn code, thử trong game (chi tiết docs/cleanup_verify.md)

## Chung

- [ ] Gắn `common::conflict` (cảnh báo hook trùng mod khác) vào các mod hook byte còn lại: autoregen, runemultiplier; xem `shared/docs/conflict.md`. AOB đầu hàm hỏng khi bị hook trước nên cần pattern đuôi

## JourneyHUD

Chi tiết: `mods/journeyhud/docs/design.md`

- [ ] [bug] JourneyHUD: game crash hỏng heap (0xC0000374, 2026-10-27 20:51, game đang giải phóng `MicrosoftDiskFileOperator`), chưa rõ mod nào; thử chỉ JourneyHUD, bấm F5 ít thôi
- [ ] [verify] JourneyHUD: khối vàng của thanh posture khi bị rút, và poise/posture trên 2.7.1.0 (đã thử 2.7.2.0)
- [ ] [verify] JourneyHUD: bản đồ nền trong hầm ngầm và dungeon, không treo ở màn hình tải; grace chỉ hiện cái đã tìm thấy, cả dungeon (`docs/map.md`)
- [ ] [research] JourneyHUD: buildup trong co-op Seamless (2026-10-28, chưa có dịp thử): hai máy cùng đánh một kẻ địch, so dòng log `Buildup:`; máy chủ thấy tổng, máy khách chỉ thấy phần mình? (`docs/buildup.md`)
- [ ] [research] JourneyHUD: hiện hiệu ứng SpEffect đang chạy của kẻ địch khoá (icon và thời gian còn lại) dưới panel buildup, theo ý tưởng ERBuffTimer; layout đã kiểm IDA (còn bộ đếm trên 3 bản exe cũ), cần log xem kẻ địch có hiệu ứng nào có icon (`docs/effects.md`)
- [ ] [research] JourneyHUD: kẻ địch xung quanh trên minimap, đầu chuỗi beacon trên 2.6.2.0 đến 2.7.1.0 (`docs/map.md`)
