# Overlay không hook `Present`: bọc swap chain (học từ ERBuffTimer 1.2.3)

**Status 2026-11-02: phân tích tĩnh ERBuffTimer 1.2.3, chưa có code hay kiểm trong game.**

Phân tích tĩnh `ERBuffTimer.dll` 1.2.3 (`C:\Users\Admin\Downloads\ERBuffTimer 1.2.3`, 950 KB, sha256 `cb7f279d...516e`), không chạy nó. Trước đó ta chỉ xem 1.2.1 (`effects.md`), khi ấy nó hook `Present` và `ExecuteCommandLists` bằng cách sửa vtable.

## Changelog liên quan (1.2.1 đến 1.2.3)

- 1.2.1: dây chuyền hook của Steam overlay và các mod overlay khác gọi vòng làm game thoát lúc khởi động; hook chuột chỉ cài khi bấm F9 lần đầu.
- 1.2.2: ME3 + Steam overlay vẫn crash; hiện chung với ERGestureWheel, ERItemWheel, ERVoiceChat.
- 1.2.3: bỏ hook `Present`, vẽ qua **lớp bọc swap chain**; đồng hồ FPS của Steam đúng lại (trước đó một số mod overlay làm nó hiện 180 thay vì 60: `Present` bị đếm nhiều lần); nếu không gắn được thì **tắt overlay và ghi lý do vào log** thay vì liều crash.

Triệu chứng này giống hệt `questpath.md` (Steam overlay đệ quy trong hook, ME3 nạp DLL cùng lúc): ERBuffTimer đã đi từ cách của hudhook (hook vtable/hàm) sang cách không đụng mã của ai.

## Cách 1.2.3 làm

1. **Hook IAT (bảng import của exe game; suy từ chuỗi log, chưa đọc hàm cài)** `CreateDXGIFactory`, `CreateDXGIFactory1`, `CreateDXGIFactory2` của `dxgi.dll` (log `game import %s hooked`, `(already hooked)`; nếu game không import chúng: `swapchain wrapper disabled`). Hàm thay thế gọi hàm gốc rồi, với factory trả về, chạy một lần (cờ `byte_1800E18CD`) `sub_180077660` (đã đọc).
2. `sub_180077660` sửa **vtable của factory** (`VirtualProtect` đổi 8 byte một slot, không đụng mã): slot 10 `CreateSwapChain` (+0x50) và slot 15 `CreateSwapChainForHwnd` (+0x78), lưu con trỏ gốc rồi thay bằng hàm của mod. Log `swapchain creation hooked (CreateSwapChain %d, CreateSwapChainForHwnd %d)`.
3. Hàm thay thế (`sub_180077260` / `sub_180077440`):
   - Đếm lồng bằng bộ đếm TLS (+28): chỉ làm việc ở lần gọi **ngoài cùng**, các hook khác (Steam, RTSS, ReShade) gọi lồng thì bỏ qua.
   - Gọi hàm gốc; thành công, có swap chain, và cửa sổ lớn hơn 128x128 (`sub_1800771E0`, dùng `GetClientRect` khi kích thước 0) thì bọc.
   - QueryInterface swap chain lấy `IDXGISwapChain4`; thiếu thì không bọc (log `game swapchain has no IDXGISwapChain4, not wrapped`).
   - Tham số đầu của `CreateSwapChain` là thiết bị: QueryInterface ra `ID3D12CommandQueue` (IID `unk_1800CCC08`, suy luận), gọi slot 18 (`GetDesc`) và chỉ giữ queue nếu hợp lệ (lấy một lần, `qword_1800E1C38`). Log `wrapped (queue found|NOT found)`. Overlay chỉ chạy khi có queue.
   - Thay con trỏ swap chain trả về bằng `SwapChainWrapper` (24 byte: vtable, con trỏ swap chain thật, refcount).
4. **`SwapChainWrapper`** (class COM, vtable 40 slot): `QueryInterface` tự trả mình cho IID của `IDXGIObject`/`DeviceSubObject`/`SwapChain` đến `SwapChain4`, IID khác chuyển cho swap chain thật; các slot khác chuyển thẳng. Hai slot có việc riêng (đọc `Present`; `ResizeBuffers` chưa):
   - `Present` (slot 8, `sub_180076F00`): nếu chưa ở trong lần gọi lồng (TLS +16) thì vẽ overlay **trước** `Present` thật (`sub_180076BA0`), rồi gọi `Present` thật. Cờ `DXGI_PRESENT_TEST` (bit 0) không vẽ. Overlay khởi tạo lười ở lần `Present` đầu (`overlay initialized` / `overlay init FAILED`; hỏng một lần thì tắt hẳn).
   - `ResizeBuffers` (slot 13, `sub_180076FB0`): có xử lý riêng, chưa đọc (chắc để thả tài nguyên của overlay trước khi đổi cỡ).
