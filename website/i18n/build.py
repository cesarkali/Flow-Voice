"""Builds the FlowVoice site in every language.

The Portuguese pages in website/ are the source. For each other language this writes website/<lang>/<page>.html
from the table i18n/src/pages/<lang>.json, and every page (the Portuguese ones too) gets the hreflang links and
the language picker. It also writes i18n/<lang>.js (texts for site.js, from i18n/src/demo/<lang>.json),
sitemap.xml, and i18n/src/units.json: every Portuguese text that needs translating, with where it appears.

A unit is the outermost element holding text of its own (p, li, dt, h2, a, button, span…). Tags inside it become
markers, so the translation can move them but never lose them: <b>x</b> is [1]x[/1], and an element with nothing
inside (img, svg, the caret) is [2/]. Attributes people read (alt, title, aria-label, placeholder, data-l), the
meta descriptions and titles, and the JSON-LD description are units too. A table maps each unit, as written in
Portuguese with its spaces collapsed, to its translation; change the Portuguese and the build names what to redo.

  python website/i18n/build.py           build; fails, listing them, if any unit is missing or broken
  python website/i18n/build.py --check   the same checks, and fails if a written file is out of date
"""

import html
import io
import json
import re
import sys
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent  # website/
SRC = ROOT / "i18n" / "src"
SITE = "https://flowvoice.caliberda.com.br"
PAGES = ["index", "guia", "novidades", "privacidade"]

# code, <html lang>/hreflang, og:locale, name in the language itself (as in the app), short code, "Language"
LANGS = [
    ("pt", "pt-BR", "pt_BR", "Português", "PT", "Idioma"),
    ("en", "en", "en_US", "English", "EN", "Language"),
    ("es", "es", "es_ES", "Español", "ES", "Idioma"),
    ("fr", "fr", "fr_FR", "Français", "FR", "Langue"),
    ("de", "de", "de_DE", "Deutsch", "DE", "Sprache"),
    ("ja", "ja", "ja_JP", "日本語", "JA", "言語"),
    ("zh", "zh-Hans", "zh_CN", "简体中文", "ZH", "语言"),
    ("ru", "ru", "ru_RU", "Русский", "RU", "Язык"),
    ("hi", "hi", "hi_IN", "हिन्दी", "HI", "भाषा"),
    ("bn", "bn", "bn_BD", "বাংলা", "BN", "ভাষা"),
]
L = {l[0]: dict(zip(("code", "tag", "locale", "name", "short", "word"), l)) for l in LANGS}
X_DEFAULT = "en"

TEXT_ATTRS = ("alt", "title", "aria-label", "placeholder", "data-l")
META_KEYS = ("description", "og:title", "og:description", "og:image:alt", "twitter:title", "twitter:description", "twitter:image:alt")
LOCAL_PATHS = re.compile(r'(\s(?:href|src)=")((?:assets/|i18n/|styles\.css|pages\.css|site\.js)[^"]*")')
# a unit made only of these (and numbers, punctuation, markers) stays as it is in every language
KEEP = sorted(
    """FlowVoice Groq groq openai/gpt-oss-20b Gemini Google Mistral OpenRouter Tavily OpenAI Ollama GitHub Sponsors Pix
    Outlook Slack ChatGPT Claude Notion Gmail Word WhatsApp Teams Discord VS Code Chrome Edge Windows Ctrl Shift Alt AltGr
    Esc esc Júlio Caliberda Rust Tauri Svelte Python PySide6 Tiny Base Small Large v3 Turbo Distil-Whisper pt-BR MB GB
    macOS Vercel Hugging Face Whisper K Y U V x""".split(),
    key=len, reverse=True,
)
KEEP_RE = re.compile(r"(?<![\w-])(?:" + "|".join(re.escape(w) for w in KEEP) + r")(?![\w-])")
LETTER = re.compile(r"[^\W\d_]")
# Words and endings that, among the site's languages, only Portuguese has (with the Spanish words that share a
# spelling taken out for Spanish), to catch text left untranslated. Names that contain Portuguese are allowed.
PT_WORDS = re.compile(
    r"[ãõ]|ção|ções|\b(?:você|vocês|não|também|está|estão|são|página|grátis|ditado|cápsula|chaves?|então|isso|até|já|só|"
    r"dá|pelo|pela|muito|aqui|agora|depois|configurações|reunião|quinta|né|pra|"
    r"Baixar|Novidades|Privacidade|Guia|Dúvidas|Geral|Atalhos|Provedores|Colado|Substituir|Traduzir)\b",
    re.I,
)
PT_ALSO = {"es": {"está", "página", "cápsula"}}
PT_NAMES = re.compile(r"Português|Júlio Caliberda|Distil-Whisper pt-BR|br\.com\.caliberda\.flowvoice|caliberda\.com\.br")


