#!/usr/bin/env python3
"""Scan files with VirusTotal (API v3) and print how many engines flag them.

Run it on a release zip before publishing, to see which engines raise false positives.
Needs `VT_API_KEY` in the environment or in `.env` at the repo root (the free key allows
about 4 requests per minute and 500 per day, for non-commercial use).

    python scripts/vt-scan.py builds/<crate>/<Mod>-<version>.zip
    python scripts/vt-scan.py new.zip old.zip           # compare several files
    python scripts/vt-scan.py new.zip --no-upload       # look up the hash only
    python scripts/vt-scan.py new.zip --fail-above 5    # exit 1 if more than 5 engines flag it

Each file is looked up by SHA-256 first, which uploads nothing. If VirusTotal does not know the
file it is uploaded, and an uploaded file becomes public on VirusTotal. Release builds are public
anyway, but an unreleased test build would be shared too; use --no-upload to avoid that.
Exit codes: 0 = scanned, 1 = above --fail-above, 2 = error (missing key, network, no result).
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
POLL_MAX = 24  # 6 minutes
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
    """Upload a file (up to 32 MB) and return the analysis id."""
    size = os.path.getsize(path)
    if size > MAX_DIRECT_UPLOAD:
        sys.exit(f"{path}: {size} bytes, larger than 32 MB (needs an upload_url, not supported)")
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
        raise RuntimeError(f"upload failed (HTTP {status}): {data}")
    return data["data"]["id"]


def wait_for_analysis(key, analysis_id):
    for _ in range(POLL_MAX):
        status, data = request(key, "GET", f"/analyses/{analysis_id}")
        if status == 429:
            time.sleep(POLL_SECONDS)
            continue
        if status != 200:
            raise RuntimeError(f"reading the result failed (HTTP {status}): {data}")
        attrs = data["data"]["attributes"]
        if attrs.get("status") == "completed":
            return
        time.sleep(POLL_SECONDS)
    raise RuntimeError("timed out waiting for the analysis (run the same command again in a few minutes)")


def lookup(key, sha256):
    """Latest analysis of the hash, or None if VirusTotal does not know it."""
    status, data = request(key, "GET", f"/files/{sha256}")
    if status == 404:
        return None
    if status != 200:
        raise RuntimeError(f"hash lookup failed (HTTP {status}): {data}")
    return data["data"]["attributes"]


def scan(key, path, allow_upload):
    sha256 = sha256_of(path)
    attrs = lookup(key, sha256)
    source = "already on VirusTotal"
    if attrs is None:
        if not allow_upload:
            return {"path": path, "sha256": sha256, "known": False}
        print(f"{path}: not on VirusTotal yet, uploading...", file=sys.stderr)
        wait_for_analysis(key, upload(key, path))
        # Look the hash up again; the results can take a moment to appear.
        for _ in range(5):
            attrs = lookup(key, sha256)
            if attrs and attrs.get("last_analysis_results"):
                break
            time.sleep(5)
        if not attrs:
            raise RuntimeError("no result after the upload")
        source = "just uploaded"
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
        print("   VirusTotal does not know this file (--no-upload, nothing was uploaded).")
        return
    detections = result["malicious"] + result["suspicious"]
    extra = f" (+{result['suspicious']} suspicious)" if result["suspicious"] else ""
    print(f"   VirusTotal: {result['malicious']}/{result['total']} malicious{extra}  [{result['source']}]")
    for engine, label in sorted(result["flagged"].items()):
        print(f"     - {engine}: {label}")
    if not detections:
        print("     (no engine flags it)")
    print(f"   {GUI}{result['sha256']}")


def main():
    # Windows consoles default to cp1252 and choke on non-ASCII engine labels.
    for stream in (sys.stdout, sys.stderr):
        stream.reconfigure(encoding="utf-8", errors="replace")
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("files", nargs="+", help="files to scan (a release zip or a DLL)")
    parser.add_argument("--no-upload", action="store_true", help="look up the hash only, do not upload")
    parser.add_argument("--fail-above", type=int, default=None, metavar="N", help="exit 1 if more than N engines flag a file")
    args = parser.parse_args()

    key = load_key()
    if not key:
        print("Missing VT_API_KEY (environment variable or .env at the repo root).", file=sys.stderr)
        return 2
    worst = 0
    for path in args.files:
        if not os.path.isfile(path):
            print(f"File not found: {path}", file=sys.stderr)
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
