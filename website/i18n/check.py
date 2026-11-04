"""Checks the built site: Portuguese left in the translated pages, and broken local links and paths.

  python website/i18n/check.py

Portuguese: every text node (outside code, scripts and styles) and every attribute people read, in each
website/<lang>/ page and in i18n/<lang>.js, is searched for words and endings only Portuguese has here.
Links: every relative href/src in every page (Portuguese too) must reach a file, and every #anchor an id in that page.
"""

import html
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import build  # noqa: E402

ROOT = build.ROOT
def texts(tree):
    def go(el):
        for c in el.children:
            if isinstance(c, build.El):
                if c.tag in ("script", "style", "code", "svg"):
                    continue
                for a in build.TEXT_ATTRS + ("content",):
                    v = c.attr(a)
                    if v and (a != "content" or (c.attr("name") or c.attr("property")) in build.META_KEYS):
                        yield f"{c.tag}[{a}]", html.unescape(v)
                yield from go(c)
            elif not isinstance(c, build.Comment) and c.strip():
                yield el.tag, html.unescape(c)
    yield from go(tree)


def target_ok(page, ref, ids):
    if re.match(r"^(?:[a-z]+:|//)", ref):
        return None
    path, _, frag = ref.partition("#")
    path = path.split("?")[0]
    f = (page.parent / path).resolve() if path else page
    if f.is_dir():
        f = f / "index.html"
    if not f.exists():
        return f"{ref}: no file {f.relative_to(ROOT) if f.is_relative_to(ROOT) else f}"
    if frag and f.suffix == ".html":
        if f not in ids:
            ids[f] = set(re.findall(r'\sid="([^"]+)"', f.read_text(encoding="utf-8")))
        if frag not in ids[f]:
            return f"{ref}: no id #{frag} in {f.relative_to(ROOT).as_posix()}"
    return None


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    problems, ids, n = [], {}, 0
    for code in build.L:
        for p in build.PAGES:
            f = ROOT / ("" if code == "pt" else code) / f"{p}.html"
            rel = f.relative_to(ROOT).as_posix()
            if not f.exists():
                problems.append(f"{rel}: missing")
                continue
            src = f.read_text(encoding="utf-8")
            tree = build.parse(src)
            n += 1
            if code != "pt":
                for where, t in texts(tree):
                    m = build.portuguese(code, t)
                    if m:
                        problems.append(f"{rel}: Portuguese '{m}' in {where}: {t.strip()[:120]}")
                if f'<script src="../i18n/{code}.js" defer></script>' not in src:
                    problems.append(f"{rel}: does not load i18n/{code}.js")
                if f'<html lang="{build.L[code]["tag"]}"' not in src:
                    problems.append(f"{rel}: wrong <html lang>")
            for el in build.walk(tree):
                for a in ("href", "src"):
                    v = el.attr(a)
                    if v:
                        bad = target_ok(f, html.unescape(v), ids)
                        if bad:
                            problems.append(f"{rel}: <{el.tag} {a}> {bad}")
    for code in build.L:
        js = ROOT / "i18n" / f"{code}.js"
        if not js.exists():
            problems.append(f"i18n/{code}.js: missing")
        elif code != "pt":
            m = build.portuguese(code, js.read_text(encoding="utf-8"))
            if m:
                problems.append(f"i18n/{code}.js: Portuguese '{m}'")
    print("\n".join(problems) if problems else f"ok: {n} pages, no Portuguese left in translations, every local link and path resolves")
    sys.exit(1 if problems else 0)


if __name__ == "__main__":
    main()
