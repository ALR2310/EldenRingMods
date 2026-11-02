# crash: ghi lỗi nghiêm trọng vào log mod

**Status 2026-11-02: stack unwind bằng `.pdata`, byte lệnh, chú thích thanh ghi, dòng `outermost:` cho stack overflow; đã thử bằng crash cố ý và crash thật trong game (ME2 và ME3). Bản đầu 2026-10-16. Code ở `shared/src/crash.rs`, cài tự động bởi `logger::install_panic_hook`.**

## Vì sao có

Crash của game khi gọi nhiều Mimic Tear trong Seamless (2026-10-16) không để lại dump WER, không có sự kiện `Application Error`, và log mod dừng giữa chừng (không có `PANIC`). Game tự thoát nên không có gì để phân tích. Cần ghi địa chỉ lỗi ngay trong log mod.

## Cách hoạt động

Vectored exception handler (`AddVectoredExceptionHandler`, đứng đầu danh sách) chỉ **ghi log rồi trả `EXCEPTION_CONTINUE_SEARCH`**, nên không đổi hành vi game. Chỉ ghi các mã lỗi nghiêm trọng: access violation, stack overflow, illegal / privileged instruction, chia 0, fast fail, heap corruption. Mỗi (mã, địa chỉ) ghi một lần, tối đa 16 mục mỗi tiến trình. Mỗi mục là 4 dòng (stack overflow thêm dòng `outermost:`):

```
Exception: access violation, read 0x30, at eldenring.exe+0x1E92100 (thread 1234)
  code: 48 8B 01 ... (16 byte tại địa chỉ lỗi)
  rax=... rcx=0x7FF...(eldenring.exe+0x...) ... (8 thanh ghi), 1 dòng nữa r8-r15
  stack: eldenring.exe+0x1E92100 < eldenring.exe+0x... < SpiritMultiplier.dll+0x... < ...
```

Địa chỉ in dạng `module+RVA`, tra thẳng trên IDA database của phiên bản game (xem `reference_ida_database`). Thanh ghi trỏ vào một module được chú thích `(module+RVA)`.

**Stack** được unwind bằng bảng `.pdata` (`RtlLookupFunctionEntry` + `RtlVirtualUnwind`), frame đầu là chính chỗ lỗi; dừng khi rsp ra ngoài stack của luồng (TEB) hoặc không tăng. Với stack overflow (không unwind được vì stack đã cạn), hoặc khi unwind ra dưới 2 frame, dùng cách cũ: quét 0x400 byte đầu của stack lấy giá trị nằm trong một module và đứng sau lệnh `call` (kém chính xác hơn). Với stack overflow, innermost toàn là vòng đệ quy nên có thêm dòng `outermost:` quét 0x1000 byte trên cùng của stack (từ ngoài vào trong: thread start, ..., chỗ vào vòng lặp).

**Không dùng unhandled-exception filter** (đã thử 2026-11-02 rồi bỏ): để đánh dấu "lỗi làm chết tiến trình" bằng `SetUnhandledExceptionFilter` thì trên ModEngine2 nó bị thay sau vài giây bằng `_scrt_unhandled_exception_filter` của CRT MSVC trong `eldenring.exe` (RVA `0x24FFD78` ở 2.7.2.0; hàm này chỉ gọi `terminate()` cho vài loại C++ exception, trả `EXCEPTION_CONTINUE_SEARCH` cho phần còn lại, không gọi filter trước nó, không ghi dump), nên không ra. Trên ModEngine3 thì ra, nhưng khối `FATAL` chỉ lặp lại địa chỉ của dòng `Exception:` ngay trên nó. Dòng `Exception:` cuối cùng trước khi log dừng đã đủ để điều tra.

## Lưu ý

- Đây là exception **first-chance**: game có thể tự bắt và bỏ qua một số, nên dòng `Exception:` cuối cùng trước khi log dừng mới là lỗi làm chết tiến trình, không phải mọi dòng. Ví dụ `QuestPath.dll+0x18108` luôn hiện lúc khởi động: một hàm đọc thử bộ nhớ có bắt lỗi (`mov eax,[rcx]` ... `xor al,al`), vô hại.
- Mỗi DLL mod có một bản handler riêng nên cùng một lỗi có thể hiện ở log của nhiều mod.
- Không ghi C++ exception (`0xE06D7363`) và breakpoint.
- Chưa có: minidump, bắt hang / `TerminateProcess`, ghi mod hay tính năng nào đang chạy.