def portuguese(lang, text):
    """The first Portuguese-only word in a text meant to be in another language, or None."""
    for m in PT_WORDS.finditer(PT_NAMES.sub("", text)):
        if m.group(0).lower() not in PT_ALSO.get(lang, ()):
            return m.group(0)
    return None
MARK = re.compile(r"\[(/?)(\d+)(/?)\]")

# ---------- a small HTML tree that serializes back to exactly the source ----------
TOKEN = re.compile(
    r"<!--.*?-->|<![^>]*>|</([a-zA-Z][\w:-]*)\s*>"
    r"|<([a-zA-Z][\w:-]*)((?:\s+[^\s\"'>/=]+(?:\s*=\s*(?:\"[^\"]*\"|'[^']*'|[^\s\"'>]+))?)*)\s*(/?)>",
    re.S,
)
VOID = {"area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track", "wbr"}
RAW = {"script", "style"}


class El:
    def __init__(self, tag, start, attrs):
        self.tag, self.start, self.attrs, self.children, self.end = tag, start, attrs, [], ""

    def html(self):
        return self.start + "".join(c if isinstance(c, str) else c.html() for c in self.children) + self.end

    def attr(self, name):
        m = re.search(r'\s' + re.escape(name) + r'="([^"]*)"', self.attrs)
        return m.group(1) if m else None


class Comment(str):
    pass


def parse(src):
    root = El("#root", "", "")
    stack = [root]
    pos = 0
    while pos < len(src):
        m = TOKEN.search(src, pos)
        if not m:
            stack[-1].children.append(src[pos:])
            break
        if m.start() > pos:
            stack[-1].children.append(src[pos:m.start()])
        raw = m.group(0)
        pos = m.end()
        if raw.startswith("<!"):
            stack[-1].children.append(Comment(raw))
        elif m.group(1):  # end tag
            tag = m.group(1).lower()
            if stack[-1].tag != tag:
                line = src.count("\n", 0, m.start()) + 1
                raise SystemExit(f"HTML: </{tag}> closes <{stack[-1].tag}> at line {line}")
            stack[-1].end = raw
            stack.pop()
        else:
            tag = m.group(2).lower()
            el = El(tag, raw, m.group(3))
            stack[-1].children.append(el)
            if m.group(4) or tag in VOID:
                continue
            if tag in RAW:
                close = src.lower().index(f"</{tag}>", pos)
                el.children.append(src[pos:close])
                el.end = src[close:close + len(tag) + 3]
                pos = close + len(tag) + 3
                continue
            stack.append(el)
    if len(stack) != 1:
        raise SystemExit(f"HTML: <{stack[-1].tag}> never closed")
    out = root.html()
    assert out == src, "parser does not round-trip"
    return root


def walk(el):
    for c in el.children:
        if isinstance(c, El):
            yield c
            yield from walk(c)


def own_text(el):
    return any(isinstance(c, str) and not isinstance(c, Comment) and c.strip() for c in el.children)


def deep_text(el):
    return any(c.strip() if isinstance(c, str) and not isinstance(c, Comment) else isinstance(c, El) and deep_text(c)
               for c in el.children)


def text_units(el, out):
    for c in el.children:
        if not isinstance(c, El) or c.tag in RAW or c.tag == "svg":
            continue
        if own_text(c):
            out.append(c)
        else:
            text_units(c, out)
    return out


