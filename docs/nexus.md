# Nexus Mods: viết trang mod, changelog, gọi API, logo

**Status 2026-10-26: quy tắc gom từ phản hồi của người dùng trong các phiên 2026-09-10 đến 2026-10-13 (trước đó nằm ở memory của Claude, nay nằm ở đây). Quy trình phát hành từng bước nằm ở skill `deploy-nexus-mod` (`.claude/skills/deploy-nexus-mod/SKILL.md`); danh sách mod ID và cách đối chiếu changelog nằm ở `CLAUDE.md`.**

## Trang mod (`nexus_page.bbcode`)

Khung mẫu cho mod mới: `docs/nexus_page.bbcode`. Trang được dán tay lên Nexus, nên sửa xong phải nhắc người dùng dán.

**Phong cách (người dùng duyệt 2026-10-13, "format này đẹp"):** chạy `python scripts/nexus_style.py <file>` (idempotent, `--check` để xem trước) thay vì tự gõ tay. Nó biến tiêu đề thường, Disclaimer đỏ cũ và khối Support cũ thành:

- Tiêu đề mục: `[size=4][b]` + gradient vàng, **mỗi chữ một thẻ `[color=#hex]`**, sáng ở giữa (`#F6DE92`), ấm hơn ở hai đầu (`#C0860A`), dấu cách không gắn thẻ. Không dùng đầu tối kiểu `#6A4700` (đã bị từ chối vì quá tối).
- Dòng Disclaimer: cùng kỹ thuật nhưng đỏ sang san hô (`#D93A2B` hai đầu, `#FF9C8C` ở giữa) để vẫn đọc ra là cảnh báo và khác tiêu đề vàng.
- Mục Support (luôn là mục cuối): câu "If you enjoy this mod and want to support my work, you can buy me a coffee via Ko-fi or PayPal! Any donation is greatly appreciated.", hai từ Ko-fi và PayPal là link (dự phòng khi ảnh hỏng), rồi nút Ko-fi `[img width=203]` và logo PayPal `PP_logo_h_200x51.png` cạnh nhau. Ko-fi `https://ko-fi.com/alr2310`, PayPal `https://www.paypal.com/paypalme/alr2310` (`ko-fi.com/L7S227ME2P` là link cũ, đã thay).
- Chú thích dưới gif demo: `[size=3][b][color=#5DADE2]...`. Nexus nhận `[img width=N]`.
- `windowresize` (nội bộ, không đăng) không áp dụng.

**Từ ngữ:**

- **Không bao giờ dùng "lifesteal"** (cả "true lifesteal") ở bất cứ chỗ nào của trang: dòng giới thiệu, Features, changelog. Tả cơ chế bằng lời thường, ví dụ "percent of damage dealt". Bị từ chối hai lần trong cùng ngày 2026-09-14 (autoregen); coi là cấm chung cho mọi mod. Trước khi đưa bản nháp, tìm từ này trong đó.
- Dòng giới thiệu theo khuôn: "A DLL mod that adds/lets you ..., plus/or ... . Fully configurable via its config file." Ngắn, không ghi tên file.

**Features: chỉ ghi điều người chơi thật sự được hưởng** (chốt trên SpiritMultiplier 2026-09-29):

- Sửa lỗi do chính mod gây ra (vd. "kẻ địch không còn biến mất khi có nhiều spirit", vốn chỉ xảy ra vì mod thêm spirit) không phải tính năng, bỏ khỏi Features.
- Giới hạn an toàn (vd. MaxSpirits 60) không phải một bullet Features; nói một lần ở dòng riêng, giọng nhẹ.
- Dùng từ thường ("ghost color", không phải "blue ghosts"); mỗi bullet một ý, một câu ngắn (xem mục "Văn phong" trong `CLAUDE.md`).

