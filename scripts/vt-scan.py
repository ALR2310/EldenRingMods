#!/usr/bin/env python3
"""Quét file bằng VirusTotal (API v3) và in số hãng gắn cờ.

Dùng cho zip phát hành trước khi đăng lên Nexus (skill `deploy-nexus-mod`) để
biết trước hãng nào báo nhầm. Cần `VT_API_KEY` trong `.env` ở gốc repo (khoá
miễn phí của tài khoản VirusTotal; gói miễn phí ~4 yêu cầu/phút, 500/ngày,
chỉ cho mục đích phi thương mại).

    python scripts/vt-scan.py builds/animcancel/AnimCancel-1.1.1.zip
    python scripts/vt-scan.py new.zip old.zip           # so sánh nhiều file
    python scripts/vt-scan.py new.zip --no-upload       # chỉ tra hash, không tải lên
    python scripts/vt-scan.py new.zip --fail-above 5    # exit 1 nếu > 5 hãng gắn cờ

Mỗi file: tra theo SHA-256 trước (không tải gì lên); chưa có trên VirusTotal thì
tải lên (**file trở thành công khai trên VirusTotal**; bản phát hành vốn đã
public, nhưng build thử chưa phát hành cũng sẽ bị chia sẻ, dùng --no-upload để
tránh) rồi chờ phân tích xong. Mã thoát: 0 = đã quét xong, 1 = vượt
--fail-above, 2 = lỗi (thiếu khoá, mạng, không có kết quả).
"""

import argparse
import hashlib
import json
import os
import sys
import time
import urllib.error
import urllib.request
import uuid

API = "https://www.virustotal.com/api/v3"
GUI = "https://www.virustotal.com/gui/file/"
POLL_SECONDS = 15
POLL_MAX = 24  # 6 phút
MAX_DIRECT_UPLOAD = 32 * 1024 * 1024


def load_key():
    key = os.environ.get("VT_API_KEY", "").strip()
    if key:
        return key
    env = os.path.join(os.path.dirname(os.path.abspath(__file__)), os.pardir, ".env")
    try:
        with open(env, encoding="utf-8") as f:
            for line in f:
                name, _, value = line.strip().partition("=")
                if name.strip() == "VT_API_KEY":
                    return value.strip().strip('"').strip("'")
    except OSError:
        pass
    return ""


def request(key, method, path, body=None, headers=None):
    req = urllib.request.Request(API + path, data=body, method=method)
    req.add_header("x-apikey", key)
    req.add_header("accept", "application/json")
    for name, value in (headers or {}).items():
        req.add_header(name, value)
    try:
        with urllib.request.urlopen(req, timeout=120) as resp:
            return resp.status, json.load(resp)
    except urllib.error.HTTPError as err:
        try:
            return err.code, json.load(err)
        except ValueError:
            return err.code, {}


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def upload(key, path):
    """Tải file lên (<= 32 MB), trả về id của lần phân tích."""
    size = os.path.getsize(path)
    if size > MAX_DIRECT_UPLOAD:
        sys.exit(f"{path}: {size} byte, lớn hơn 32 MB (cần upload_url, chưa hỗ trợ)")
    boundary = uuid.uuid4().hex
    name = os.path.basename(path)
    with open(path, "rb") as f:
        content = f.read()
    body = (
        f"--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{name}\"\r\n"
        "Content-Type: application/octet-stream\r\n\r\n"
    ).encode() + content + f"\r\n--{boundary}--\r\n".encode()
    status, data = request(
        key, "POST", "/files", body, {"content-type": f"multipart/form-data; boundary={boundary}"}
    )
    if status != 200:
        raise RuntimeError(f"tải lên thất bại (HTTP {status}): {data}")
    return data["data"]["id"]


