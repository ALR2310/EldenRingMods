# Phân tích dump crash của eldenring.exe

**Status 2026-10-26: cdb chạy được trên dump đầy đủ (11,8 GB nạp trong khoảng nửa phút đến vài phút); công thức dự phòng bằng python (không cần WinDbg) vẫn đúng. Các dump nằm ở `dumps/` (gitignore, mỗi file 10 đến 20 GB: xoá khi xong). `shared/docs/crash.md` là phần ngược lại, mod tự ghi log lỗi ra file lúc chạy.**

## cdb (ưu tiên)

WinDbg (bản Store) có sẵn nhưng `cdb.exe` không nằm trong PATH:

```
C:\Program Files\WindowsApps\Microsoft.WinDbg_<ver>_x64__8wekyb3d8bbwe\amd64\cdb.exe
```

`<ver>` đổi theo bản cập nhật (đã là `1.2606.22001.0`, rồi `1.2610.1001.0`): tìm bằng `Get-ChildItem "C:\Program Files\WindowsApps" -Directory -Filter "Microsoft.WinDbg_*"`.

- **Dùng file lệnh `-cf`, không dùng `-c "..."` qua `Start-Process -ArgumentList`:** PowerShell cắt chuỗi lệnh ở dấu `;` và cdb báo `bad EventToSignal`. Ghi các lệnh mỗi dòng một lệnh vào một file rồi chạy `& $cdb -z "<đường dẫn tuyệt đối, kiểu Windows>.dmp" -cf cmds.txt *> out.txt` (chạy nền, dump lớn).
- Bộ lệnh tối thiểu: `.lines -e`, `.ecxr`, `r`, `k 25`, `u rip-20 L12`, `lm`, `q`. Với dump mà `RIP` không thuộc module nào, `u` báo `Memory access error`.
- `.ecxr` nạp ngữ cảnh ngoại lệ; `lm` cho biết module nào đang nạp (kể cả các mod DLL, nên thấy ngay mod nào có mặt trong tiến trình lúc crash).
- Exe không có thông tin unwind dùng được (`.pdata` + Arxan), nên `k` thường đầy dữ liệu rác; để tìm chuỗi gọi thật thì quét stack theo mẹo "địa chỉ trả về nằm ngay sau một lệnh call".
- Ánh xạ `exe+RVA` vào database IDA của đúng phiên bản (`dumps/eldenring/<ver>/eldenring.exe.i64`, xem `scripts/ida/README.md`). **Đối chiếu đúng phiên bản exe của dump** (đọc dòng đầu của log mod hoặc `lm`): RVA lệch giữa các bản.

## Dự phòng: python

`pip` đã có `minidump`, `capstone`, `pefile`. `MinidumpFile.parse()` mở được file 18 GB. Tự đọc `CONTEXT` ngoại lệ (x64: `Rax` ở `0x78`, `Rip` ở `0xF8`; `ex.ThreadContext` chỉ là bộ mô tả Rva/DataSize). `thread.ContextObject` của luồng lỗi là ngữ cảnh của handler chứ không phải của chỗ lỗi. `rd.read()` hỏng khi vượt biên segment: đọc theo từng khối cỡ trang.

So sánh `.text` trong dump với exe sạch rất nhiễu (relocation, stub Arxan `eb 72`, nhiều `e9` nhảy vào stub ở địa chỉ thấp).

Log lỗi của các mod khác cũng hữu ích: `MapForGoblins_crash.log` ghi mọi first-chance AV kèm stack phía game, cho biết lỗi lặp lại hay không; sự kiện `Application Error` số 1000 cho offset lỗi.

## Các lần đã phân tích

- **2026-10-04** (`eldenring.exe.22136.dmp`, game 2.7.1.0, profile ME3 Convergence): AV đọc `0x30` tại `exe+0x1E92100`, trong hàm tạo CGShaderResourceView (`sub_141E8A1C0`): ô texture `[rsi]` NULL vì lần tạo buffer 16 byte ngay trước (`sub_141D51E30` → `sub_141E88B40`) trả lỗi mà engine bỏ qua. Lặp lại khoảng 11 giây sau khi chạy ở hai lần (12:03 và 12:07). Không mod nào vá vùng mã render. Máy có hai GPU (AMD Radeon 610M 512 MB và RTX 4060), chưa xác định game chọn card nào.
- **2026-10-26** (`eldenring.exe.162860.dmp`, game 2.7.2.0): crash do chính NoFallDamage đặt hardware breakpoint (debug register) lên HP của người chơi trên 162 luồng. Ngoại lệ là `access violation, execute` tại `RIP = 0x7FF711F1C160`, **nằm ngoài mọi module** (exe ở `0x7FF6D1460000` đến `0x7FF6D725D600`), bộ nhớ ở đó không tồn tại, stack toàn dữ liệu rác. Suy luận (chưa chứng minh trực tiếp): chống can thiệp của game phát hiện debug register và làm hỏng luồng. Kết luận dùng được: **không dùng hardware breakpoint trong DLL của dự án**; muốn quan sát một hàm thì hook inline kiểu `autoregen/hit_hook.rs` (trampoline + prologue chép lại), và trong hook không cấp phát bộ nhớ hay lấy khoá (game đã thay bộ cấp phát).