def to_marked(el):
    """The element's content with inner tags as markers; tags[n] holds what each marker stands for."""
    tags, parents, n = {}, {}, 0

    def go(children, parent):
        nonlocal n
        parts = []
        for c in children:
            if isinstance(c, Comment) or not isinstance(c, El):
                parts.append(str(c))
                continue
            n += 1
            k = n
            parents[k] = parent
            if c.tag in VOID or c.tag == "svg" or not deep_text(c):
                tags[k] = c.html()
                parts.append(f"[{k}/]")
            else:
                tags[k] = (c.start, c.end)
                parts.append(f"[{k}]" + go(c.children, k) + f"[/{k}]")
        return "".join(parts)

    return go(el.children, 0), tags, parents


def norm(s):
    return re.sub(r"\s+", " ", s).strip()


def keeps(s):
    """True when the text has nothing to translate (names, keys, numbers)."""
    return not LETTER.search(KEEP_RE.sub("", MARK.sub("", html.unescape(s))))


def check_markers(t, parents):
    """Problems with the markers of a translation, against the source's ([n]…[/n] pairs, [n/], nesting)."""
    stack, seen = [0], []
    for m in MARK.finditer(t):
        close, k, self_ = m.group(1) == "/", int(m.group(2)), m.group(3) == "/"
        if k not in parents:
            return f"unknown marker {m.group(0)}"
        if close:
            if stack[-1] != k:
                return f"{m.group(0)} closes the wrong marker"
            stack.pop()
            continue
        if parents[k] != stack[-1]:
            return f"{m.group(0)} moved in or out of another marker"
        seen.append(k)
        if not self_:
            stack.append(k)
    if len(stack) != 1:
        return f"[{stack[-1]}] never closed"
    if sorted(seen) != sorted(parents):
        missing = sorted(set(parents) - set(seen))
        return "missing marker " + ", ".join(f"[{k}]" for k in missing) if missing else "a marker appears twice"
    return None


def safe_text(t):
    if "<" in t:
        return None
    return re.sub(r"&(?!#?\w+;)", "&amp;", t)


# ---------- pages ----------
BLOCK = re.compile(r"<!-- i18n:(\w+) -->.*?<!-- /i18n:\1 -->\n[ \t]*", re.S)


def page_url(lang, page):
    return SITE + "/" + ("" if lang == "pt" else lang + "/") + ("" if page == "index" else page)


def href(frm, to, page):
    if frm == to:
        return "./" if page == "index" else page + ".html"
    path = ("" if to == "pt" else to + "/") + ("" if page == "index" else page + ".html")
    return ("" if frm == "pt" else "../") + path or "./"


GLOBE = ('<svg viewBox="0 0 24 24" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="1.7" '
         'stroke-linecap="round"><circle cx="12" cy="12" r="8.6"/><path d="M3.6 12h16.8M12 3.4c2.3 2.4 3.5 5.3 3.5 '
         '8.6s-1.2 6.2-3.5 8.6c-2.3-2.4-3.5-5.3-3.5-8.6s1.2-6.2 3.5-8.6z"/></g></svg>')


def alternates(page):
    lines = [f'<link rel="alternate" hreflang="{L[c]["tag"]}" href="{page_url(c, page)}">' for c in L]
    lines.append(f'<link rel="alternate" hreflang="x-default" href="{page_url(X_DEFAULT, page)}">')
    return "<!-- i18n:alternates -->\n" + "\n".join(lines) + "\n<!-- /i18n:alternates -->\n"


def picker(lang, page, indent):
    me = L[lang]
    items = []
    for c, d in L.items():
        cur = ' aria-current="true"' if c == lang else ""
        items.append(f'{indent}    <li><a href="{href(lang, c, page)}" hreflang="{d["tag"]}" lang="{d["tag"]}" '
                     f'data-lang="{c}"{cur}>{d["name"]}</a></li>')
    return (f"<!-- i18n:picker -->\n{indent}<details class=\"lang\">\n"
            f"{indent}  <summary title=\"{me['word']}\" aria-label=\"{me['word']}: {me['name']}\">{GLOBE}<span>{me['short']}</span></summary>\n"
            f"{indent}  <ul class=\"lang-menu\">\n" + "\n".join(items) + "\n"
            f"{indent}  </ul>\n{indent}</details>\n{indent}<!-- /i18n:picker -->\n{indent}")


