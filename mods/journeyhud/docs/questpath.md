# Tương thích với QuestPath

**Status 2026-11-02: đã ghi nhận cách tránh crash trên ModEngine3; nguyên nhân và cách sửa trong mod chưa được kiểm.**

Báo cáo: YuriElaris (Nexus, 2026-11-01): JourneyHUD + QuestPath làm game thoát ngay khi khởi động trên ModEngine3.

## Hiện tượng (log 2026-11-01)

- ME2: chạy bình thường. ME3: crash ngay lúc nạp, log JourneyHUD dừng trước `Overlay: DX12 hooks applied`.
- `Exception: access violation, read 0xFFFFFFFFFFFFFFFF, at QuestPath.dll+0x18108` có ở cả ME2 và ME3, thread của QuestPath, ngay sau `flag-manager search` của nó: QuestPath tự dò bộ nhớ và tự bắt, vô hại.
- `Exception: stack overflow at GameOverlayRenderer64.dll` (Steam overlay) chỉ ở ME3: đệ quy trong hook của Steam, chết trước khi hudhook xong.

## Tái hiện ở máy ta (2026-11-02)

Profile ME3 `TestMod` chỉ bật JourneyHUD, QuestPath, WindowResize (nay gộp vào Toolbox; không delay, không ERSS-FG) thì crash lặp lại mọi lần: `Exception: stack overflow at GameOverlayRenderer64.dll+0xDE197`, vòng đệ quy nằm trong Steam overlay. Dòng `outermost:` của `crash.rs` cho thấy thread tràn stack do JourneyHUD tạo ra (`JourneyHUD.dll+0x133235` rồi `+0x73677`, `+0x15059`, `+0x1AC98B`, `+0x1AC722`, sau đó vào `ntdll`), cùng với việc log không có `Overlay: DX12 hooks applied`: đó là thread đang cài hook DX12. Các RVA chưa được dịch ra tên hàm (không có công cụ đọc PDB). Cùng build chạy bình thường trên ME2.

## Nguyên nhân (suy đoán, chưa chứng minh)

hudhook tạo swapchain tạm để lấy vtable DX12 trong lúc QuestPath hook việc tạo DXGI (`DXGI creation hooks installed`) và Steam overlay cũng hook: ba lớp hook gọi lẫn nhau. Mỗi mod chạy riêng thì không lỗi; ME3 nạp DLL gần như đồng thời nên trùng thời điểm.

## Cách tránh hiện tại

Trong file `.me3`, đặt delay cho JourneyHUD (đã thử, hết lỗi):

```toml
[[natives]]
path = "dll/JourneyHUD.dll"
initializer.delay.ms = 3000
```

## Việc triệt để

Xem thêm `overlay_swapchain.md` (ERBuffTimer 1.2.3 bọc swap chain để tránh đúng lỗi này).

Tự trì hoãn `ui::install()` trong mod để không phải bắt người dùng sửa `.me3`: hoặc `General.HookDelayMs` (mặc định ~3000), hoặc đợi cho đến khi `CSTaskImp` sẵn sàng rồi cộng thêm một khoảng. Cần thử trong game với ME2, ME3 và QuestPath.
