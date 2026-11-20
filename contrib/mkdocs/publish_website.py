"""Put the website's build at the root of a `gh-pages` checkout.

    python contrib/mkdocs/publish_website.py <site_dir> <gh-pages worktree>

The root of `gh-pages` holds two things: mike's version folders, listed in
`versions.json`, and the website (`mkdocs.site.yml`), built from `main`. This
script replaces the website and leaves the versions alone. It keeps `.git`,
`CNAME`, `.nojekyll`, `versions.json` and every version and alias folder that
`versions.json` names, deletes every other root entry, and copies `<site_dir>`
in. It commits nothing; the caller does.

A website entry with the name of a kept one would overwrite a version or the
custom domain, so the script refuses it and changes nothing.
"""

from __future__ import annotations

import json
import shutil
import sys
from pathlib import Path

#: Root entries that are not the website's, beside the versions.
_KEEP = {".git", "CNAME", ".nojekyll", "versions.json"}


def kept_names(pages: Path) -> set[str]:
    """The root entries that belong to mike or to the repository."""
    keep = set(_KEEP)
    manifest = pages / "versions.json"
    if manifest.exists():
        for entry in json.loads(manifest.read_text(encoding="utf-8")):
            keep.add(entry["version"])
            keep.update(entry.get("aliases", []))
    return keep


def publish(site: Path, pages: Path) -> None:
    if not (site / "index.html").is_file():
        raise SystemExit(f"{site} is not a site build: it has no index.html")
    if not (pages / ".git").exists():
        raise SystemExit(f"{pages} is not a git checkout of gh-pages")

    keep = kept_names(pages)
    clashes = sorted(entry.name for entry in site.iterdir() if entry.name in keep)
    if clashes:
        raise SystemExit(
            "the website would overwrite what gh-pages keeps: " + ", ".join(clashes)
        )

    for entry in pages.iterdir():
        if entry.name in keep:
            continue
        if entry.is_dir() and not entry.is_symlink():
            shutil.rmtree(entry)
        else:
            entry.unlink()

    for entry in site.iterdir():
        target = pages / entry.name
        if entry.is_dir():
            shutil.copytree(entry, target)
        else:
            shutil.copy2(entry, target)


def main(argv: list[str]) -> None:
    if len(argv) != 3:
        raise SystemExit(__doc__.strip().splitlines()[2].strip())
    publish(Path(argv[1]), Path(argv[2]))


if __name__ == "__main__":
    main(sys.argv)
