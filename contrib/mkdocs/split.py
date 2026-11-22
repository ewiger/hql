"""Split `doc/public/` between the site's two builds.

hql-lang.org is two MkDocs builds from one vault (see `mkdocs.base.yml`):

- **website** (`mkdocs.site.yml`): Home, Install, the Playground and the Blog, built from `main`
  and served at the root, so it is always current;
- **docs** (`mkdocs.yml`): the Book, Examples and Reference, published by mike
  under each version.

`extra.site_part` names the build. This hook keeps that part's pages, drops
the other part's, and checks every link that crosses from one part to the
other. `assets/` belongs to both.

`exclude_docs` cannot do the dropping: the hypermarkdown plugin registers its
pages itself, in its own `on_files`, after MkDocs has applied the exclusions.
This hook runs after the plugins, so it sees those pages and removes them.

The two parts cannot link to each other with relative links, since they are
served from different places. The website reaches the documentation at
`/draft/…`, and the documentation reaches the website at `/…`. A relative link
into the other part fails the build, and so does a root-absolute link to a page
that part does not have. Both are logged as warnings, which `--strict` turns
into errors.
"""

from __future__ import annotations

import logging
import posixpath
import re

from mkdocs.exceptions import PluginError
from mkdocs.structure.files import File, Files

from hypermarkdown.scan import mask

log = logging.getLogger("mkdocs.hooks.split")

PARTS = ("website", "docs")

#: The website's pages, as MkDocs source paths. Everything else is the
#: documentation's, apart from what both share.
_WEBSITE_PAGES = {"index.md", "install.md", "blog.md", "playground.md"}
_WEBSITE_DIRS = ("blog/",)
_SHARED_DIRS = ("assets/",)

#: Where the website finds the documentation: the draft, built from `main`
#: like the website, so every page the website links to is there.
DOCS_PREFIX = "/draft/"
#: A root-absolute path to a release: a version folder (`/v0.7.1/`) or
#: `latest`. Pinned on purpose, so not checked.
_PINNED = re.compile(r"^/(v\d[^/]*|latest)(/|$)")

_HREF = re.compile(r"\]\((?P<md>[^)\s]+)|\bhref=\"(?P<html>[^\"]+)\"")
_SCHEME = re.compile(r"^[a-z][a-z0-9+.-]*:")

#: Filled by `on_files`, read by `on_page_markdown`. A hook is a module, and
#: one build runs it once.
_part: str = ""
_urls: dict[str, set[str]] = {}
_dropped: set[str] = set()


def part_of(src_uri: str) -> str:
    """`website`, `docs` or `shared`: which build a source file belongs to."""
    if src_uri.startswith(_SHARED_DIRS):
        return "shared"
    if src_uri in _WEBSITE_PAGES or src_uri.startswith(_WEBSITE_DIRS):
        return "website"
    return "docs"


def on_files(files: Files, /, *, config) -> Files:
    global _part
    _part = config.extra.get("site_part", "")
    if _part not in PARTS:
        raise PluginError(f"extra.site_part must be one of {PARTS}, not {_part!r}")

    _urls.clear()
    _urls.update({part: set() for part in PARTS})
    _dropped.clear()

    for file in list(files):
        part = part_of(file.src_uri)
        if part == "shared":
            continue
        _urls[part].add(_url(file))
        if part != _part:
            _dropped.add(file.src_uri)
            files.remove(file)

    for title, href in _nav_links(config.nav or []):
        _check_absolute(href, f"the nav entry {title!r}")
    return files


def on_post_build(*, config) -> None:
    """The documentation has no home page of its own: a version's root
    forwards to the Book, and the logo goes to the website's home
    (`extra.homepage`)."""
    if _part != "docs":
        return
    index = posixpath.join(config.site_dir, "index.html")
    with open(index, "w", encoding="utf-8") as out:
        out.write(
            '<!doctype html>\n<meta charset="utf-8">\n'
            '<meta http-equiv="refresh" content="0; url=book/">\n'
            '<link rel="canonical" href="book/">\n'
            '<title>HQL</title>\n<a href="book/">The HQL book</a>\n'
        )


def on_page_markdown(markdown: str, /, *, page, config, files) -> str:
    src = page.file.src_uri
    for match in _HREF.finditer(mask(markdown)):
        href = match.group("md") or match.group("html")
        if _SCHEME.match(href) or href.startswith(("#", "//")):
            continue
        if href.startswith("/"):
            _check_absolute(href, src)
            continue
        path = href.split("#", 1)[0].split("?", 1)[0]
        target = posixpath.normpath(posixpath.join(posixpath.dirname(src), path))
        if target in _dropped:
            other = part_of(target)
            log.warning(
                "%s links to %s, which is in the %s part of the site and not "
                "in this build; link to it as %s",
                src,
                href,
                other,
                _absolute_for(target, other),
            )
    return markdown


def _url(file: File) -> str:
    """A page's URL inside its own build, `""` for the home page."""
    if not file.is_documentation_page():
        return file.src_uri
    return "" if file.url in (".", "./") else file.url


def _absolute_for(src_uri: str, part: str) -> str:
    stem = src_uri[: -len(".md")] if src_uri.endswith(".md") else src_uri
    stem = "" if stem == "index" else stem + "/"
    return (DOCS_PREFIX if part == "docs" else "/") + stem


def _check_absolute(href: str, where: str) -> None:
    """A root-absolute link must name a page of the part it points into."""
    path = href.split("#", 1)[0].split("?", 1)[0]
    if _PINNED.match(path):
        return
    if path.startswith(DOCS_PREFIX):
        part, rest = "docs", path[len(DOCS_PREFIX) :]
    else:
        part, rest = "website", path[1:]
    last = rest.rsplit("/", 1)[-1]
    if rest and "." not in last:
        rest = rest.rstrip("/") + "/"
    if rest not in _urls[part]:
        log.warning(
            "%s links to %s, but the %s part of the site has no page %s",
            where,
            href,
            part,
            rest or "(home)",
        )


def _nav_links(node, title: str = ""):
    """Every root-absolute link in the nav, with its title."""
    if isinstance(node, str):
        if node.startswith("/"):
            yield title, node
    elif isinstance(node, dict):
        for key, value in node.items():
            yield from _nav_links(value, key)
    elif isinstance(node, list):
        for item in node:
            yield from _nav_links(item, title)
