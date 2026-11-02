# crash: ghi lỗi nghiêm trọng vào log mod

**Status 2026-11-02: thêm dòng `FATAL`, unwind bằng `.pdata`, byte lệnh, chú thích thanh ghi (build + test đơn vị, chưa thử trong game, chưa gặp crash thật). Bản đầu 2026-10-16. Code ở `shared/src/crash.rs`, cài tự động bởi `logger::install_panic_hook`.**

## Vì sao có

Crash của game khi gọi nhiều Mimic Tear trong Seamless (2026-10-16) không để lại dump WER, không có sự kiện `Application Error`, và log mod dừng giữa chừng (không có `PANIC`). Game tự thoát nên không có gì để phân tích. Cần ghi địa chỉ lỗi ngay trong log mod.

## Cách hoạt động

Vectored exception handler (`AddVectoredExceptionHandler`, đứng đầu danh sách) chỉ **ghi log rồi trả `EXCEPTION_CONTINUE_SEARCH`**, nên không đổi hành vi game. Chỉ ghi các mã lỗi nghiêm trọng: access violation, stack overflow, illegal / privileged instruction, chia 0, fast fail, heap corruption. Mỗi (mã, địa chỉ) ghi một lần, tối đa 16 mục mỗi tiến trình. Mỗi mục là 5 dòng:

```
Exception: access violation, read 0x30, at eldenring.exe+0x1E92100 (thread 1234)
  code: 48 8B 01 ... (16 byte tại địa chỉ lỗi)
  rax=... rcx=0x7FF...(eldenring.exe+0x...) ... (8 thanh ghi), 1 dòng nữa r8-r15
  stack: eldenring.exe+0x1E92100 < eldenring.exe+0x... < SpiritMultiplier.dll+0x... < ...
```

Địa chỉ in dạng `module+RVA`, tra thẳng trên IDA database của phiên bản game (xem `reference_ida_database`). Thanh ghi trỏ vào một module được chú thích `(module+RVA)`.

**Stack** được unwind bằng bảng `.pdata` (`RtlLookupFunctionEntry` + `RtlVirtualUnwind`), frame đầu là chính chỗ lỗi; dừng khi rsp ra ngoài stack của luồng (TEB) hoặc không tăng. Với stack overflow, hoặc khi unwind ra dưới 2 frame, dùng cách cũ: quét 0x400 byte đầu của stack lấy giá trị nằm trong một module và đứng sau lệnh `call` (kém chính xác hơn).

**Dòng `FATAL`:** `SetUnhandledExceptionFilter` ghi lại exception với đầu dòng `FATAL Exception:` (cùng bộ dòng trên, bất kể mã lỗi, bỏ qua dedupe) rồi gọi filter cũ. Đây là lỗi làm chết tiến trình; các dòng `Exception:` khác có thể do game tự bắt. Nếu game đặt lại filter sau mod thì không có dòng `FATAL`, khi đó dùng dòng cuối của log.

## Lưu ý

- Đây là exception **first-chance**: game có thể tự bắt và bỏ qua một số, nên chỉ dòng `FATAL Exception:` (hoặc dòng cuối log nếu không có) là lỗi làm chết tiến trình, không phải mọi dòng `Exception:`.
- Mỗi DLL mod có một bản handler riêng nên cùng một lỗi có thể hiện ở log của nhiều mod.
- Không ghi C++ exception (`0xE06D7363`) và breakpoint ở handler thường; chỉ dòng `FATAL` ghi mọi mã.
- Chưa có: minidump, bắt hang / `TerminateProcess`, ghi mod hay tính năng nào đang chạy.
