# codepatch: chọn cách patch code của game

**Status 2026-10-13: đang dùng ở `weightmultiplier`, `spiritmultiplier`, `speedmultiplier` (và `sometweaks`, đã bỏ). Code ở `shared/src/codepatch.rs`.**

## Chọn cách nào

| Tình huống | Dùng | Ghi chú |
|---|---|---|
| Lệnh thay thế vừa khít cửa sổ byte gốc | `overwrite_bytes` | Không stub, không jump |
| Vị trí patch là `call rel32` / `jmp rel32` | `redirect_rel32` | Chỉ đổi 4 byte displacement; xem bên dưới |
| Cần stub nhưng vị trí không phải call, chỉ có >= 5 byte | `install_jmp_hook` | `E9 rel32` + NOP phần dư, stub phải nằm trong ±2 GiB |
| Có >= ~12 byte chỗ trống | jump tuyệt đối (`mov reg, imm64; jmp reg`) | Không cần tìm vùng nhớ gần; xem `hit_hook.rs` của autoregen, `hook.rs` của runemultiplier |

`install_jmp_hook` là phương án dự phòng khi thiếu chỗ cho jump tuyệt đối;
`weightmultiplier` dùng nó vì patch site chỉ có 7 byte.

## Vì sao `redirect_rel32` được ưu tiên cho call

Các mod DLL khác (Seamless Co-op là ví dụ đã gặp) tìm patch site của chúng
bằng chữ ký byte và làm game thoát khi chữ ký không còn khớp. Chữ ký thường để
wildcard displacement của call (`E8 ? ? ? ?`) nhưng không wildcard opcode hay
các lệnh xung quanh. Đổi 4 byte displacement thì chữ ký của mod kia vẫn khớp;
ghi đè cả lệnh bằng `E9` thì không. Nếu mod khác đã chuyển hướng cùng call đó,
`rel32_target` trả về stub của họ, nên hai bên nối chuỗi được với nhau.

Xem `mods/speedmultiplier/docs/seamless.md` và `dumps/seamless-diff/` cho phần
Seamless đã vá những gì.

## Chi tiết cài đặt

- Stub được cấp phát gần vị trí patch bằng `VirtualAlloc` tại địa chỉ gợi ý,
  tìm lùi trước rồi tiến, bước 64 KiB, trong ±0x7000_0000 (chừa khoảng cách với
  giới hạn 0x7FFFFFFF của rel32). Kỹ thuật này gốc từ bản C++ của RuneMultiplier
  và ReductionWeight (nay là WeightMultiplier), vốn học từ các DLL cheat tham khảo.
- Mọi hàm ghi vào code đều bỏ bảo vệ trang, ghi, trả lại bảo vệ cũ, rồi
  `FlushInstructionCache`. Thất bại ở bất kỳ bước nào thì byte gốc giữ nguyên
  (stub đã cấp phát được giải phóng).
