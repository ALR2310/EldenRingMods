# crash: ghi lỗi nghiêm trọng vào log mod

**Status 2026-10-16: hoạt động (build + test đơn vị), chưa gặp crash thật để kiểm chứng. Code ở `shared/src/crash.rs`, cài tự động bởi `logger::install_panic_hook`.**

## Vì sao có

Crash của game khi gọi nhiều Mimic Tear trong Seamless (2026-10-16) không để lại dump WER, không có sự kiện `Application Error`, và log mod dừng giữa chừng (không có `PANIC`). Game tự thoát nên không có gì để phân tích. Cần ghi địa chỉ lỗi ngay trong log mod.

## Cách hoạt động

Vectored exception handler (`AddVectoredExceptionHandler`, đứng đầu danh sách) chỉ **ghi log rồi trả `EXCEPTION_CONTINUE_SEARCH`**, nên không đổi hành vi game. Chỉ ghi các mã lỗi nghiêm trọng: access violation, stack overflow, illegal / privileged instruction, chia 0, fast fail, heap corruption. Mỗi (mã, địa chỉ) ghi một lần, tối đa 16 mục mỗi tiến trình. Mỗi mục là 4 dòng:

```
Exception: access violation, read 0x30, at eldenring.exe+0x1E92100 (thread 1234)
  rax=... rcx=... (8 thanh ghi), 1 dòng nữa r8-r15
  stack: eldenring.exe+0x... < SpiritMultiplier.dll+0x... < ...
```

Địa chỉ in dạng `module+RVA`, tra thẳng trên IDA database của phiên bản game (xem `reference_ida_database`). Stack là các giá trị trên stack (0x400 byte đầu) nằm trong một module và đứng sau lệnh `call`; đọc bằng `ReadProcessMemory` nên không tự gây lỗi.

## Lưu ý

- Đây là exception **first-chance**: game có thể tự bắt và bỏ qua một số, nên dòng cuối của log mới quan trọng, không phải mọi dòng `Exception:`.
- Mỗi DLL mod có một bản handler riêng nên cùng một lỗi có thể hiện ở log của nhiều mod.
- Không ghi C++ exception (`0xE06D7363`) và breakpoint.