**Mục "Why this mod?"** (tuỳ chọn, người dùng rất thích ở SoulsTeleport 2026-09-25): ngay sau dòng giới thiệu, in nghiêng, ngôi thứ nhất, góc nhìn người chơi, tiếng Anh, không chi tiết kỹ thuật. Dạng mặc định gồm **2 đoạn**: một câu hỏi gần gũi về điều phiền mà mod giải quyết, rồi "So I made this mod: ...". Không thêm đoạn "Yeah, me too" + đùa (người dùng bỏ nó ở SpeedMultiplier 2026-09-29). Không phải mod nào cũng cần: SpiritMultiplier bị bỏ hẳn mục này, nên **đề nghị chứ đừng mặc định**.

## Changelog

Nguồn duy nhất là `mods/<mod>/CHANGELOG.md` (từ 2026-10-07 không còn khối changelog trong `nexus_page.bbcode`); skill `deploy-nexus-mod` gửi nội dung đó lên tab Changelog của Nexus.

- **Mỗi bullet ngắn nhất có thể**, một tính năng hoặc một lỗi một dòng, nói điều người dùng thấy. Không ghi tên key ini, ngưỡng số, cơ chế hay "vì sao nó hỏng": những thứ đó vào `HISTORY.md` và `docs/`. Bị sửa hai lần: AutoRegen 2.5.0 (2026-09-10, ba bullet cho cái chỉ cần một dòng) và AutoRegen 2.6.3 (2026-09-24, "Fixed config changes being ignored when the mod is installed in a folder with non-English characters" vẫn quá cụ thể, phải là "Fixed install paths with non-ASCII characters.").
- Gọi tên **loại** sửa lỗi, không tả triệu chứng hay tình huống. Bug thì mở đầu "Fixed ...", chỉ dùng "Added ..." cho tính năng mới.
- **Gửi lên API không thêm `- ` hay ký hiệu bullet nào vào đầu dòng.** Nexus tự biến mỗi dòng thành một mục (lần chạy thật đầu tiên của skill, 2026-09-11, tạo ra `- -` kép trên trang). Lấy nội dung mỗi bullet trong `CHANGELOG.md` bỏ `- `, nối bằng `\n`, dùng giống hệt cho cả hai trường `changelog` và `description` của `createModFileVersion`.

## API

- **Khoá:** `NEXUS_API_KEY` trong `.env` ở gốc repo (đã gitignore, mẫu `.env.example`, chỉ cho công cụ phát hành, không mod nào đọc nó). Gửi bằng header `apikey`. Kiểm bằng `GET https://api.nexusmods.com/v1/users/validate.json`. Domain game luôn là `eldenring`.
- **Changelog hiện có:** `GET /v1/games/eldenring/mods/<mod_id>/changelogs.json`. Endpoint này **từng bỏ sót bản cũ có thật** (thiếu 1.0.0, 1.1.0, 1.1.1 của AutoRegen dù có trên trang). Thiếu một version trong phản hồi không chứng minh là chưa đăng: hỏi người dùng xác nhận trên trang thật trước khi xoá bất kỳ mục changelog cũ nào.
- **Upload (v3):** `POST /v3/uploads` trả URL PUT đã ký sẵn. URL đó ký cả `content-type` trong `X-Amz-SignedHeaders`, nhưng tài liệu (`docs/nexus-openapi.yaml`) không nhắc, chỉ nêu `Content-Disposition` và `Content-MD5`. Thiếu `Content-Type`, hay để curl mặc định `application/x-www-form-urlencoded` qua `--data-binary`, thì PUT hỏng với `SignatureDoesNotMatch` của Cloudflare R2, trông như lỗi ký phía server nhưng thật ra là thiếu header. Luôn gửi `-H "Content-Type: application/octet-stream"` (kiểm 2026-09-11, HTTP 200 ngay).
- **Tệp đầu tiên của một trang chưa có tệp nào:** `POST /v3/mod-files` với `mod_id` là ID nội bộ của trang (vd. SoulsTeleport `18610093304687`, FasterRevival `18610093304728`, AnimCancel `18610093304896`), các bản sau đi qua `createModFileVersion` của skill. Sau khi upload tệp đầu, trang vẫn ở `not_published`: người dùng tự công khai trên web.
- **Danh mục** (`GET /v1/games/eldenring.json` → `categories`, lấy 2026-10-12; chọn khi tạo mod trên Nexus, không nằm trong `nexus_page.bbcode`), tất cả nằm dưới gốc "Elden Ring" (1): 2 Miscellaneous, 3 Visuals, 4 User Interface, 5 Utilities, 6 Items, 7 Armour, 8 Weapons, 9 Gameplay, 10 Characters, 11 Magic. Mod của người dùng: **Gameplay (9)** cho autoregen, runemultiplier, weightmultiplier, passiverunes, spiritmultiplier, speedmultiplier, animcancel (chốt 2026-10-12); Weapons (8) cho risearcher; Utilities (5) cho soulsteleport; Miscellaneous (2) cho fasterrevival. Dùng bảng này, chỉ lấy lại khi nghi có danh mục mới.
- `sometweaks` không có mod ID: **không** dùng lại ID của `runemultiplier` dù sometweaks gộp cùng tính năng đó.

