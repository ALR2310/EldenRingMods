"""Applies the house style to Nexus page files (`nexus_page.bbcode`).

    python scripts/nexus_style.py <file>...        rewrite the files in place
    python scripts/nexus_style.py --check <file>...  only report what would change

The style (decided 2026-10-13, first on animcancel):
- Section headings `[size=4][b]Title[/b][/size]`: a gold gradient, one colour
  tag per letter, light in the middle and warmer at both ends.
- The Disclaimer line: the same, in red / coral (so it still reads as a warning).
- Support: a sentence with a Ko-fi and a PayPal link (fallback if the images
  break), then the two buttons side by side. The Support section is the last
  one of a page, so everything after its heading is replaced.

Running it again changes nothing: a heading that already has colour tags, a
Disclaimer already in a gradient and a Support block that already mentions
PayPal are left as they are.
"""
import re
import sys

GOLD = ((0xC0, 0x86, 0x0A), (0xF6, 0xDE, 0x92))   # ends, centre
RED = ((0xD9, 0x3A, 0x2B), (0xFF, 0x9C, 0x8C))

KOFI = "https://ko-fi.com/alr2310"
PAYPAL = "https://www.paypal.com/paypalme/alr2310"
SUPPORT_BLOCK = (
    "If you enjoy this mod and want to support my work, you can buy me a coffee via "
    f"[url={KOFI}]Ko-fi[/url] or [url={PAYPAL}]PayPal[/url]! Any donation is greatly appreciated.\n"
    "\n"
    f"[url={KOFI}][img width=203]https://storage.ko-fi.com/cdn/kofi1.png?v=6[/img][/url]   "
    f"[url={PAYPAL}][img]https://www.paypalobjects.com/webstatic/en_US/i/buttons/PP_logo_h_200x51.png[/img][/url]\n"
)

OLD_DISCLAIMER = "[size=4][b][color=#E74C3C]Disclaimer: Only use in offline mode with EAC disabled![/color][/b][/size]"
DISCLAIMER_TEXT = "Disclaimer: Only use in offline mode with EAC disabled!"
# A plain heading: no tag inside, so one already coloured never matches.
HEADING = re.compile(r"\[size=4\]\[b\]([^\[\]]+)\[/b\]\[/size\]")


def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def gradient(text, palette):
    """One colour tag per letter (not per space); light in the middle."""
    ends, centre = palette
    count = sum(1 for c in text if not c.isspace())
    out, k = [], 0
    for c in text:
        if c.isspace():
            out.append(c)
            continue
        t = k / (count - 1) if count > 1 else 0.5
        r, g, b = lerp(ends, centre, 1 - abs(2 * t - 1))
        out.append(f"[color=#{r:02X}{g:02X}{b:02X}]{c}[/color]")
        k += 1
    return "".join(out)


def plain(line):
    """The visible text of a BBCode line."""
    return re.sub(r"\[/?[^\]]*\]", "", line).strip()


def restyle(text):
    text = text.replace(
        OLD_DISCLAIMER,
        "[size=4][b]" + gradient(DISCLAIMER_TEXT, RED) + "[/b][/size]",
    )
    text = HEADING.sub(lambda m: "[size=4][b]" + gradient(m.group(1), GOLD) + "[/b][/size]", text)

    lines = text.split("\n")
    for i, line in enumerate(lines):
        if line.startswith("[size=4]") and plain(line) == "Support":
            old = "\n".join(lines[i + 1:])
            if "paypal" in old.lower():
                break  # already the new block
            if "ko-fi" not in old.lower() or len(old.strip().split("\n")) > 6:
                raise SystemExit(f"unexpected Support block, not touched:\n{old}")
            text = "\n".join(lines[: i + 1]) + "\n" + SUPPORT_BLOCK
            break
    return text


def main():
    args = sys.argv[1:]
    check = "--check" in args
    files = [a for a in args if a != "--check"]
    if not files:
        raise SystemExit(__doc__)
    for path in files:
        with open(path, encoding="utf-8", newline="") as f:
            raw = f.read()
        crlf = "\r\n" in raw
        old = raw.replace("\r\n", "\n")
        new = restyle(old)
        status = "unchanged" if new == old else ("would change" if check else "updated")
        if new != old and not check:
            with open(path, "w", encoding="utf-8", newline="") as f:
                f.write(new.replace("\n", "\r\n") if crlf else new)
        print(f"{status:13} {path}")


if __name__ == "__main__":
    main()