def nav_langs(lang, page, indent):
    """The same links at the foot of the menu, for phones, where the header has no room for the picker."""
    me = L[lang]
    links = [f'{indent}    <a href="{href(lang, c, page)}" hreflang="{d["tag"]}" lang="{d["tag"]}" data-lang="{c}"'
             + (' aria-current="true"' if c == lang else "") + f'>{d["name"]}</a>' for c, d in L.items()]
    return (f"<!-- i18n:navlang -->\n{indent}  <div class=\"nav-lang\" role=\"group\" aria-label=\"{me['word']}\">\n"
            + "\n".join(links) + f"\n{indent}  </div>\n{indent}<!-- /i18n:navlang -->\n{indent}")


def add_blocks(src, lang, page):
    canon = re.search(r'<link rel="canonical" href="[^"]*">\n', src)
    src = src[:canon.end()] + alternates(page) + src[canon.end():]
    nav = src.index('<nav class="nav" id="nav"')
    m = re.compile(r"\n([ \t]*)</nav>").search(src, nav)
    at = m.start(1) + len(m.group(1))
    src = src[:at] + nav_langs(lang, page, m.group(1)) + src[at:]
    m = re.search(r'\n([ \t]*)<button class="theme-btn"', src)
    at = m.start(1) + len(m.group(1))
    return src[:at] + picker(lang, page, m.group(1)) + src[at:]


class Units:
    """Every translatable text in the Portuguese pages, with where it appears."""

    def __init__(self):
        self.items = {}

    def add(self, key, kind, where, parents=None):
        it = self.items.setdefault(key, {"kind": kind, "where": [], "markers": parents or {}})
        if where not in it["where"]:
            it["where"].append(where)


def describe(el):
    cls = el.attr("class")
    return el.tag + ("." + cls.split()[0] if cls else "")


def collect(page, tree, units):
    for el in text_units(tree, []):
        if el.tag == "code" or el.attr("translate") == "no":
            continue
        marked, _, parents = to_marked(el)
        key = norm(marked)
        if not keeps(key):
            units.add(key, "text", f"{page}: {describe(el)}", parents)
    for el in walk(tree):
        for a in TEXT_ATTRS:
            v = el.attr(a)
            if v and not keeps(v):
                units.add(norm(v), "attr", f"{page}: {describe(el)}[{a}]")
        if el.tag == "meta" and (el.attr("name") or el.attr("property")) in META_KEYS:
            units.add(norm(el.attr("content")), "meta", f"{page}: meta {el.attr('name') or el.attr('property')}")
        if el.tag == "script" and el.attr("type") == "application/ld+json":
            units.add(json.loads(el.children[0])["description"], "jsonld", f"{page}: JSON-LD description")