5. **Dự phòng:** nếu mod nạp muộn (game đã tạo swap chain) thì không có `Present` nào đi qua wrapper; sau một khoảng (changelog 1.2.2 nói 10 s) log `no Present through the swapchain wrapper (mod loaded after the game created it?), using vtables` rồi quay lại cách cũ hook vtable `Present`/`ExecuteCommandLists`. Cách cũ này tự nhận: vào đầu hàm đã bị mod khác vá thì gọi vòng qua đoạn vá (`calling around the patch`), hook lặp thì gọi thẳng hàm gốc (`hook loop`). Nếu `Present` cũng không gọi qua vtable (có chương trình khác chặn) thì tắt overlay kèm lời khuyên tắt RTSS, Afterburner, Discord overlay, ReShade.

Tóm lại: ERBuffTimer **không bao giờ vá mã** của `Present`; nó đổi 2 con trỏ trong vtable factory của game rồi trả cho game một đối tượng của nó. Steam overlay hook `Present` thật của swap chain mà nó thấy, không thấy mod ở giữa, nên không có vòng gọi.

## Điều ta có thể học

Hiện hudhook 0.9.3 (JourneyHUD, SoulsTeleport) **tạo swap chain tạm để lấy vtable `Present` rồi hook bằng MinHook** (inline). Đó đúng là kiểu bị ba lớp hook lồng nhau ở `questpath.md`.

1. **Bọc swap chain thay cho hook `Present`** là cách bền nhất với Steam overlay, RTSS, ReShade, QuestPath, ME3; cũng sửa luôn lỗi FPS Steam. Nhưng hudhook không có chế độ này: muốn dùng phải tự viết lớp bọc (khoảng 40 slot COM) và tự đẩy `Present` vào hudhook (nó cần `Present` của swap chain và command queue của game để chạy `ImguiRenderLoop`; hudhook có `Pipeline` bên trong, kiểm tra xem có gọi được riêng không), hoặc thay hudhook. Công sức lớn; nên coi là hướng dài hạn nếu còn nhận báo crash từ overlay.
2. **Việc nhẹ làm trước:**
   - **Tắt thay vì crash** (đã có sẵn ở ý tưởng `General.HookDelayMs` của `questpath.md`): dù hook kiểu nào, nếu overlay không gắn được thì ghi log lý do và để mod còn lại (radar, thanh buildup trong movie game, ...) chạy tiếp, không chết cả game.
   - **Log chẩn đoán rõ** như họ: hook nào cài được (`%d`), có queue không, `Present` có đến không. Hiện log JourneyHUD chỉ dừng ở `Overlay: DX12 hooks applied`, khó biết chết ở bước nào.
   - **Hook lười ở nơi gây crash:** họ cài hook chuột chỉ khi bấm F9 lần đầu (1.2.1); áp dụng cho mọi hook không cần ngay lúc nạp.
   - **Chỉ vẽ khi cửa sổ lớn hơn 128 px và không phải `DXGI_PRESENT_TEST`.**
3. **Hook IAT `CreateDXGIFactory*`** (không phải hook inline) là cách bắt thời điểm game tạo swap chain mà không đụng mã nào; có thể dùng riêng để biết "swap chain đã sinh" làm điều kiện trước khi hudhook tạo swap chain tạm (thay cho hẹn giờ cứng 3000 ms của `questpath.md`). Không tương đương với bọc nhưng rẻ hơn nhiều.
4. **Hạn chế của chính cách này:** phải nạp trước khi game tạo swap chain (ở ME3 mod nạp rất sớm nên thường được; mod đặt `initializer.delay` thì có thể lỡ, và ERBuffTimer phải có dự phòng hook vtable). Chỉ chạy được khi game import `CreateDXGIFactory*` từ exe (ER có); phụ thuộc `IDXGISwapChain4`.

## Bài học khác trong 1.2.3

- **Gộp buff trùng:** trước đây gộp theo icon nên Golden Vow biến mất khi có buff cùng icon (Crystal Tear nút Dexterity); nay gộp khi **cả icon và tên** trùng. JourneyHUD hiện mỗi `SpEffectParam` một ô (`target.rs::read_effects`, lọc `Cycled`), không gộp theo icon, nên không dính lỗi này; nếu sau này gộp thì khoá theo (icon, id param/tên).
- `show_stats` từng hiện `+100% equip load` cho gần mọi buff vì đọc trường mặc định 1.0 như thay đổi: chỉ hiện giá trị khác mặc định.

## Chưa kiểm

- Mọi điều trên là phân tích tĩnh; không chạy, không đối chiếu runtime. IID của queue, nhóm slot và ý nghĩa cờ suy từ mã và vtable; vtable `SwapChainWrapper` đọc từ file (slot 8 `Present`, 13 `ResizeBuffers`).
- Chưa xem phần hook IAT (hàm thay thế `CreateDXGIFactory*`) và cách overlay của họ dùng Dear ImGui DX12 (backend tự viết, vẽ lên back buffer bằng queue lấy được); nếu quyết định làm theo thì đọc tiếp các hàm gọi `sub_180075720` (khởi tạo) và `sub_180075D40` (vẽ).