def wait_for_analysis(key, analysis_id):
    for _ in range(POLL_MAX):
        status, data = request(key, "GET", f"/analyses/{analysis_id}")
        if status == 429:
            time.sleep(POLL_SECONDS)
            continue
        if status != 200:
            raise RuntimeError(f"đọc kết quả thất bại (HTTP {status}): {data}")
        attrs = data["data"]["attributes"]
        if attrs.get("status") == "completed":
            return
        time.sleep(POLL_SECONDS)
    raise RuntimeError("hết thời gian chờ phân tích (thử lại sau vài phút bằng cùng lệnh)")


def lookup(key, sha256):
    """Kết quả phân tích mới nhất của hash, hoặc None nếu VirusTotal chưa biết."""
    status, data = request(key, "GET", f"/files/{sha256}")
    if status == 404:
        return None
    if status != 200:
        raise RuntimeError(f"tra hash thất bại (HTTP {status}): {data}")
    return data["data"]["attributes"]


def scan(key, path, allow_upload):
    sha256 = sha256_of(path)
    attrs = lookup(key, sha256)
    source = "đã có trên VirusTotal"
    if attrs is None:
        if not allow_upload:
            return {"path": path, "sha256": sha256, "known": False}
        print(f"{path}: chưa có trên VirusTotal, đang tải lên...", file=sys.stderr)
        wait_for_analysis(key, upload(key, path))
        # Hash vừa có nên tra lại; chờ một nhịp cho kết quả hiện lên.
        for _ in range(5):
            attrs = lookup(key, sha256)
            if attrs and attrs.get("last_analysis_results"):
                break
            time.sleep(5)
        if not attrs:
            raise RuntimeError("không đọc được kết quả sau khi tải lên")
        source = "vừa tải lên"
    stats = attrs.get("last_analysis_stats", {})
    results = attrs.get("last_analysis_results", {})
    flagged = {
        engine: r.get("result") or r.get("category")
        for engine, r in results.items()
        if r.get("category") in ("malicious", "suspicious")
    }
    total = sum(stats.get(k, 0) for k in ("malicious", "suspicious", "undetected", "harmless"))
    return {
        "path": path,
        "sha256": sha256,
        "known": True,
        "source": source,
        "malicious": stats.get("malicious", 0),
        "suspicious": stats.get("suspicious", 0),
        "total": total,
        "flagged": flagged,
    }


def report(result):
    print(f"== {result['path']}")
    print(f"   SHA-256: {result['sha256']}")
    if not result["known"]:
        print("   VirusTotal chưa biết file này (đã dùng --no-upload, không tải lên).")
        return
    detections = result["malicious"] + result["suspicious"]
    extra = f" (+{result['suspicious']} suspicious)" if result["suspicious"] else ""
    print(f"   VirusTotal: {result['malicious']}/{result['total']} malicious{extra}  [{result['source']}]")
    for engine, label in sorted(result["flagged"].items()):
        print(f"     - {engine}: {label}")
    if not detections:
        print("     (không có hãng nào gắn cờ)")
    print(f"   {GUI}{result['sha256']}")


def main():
    # Windows consoles default to cp1252 and choke on Vietnamese text.
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", nargs="+", help="file cần quét (zip phát hành hoặc DLL)")
    parser.add_argument("--no-upload", action="store_true", help="chỉ tra hash, không tải file lên")
    parser.add_argument("--fail-above", type=int, default=None, metavar="N", help="exit 1 nếu số hãng gắn cờ > N")
    args = parser.parse_args()

    key = load_key()
    if not key:
        print("Thiếu VT_API_KEY (biến môi trường hoặc .env ở gốc repo).", file=sys.stderr)
        return 2
    worst = 0
    for path in args.files:
        if not os.path.isfile(path):
            print(f"Không thấy file: {path}", file=sys.stderr)
            return 2
        try:
            result = scan(key, path, not args.no_upload)
        except (RuntimeError, urllib.error.URLError) as err:
            print(f"{path}: {err}", file=sys.stderr)
            return 2
        report(result)
        if result["known"]:
            worst = max(worst, result["malicious"] + result["suspicious"])
    if args.fail_above is not None and worst > args.fail_above:
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