def translate(page, src, lang, table, problems):
    tree = parse(src)
    used = set()

    def tr(key, parents=None):
        t = table.get(key)
        if t is None:
            problems.append(f"{lang} missing: {key}")
            return None
        used.add(key)
        if parents is not None:
            bad = check_markers(t, parents)
            if bad:
                problems.append(f"{lang} {bad}: {key}  ->  {t}")
                return None
        if safe_text(t) is None:
            problems.append(f"{lang} has '<': {key}  ->  {t}")
            return None
        return safe_text(t)

    for el in walk(tree):
        start = el.start
        for a in TEXT_ATTRS:
            v = el.attr(a)
            if v and not keeps(v):
                t = tr(norm(v))
                if t is not None:
                    start = start.replace(f' {a}="{v}"', f' {a}="{t.replace(chr(34), "&quot;")}"', 1)
        if el.tag == "meta" and (el.attr("name") or el.attr("property")) in META_KEYS:
            v = el.attr("content")
            t = tr(norm(v))
            if t is not None:
                start = start.replace(f' content="{v}"', f' content="{t.replace(chr(34), "&quot;")}"', 1)
        if el.tag == "meta" and el.attr("property") == "og:locale":
            start = start.replace('content="pt_BR"', f'content="{L[lang]["locale"]}"')
        if el.tag == "meta" and el.attr("property") == "og:url" or el.tag == "link" and el.attr("rel") == "canonical":
            start = re.sub(r'="https://[^"]*"', f'="{page_url(lang, page)}"', start)
        if el.tag == "html":
            start = start.replace('lang="pt-BR"', f'lang="{L[lang]["tag"]}"')
        if el.tag == "script" and el.attr("src") == "i18n/pt.js":
            start = start.replace("i18n/pt.js", f"i18n/{lang}.js")
        el.start = LOCAL_PATHS.sub(r"\1../\2", start)
        if el.tag == "script" and el.attr("type") == "application/ld+json":
            raw = el.children[0]
            d = json.loads(raw)
            t = tr(d["description"])
            if t is not None:
                raw = raw.replace(json.dumps(d["description"], ensure_ascii=False), json.dumps(html.unescape(t), ensure_ascii=False))
            raw = raw.replace(f'"url": "{d["url"]}"', f'"url": "{page_url(lang, "index")}"')
            el.children = [raw]

    # units last: their markers keep the inner tags as rewritten above (paths, attributes)
    for el in text_units(tree, []):
        if el.tag == "code" or el.attr("translate") == "no":
            continue
        marked, tags, parents = to_marked(el)
        key = norm(marked)
        if keeps(key):
            continue
        t = tr(key, parents)
        if t is None:
            continue
        lead = marked[: len(marked) - len(marked.lstrip())]
        trail = marked[len(marked.rstrip()):]

        def back(m):
            close, k, self_ = m.group(1) == "/", int(m.group(2)), m.group(3) == "/"
            return tags[k] if self_ else tags[k][1] if close else tags[k][0]

        el.children = [lead + MARK.sub(back, t) + trail]
    return tree.html(), used


# ---------- texts for site.js ----------
def shape(v):
    if isinstance(v, dict):
        return {k: shape(x) for k, x in v.items()}
    if isinstance(v, list):
        return [shape(x) for x in v]
    return type(v).__name__


def check_demo(lang, d, pt, problems):
    a, b = shape({k: v for k, v in d.items() if k != "overlay"}), shape({k: v for k, v in pt.items() if k != "overlay"})
    if a != b:
        problems.append(f"{lang} demo: keys or types differ from pt.json")
        return
    if d["sep"] not in (" ", ""):
        problems.append(f"{lang} demo: sep must be a space or empty")
    ov = d["overlay"]
    if set(ov) != set(pt["overlay"]):
        problems.append(f"{lang} demo: overlay keys differ from pt.json")
        return
    words = ov["said"].split(" ")
    if any(not k.isdigit() or int(k) >= len(words) for k in ov["fix"]):
        problems.append(f"{lang} demo: overlay.fix has an index outside overlay.said")
    else:
        fixed = d["sep"].join(ov["fix"].get(str(i), w) for i, w in enumerate(words))
        if fixed != ov["natural"]:
            problems.append(f"{lang} demo: overlay.said with overlay.fix gives\n    {fixed}\n  but overlay.natural is\n    {ov['natural']}")
    if len(words) < 15:
        problems.append(f"{lang} demo: overlay.said needs at least 15 words")
    for k, s in d["styler"].items():
        if re.sub(r"</?s>", "", s["said"]).count("<") or s["said"].count("<s>") != s["said"].count("</s>"):
            problems.append(f"{lang} demo: styler.{k}.said may only use <s>…</s>")
    for sc in d["hero"]:
        if sc["style"] not in d["styles"]:
            problems.append(f"{lang} demo: hero style {sc['style']} is not a style key")
        if len(sc["said"].split(" ")) < 8:
            problems.append(f"{lang} demo: hero said needs at least 8 words (space-separated)")


