# JourneyHUD: game crash hỏng heap (0xC0000374)

**Status 2026-11-03: mở. Hai lần thấy, cả hai lần stack chỉ có `eldenring.exe` và `ntdll.dll` (không có khung nào của mod, và crash handler dừng ở khung đầu của game), nên chưa chứng minh mod liên quan. Chưa tìm được cách tái hiện.**

## Hai lần thấy

- **2026-10-27 20:51:** game đang giải phóng `MicrosoftDiskFileOperator` (ghi trong TODO lúc đó). Chưa rõ mod nào.
- **2026-11-03 13:21:51** (ModEngine2, ERSS tắt, `external_dlls`: ERNativeUI, JourneyHUD, SpeedMultiplier, FasterRevival, CameraFix, UltrawideFix, Toolbox; bản dựng JourneyHUD trước 13:26): `Exception: heap corruption at ntdll.dll+0x112FC5 (thread 52412)`, `rbx = 0xC0000374` (`STATUS_HEAP_CORRUPTION`), stack `ntdll.dll+0x112FC5 < ntdll.dll+0x95A65 < ntdll.dll+0xAE02 < ntdll.dll+0x12338A < ntdll.dll+0x2754C < eldenring.exe+0x254E298 < eldenring.exe+0x25234FE < eldenring.exe+0x1F65B6F < eldenring.exe+0x1EDFAC6 < eldenring.exe+0x1EDFD22 < eldenring.exe+0x242AE6F < eldenring.exe+0x242AEA4`. Log gốc đã bị ghi đè bởi các lần chạy sau.

## Điều đã biết

- `0xC0000374` là phát hiện hỏng heap lúc giải phóng hoặc cấp phát: kẻ gây hỏng (ghi tràn, giải phóng hai lần) thường đã chạy từ trước, nên khung bị phát hiện không phải nghi phạm.
- Một crash khác trong engine (AV đọc `0x30`, không phải hỏng heap) từng gặp ở máy ta cả khi chưa có JourneyHUD: xem `map.md` mục "Khi nào bắt đầu đọc" và `docs/crash_dump_analysis.md`.

## Việc tiếp theo

- Khi gặp lại: giữ nguyên log (đừng chạy lại ngay) và ghi mod nào đang bật; thử chạy chỉ JourneyHUD; một hướng chưa thử là bật Page Heap (`gflags /p /enable eldenring.exe /full`) để bắt đúng chỗ ghi tràn (chỉ offline, không có Easy Anti-Cheat).