## Logo và thumbnail

Mẫu prompt (dán cho ChatGPT): `docs/logo_prompt.txt` (logo 1024x1024, nền trong suốt) và `docs/thumbnail_prompt.txt` (ảnh bìa 16:9, cảnh đầy đủ có nền, không chữ; tiêu đề thêm tay). Sửa mẫu ở đó, không sửa ghi chú nào khác. Bài học đã đúc vào mẫu v2 (2026-09-28):

- **Dấu hiệu chung của series** (2026-09-25): mọi logo có một biểu tượng vàng mảnh phát sáng kiểu Elden Ring **phía sau** chủ thể (vòng lớn gồm cung đan nhau, một vạch sáng dọc qua tâm, một vạch ngang ngắn gần đỉnh). Phải đòi rõ ràng là lớp nền; một prompt "ONLY <subject>, NO scenery" làm GPT bỏ mất nó.
- Nền **trong suốt** (PNG alpha), không phải nền đen; dặn thêm "không vẽ ô bàn cờ vào ảnh". Không chữ trong ảnh (AI vẽ chữ sai). Không nhân vật khi người dùng muốn biểu tượng thuần.
- Neo phong cách = "icon và artwork trong game Elden Ring" (sơn dầu, mờ, cũ, ánh sáng tiết chế). Khối phong cách dùng chung phải trung tính về chủ thể; chất liệu và kiến trúc chỉ nằm ở phần chủ thể của từng mod (bản nháp ghi "inventory item icon" bị chê là thiên về DropMultiplier).
- Ảnh đính kèm thường chỉ một (chủ thể), đôi khi nhiều để ghép. Ảnh 1 phải **cắt nguyên si**, giữ màu tối, cũ và ánh sáng trong game, không "chỉnh", làm sáng hay thêm viền màu. Cách chắc nhất: tự tách nền trước rồi để AI chỉ thêm biểu tượng.
- GPT bỏ qua "3 ảnh riêng" và trả một lưới 3 ô: yêu cầu **một ảnh mỗi tin nhắn**. Sửa nhỏ một kết quả tốt thì gửi "edit this image, change only X, keep everything else", tối đa 1 đến 2 lần (mỗi lần dựng lại ảnh trước nên càng trôi về kiểu "AI": sàn bóng ướt, quá nét, xoáy neon); sau đó mở **chat mới** từ ảnh gốc với prompt đầy đủ đã cập nhật.
- Cho kích thước bằng mốc cụ thể ("đỉnh huy hiệu ngang đỉnh mũ"), không dùng "đừng quá lớn". Ưu tiên câu khẳng định hơn danh sách "tránh". Giữ ánh sáng sát chủ thể để viền alpha sạch. Yêu cầu 16:9 thẳng (GPT trả được 1536x864); chỉ khi nó trả 3:2 mới cắt.
- Ví dụ khái niệm đã dùng: DropMultiplier = vật phẩm rơi phát sáng với mũi tên lên hoặc "x3", vàng hổ phách; WeightMultiplier = bóng giáp hay vũ khí với biểu tượng cân bằng hoặc lông vũ, bạc và xanh nhạt; SoulsTeleport = hai bóng Tarnished liên kết bởi xoáy sáng, một người tan thành hạt rồi hiện lại bên cạnh, vàng nhạt và xanh ma.