def main():
    check = "--check" in sys.argv
    problems, writes = [], {}
    src = {p: BLOCK.sub("", (ROOT / f"{p}.html").read_text(encoding="utf-8")) for p in PAGES}
    trees = {p: parse(s) for p, s in src.items()}

    units = Units()
    for p in PAGES:
        collect(p, trees[p], units)
    writes[SRC / "units.json"] = json.dumps(
        [{"pt": k, "kind": v["kind"], "where": v["where"]} for k, v in units.items.items()], ensure_ascii=False, indent=1) + "\n"

    demo = {c: json.loads((SRC / "demo" / f"{c}.json").read_text(encoding="utf-8")) for c in L if (SRC / "demo" / f"{c}.json").exists()}
    for c in L:
        if c not in demo:
            problems.append(f"{c} demo: i18n/src/demo/{c}.json is missing")
        elif c != "pt":
            check_demo(c, demo[c], demo["pt"], problems)

    for p in PAGES:
        writes[ROOT / f"{p}.html"] = add_blocks(src[p], "pt", p)
    for c in L:
        if c == "pt":
            continue
        tf = SRC / "pages" / f"{c}.json"
        table = json.loads(tf.read_text(encoding="utf-8")) if tf.exists() else {}
        table = {norm(k): v for k, v in table.items()}
        used_all = set()
        for p in PAGES:
            out, used = translate(p, src[p], c, table, problems)
            used_all |= used
            writes[ROOT / c / f"{p}.html"] = add_blocks(out, c, p)
        for k in sorted(set(table) - used_all):
            print(f"warning: {c} has an entry no page uses (old text?): {k[:90]}")
        for k in used_all:
            m = portuguese(c, table[k])
            if m:
                problems.append(f"{c} looks Portuguese ('{m}'): {k}  ->  {table[k]}")

    for c in L:
        if c not in demo:
            continue
        data = {k: v for k, v in demo[c].items() if k != "hint"}
        if c == "pt":
            data["hints"] = {o: demo[o]["hint"] for o in L if o != "pt" and o in demo}
        body = json.dumps({c: data}, ensure_ascii=False, separators=(",", ":"))
        writes[ROOT / "i18n" / f"{c}.js"] = (
            "/* Generated by i18n/build.py from i18n/src/demo/" + c + ".json. Do not edit. */\n"
            "window.FV_I18N=Object.assign(window.FV_I18N||{}," + body + ");\n")

    today = date.today().isoformat()
    sm = ['<?xml version="1.0" encoding="UTF-8"?>',
          '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">']
    for p in PAGES:
        for c in L:
            sm.append(f"  <url>\n    <loc>{page_url(c, p)}</loc>\n    <lastmod>{today}</lastmod>")
            sm += [f'    <xhtml:link rel="alternate" hreflang="{L[o]["tag"]}" href="{page_url(o, p)}"/>' for o in L]
            sm.append(f'    <xhtml:link rel="alternate" hreflang="x-default" href="{page_url(X_DEFAULT, p)}"/>\n  </url>')
    sm.append("</urlset>")
    writes[ROOT / "sitemap.xml"] = "\n".join(sm) + "\n"

    if problems:
        problems = list(dict.fromkeys(problems))
        print("\n".join(problems))
        print(f"\n{len(problems)} problem(s); nothing written except i18n/src/units.json.")
        if not check:
            io.open(SRC / "units.json", "w", encoding="utf-8", newline="\n").write(writes[SRC / "units.json"])
        sys.exit(1)

    stale = []
    for path, text in writes.items():
        old = path.read_text(encoding="utf-8") if path.exists() else None
        if path.name == "sitemap.xml" and old is not None:
            old = re.sub(r"<lastmod>[^<]*</lastmod>", f"<lastmod>{today}</lastmod>", old)
        if old != text:
            stale.append(path)
            if not check:
                path.parent.mkdir(parents=True, exist_ok=True)
                io.open(path, "w", encoding="utf-8", newline="\n").write(text)
    rel = lambda p: p.relative_to(ROOT).as_posix()
    if check:
        if stale:
            print("out of date: " + ", ".join(rel(p) for p in stale))
            sys.exit(1)
        print(f"ok: {len(units.items)} units, {len(L)} languages, everything up to date")
    else:
        print(f"ok: {len(units.items)} units, {len(L)} languages; wrote {len(stale)} file(s)" + (": " + ", ".join(rel(p) for p in stale) if stale else ""))


if __name__ == "__main__":
    sys.stdout.reconfigure(encoding="utf-8")
    main()
