"""Rebase the relative links of a folder note. A stopgap until the
`hypermarkdown` MkDocs plugin does it itself (hypermarkdown issue 0010).

The plugin registers a folder note `a/b/index.hmd` as the page `a/b.md`,
one directory above the file, so `a/b/` is its URL. Wikilinks are rewritten
for that place, but an ordinary relative Markdown link is left as written,
against the file's own directory, and MkDocs then resolves it from `a/`.
`[`Bool`](bool.hmd)` written in `learn/02-data-types/index.hmd` arrives at
`learn/bool.hmd`.

This hook runs after the plugin. In a folder note it takes each relative link
that names nothing from the registered page but names a file from the folder
note's own directory, and points it at that file, by its page name when it is
a card. Everything else is left alone.
"""

from __future__ import annotations

import posixpath
import re
from pathlib import Path

from hypermarkdown import urls
from hypermarkdown.scan import mask

#: An inline link's destination: `](path#fragment)` or `](path "title")`.
_HREF = re.compile(r"\]\((?P<path>[^)\s#]+)(?P<rest>[#)\s])")
_SCHEME = re.compile(r"^[a-z][a-z0-9+.-]*:")


def on_page_markdown(markdown, /, *, page, config, files):
    docs = Path(config.docs_dir).resolve()
    src = page.file.src_uri
    if not src.endswith(".md"):
        return markdown
    note = docs / src[: -len(".md")] / "index.hmd"
    if not note.is_file():
        return markdown

    registered = posixpath.dirname(src)
    real = posixpath.dirname(note.relative_to(docs).as_posix())

    def page_for(path: str) -> str | None:
        """The registered page a docs-relative path is, or None. A `.hmd`
        source is excluded from the site and stands for the page it becomes."""
        if path.endswith(".hmd"):
            return urls.dest_for(docs, docs / path) if (docs / path).is_file() else None
        if files.get_file_from_path(path) is not None:
            return path
        return None

    masked = mask(markdown)
    out: list[str] = []
    cursor = 0
    for match in _HREF.finditer(masked):
        href = match.group("path")
        if _SCHEME.match(href) or href.startswith("/"):
            continue
        if page_for(posixpath.normpath(posixpath.join(registered, href))) is not None:
            continue
        target = posixpath.normpath(posixpath.join(real, href))
        dest = page_for(target)
        if dest is None and not (docs / target).exists():
            continue
        new = posixpath.relpath(dest or target, registered or ".")
        out.append(markdown[cursor : match.start("path")])
        out.append(new)
        cursor = match.end("path")
    out.append(markdown[cursor:])
    return "".join(out)
