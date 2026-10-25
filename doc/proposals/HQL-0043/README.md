# HQL-0043: Public documentation: a reference page per primitive, permanent URLs and versions

**Status**: drafted
**Created**: 2026-10-25
**Source**: the documentation-site section of [`doc/stack.md`](../../stack.md),
[issue 0054](../../issues/0054-user-facing-website.md), and
[HQL-0042](../HQL-0042/README.md), whose doc strings these pages publish

## Abstract

Every primitive of HQL — each function, type, trait and module the standard
library exports — has a manual page under `doc/public/reference/std/`, and
that page has a permanent URL under `/latest/`:
`https://hql.hypermarkdown.org/latest/reference/std/seq/range/`. Every
release's documentation is also published under its own version, `/vX.Y.Z/`,
built once from the release tag by mike; `/latest/` is the newest release.
Cutting a release updates the reference pages from the doc strings in
`lib/std`, publishes them as that version, and moves `/latest/` to it. A URL
that an `hql` binary prints names its own version. This replaces the
"latest release only, no archive" rule in `doc/stack.md` and the matching
scope of issue 0054.

## Motivation

- The site has no reference for the standard library. A reader who meets
  `range` in a program has no page to look it up on.
- A URL needs to stay valid as the site is reorganised. A primitive's page is
  what a doc string, a diagnostic or another page links to, so its address
  is fixed by the primitive's name, not by the site's layout.
- A URL printed by a binary has to name a page that describes that binary.
  When only the latest release is published, an older binary's URL leads to
  a page for a newer language, which before 1.0 may differ.

## Non-goals

- Doc strings themselves: their syntax, attachment and reporting are
  [HQL-0042](../HQL-0042/README.md).
- Reference pages for the language's syntax, I/O and CLI. Those stay
  hand-written work under issue 0054.
- Backfilling documentation for releases before this record is implemented.

## Specification

### 1. A page per primitive

Every primitive the standard library exports has a manual page.

- A **primitive** is a name exported by a standard module: a function (all
  its forms), a type, a trait, or the module itself.
- The page of primitive `<name>` in module `std/<path>` is
  `doc/public/reference/std/<path>/<name>.md`; a module's page is
  `doc/public/reference/std/<path>/index.md`. `range` in `std/seq` is
  `doc/public/reference/std/seq/range.md`, and `std/io/fs` is
  `doc/public/reference/std/io/fs/index.md`.
- `doc/public/reference/std/index.md` lists every standard module.
- A page has two parts. The **declaration part** is the primitive's
  declarations and doc strings, as `help` prints them (HQL-0042), written
  from `lib/std` between two marker lines:

  ```text
  <!-- hql reference: std/seq.range -->
  …declarations and doc strings…
  <!-- hql reference: end -->
  ```

  The **manual** follows it: what the primitive does, shown with examples
  and their output, written by hand. Nothing edits the declaration part by
  hand, and nothing writes the manual.
- The pages are committed files. They describe the release they were last
  updated for; how they are updated is
  [Releasing the documentation](#4-releasing-the-documentation).
- Every primitive MUST have a page, and every page under
  `doc/public/reference/std/` MUST name a primitive.
- A page renders a doc string's citation (HQL-0042) as a relative Markdown
  link to the same page, so the link stays inside the version it is
  published under.

### 2. Permanent URLs

A primitive's page has a permanent URL, fixed by the primitive's module and
name.

```text
https://hql.hypermarkdown.org/latest/reference/std/<path>/<name>/
https://hql.hypermarkdown.org/latest/reference/std/<path>/
https://hql.hypermarkdown.org/v<X.Y.Z>/reference/std/<path>/<name>/
```

- The `/latest/` URL is the permanent one. It is what is linked from outside
  a versioned page: the README, issues, other sites.
- The `/vX.Y.Z/` URL names that release's page. An `hql` binary prints URLs
  under its own version (`CARGO_PKG_VERSION`), never `/latest/`, so a URL it
  prints describes the language it runs. A binary built from an unreleased
  version prints URLs that resolve once that version is released.
- A page path from the `doc/public/` root maps to its URL by MkDocs's
  directory URLs: the extension is dropped, a folder's `index` names the
  folder, and the URL ends in `/`.
- A primitive that is renamed or removed keeps no `/latest/` page. Its
  versioned pages stay where they were published.

### 3. Versions

Every release's documentation is published under its version, and
`/latest/` is the newest release.

- A release `vX.Y.Z` publishes its site at
  `https://hql.hypermarkdown.org/vX.Y.Z/`, built once from the tag and never
  rebuilt. A correction to a published version's pages ships in the next
  release.
- `https://hql.hypermarkdown.org/latest/` redirects each page to the same page
  of the newest release, and the site root redirects to `/latest/`.
- Every page carries the version selector and a canonical link to its
  `/latest/` URL.
- `main` and pull requests build the site with `--strict` and publish nothing.
- Releases before the first one built this way are not published under a
  version.

**Mechanism: mike.** [mike](https://github.com/jimporter/mike) deploys each
MkDocs build into its own version directory on a `gh-pages` branch, keeps the
version list in `versions.json`, and maintains aliases. The Material theme
reads that list for its version selector (`extra.version.provider: mike`), so
nothing is written by hand. A version is deployed once, from its tag, and a
later deploy touches only its own directory and the alias, which is what
makes `/vX.Y.Z/` describe the binary that prints it. The alternative that
keeps the current Pages artifact rebuilds every version from its tag on each
deploy, which needs every old tag to still build with today's toolchain; mike
stores what was built instead. The cost is a `gh-pages` branch, which the
current arrangement avoids, and Pages serving from it.

- `mike` is added to `pyproject.toml` with `uv add`, and `uv.lock` pins it.
- `mkdocs.yml` gains the `mike` plugin with `alias_type: redirect` and
  `canonical_version: latest`, and `extra.version: {provider: mike, default:
  latest}`.
- The deploy is the `docs` job of `release.yml`, described in
  [Releasing the documentation](#4-releasing-the-documentation). `pages.yml`
  keeps building the site with `--strict` on pull requests and deploys
  nothing.
- Settings → Pages → Source becomes *Deploy from a branch*, `gh-pages`, root.
  A `CNAME` file holding `hql.hypermarkdown.org` sits at the branch root,
  written by the first deploy; mike leaves root files it did not create.

### 4. Releasing the documentation

Updating the reference and publishing it are steps of cutting a release.
For a release `X.Y.Z`:

1. **Update the reference.** With the version bumped and the `CHANGELOG.md`
   section written, `cargo run --locked -- reference doc/public/reference/std`
   rewrites the declaration part of every page from `lib/std`'s doc strings,
   creates the page of a primitive that has none (a declaration part and an
   empty manual), and lists each page that names no primitive any more. The
   result is committed with the release commit, and a new page's manual is
   written before the tag.
2. **Check before tagging.** The `build` job of `release.yml` runs
   `hql reference --check doc/public/reference/std`, which writes nothing and
   fails on a declaration part that differs from `lib/std`, a primitive with
   no page, a page naming no primitive, or a page with an empty manual:
   `` doc/public/reference/std/seq/range.md: the declaration part differs from
   lib/std; run `hql reference` ``. A tag whose documentation is stale is not
   released.
3. **Publish the version.** A `docs` job of `release.yml`, after `release`
   and before `crates-io`, fetches `gh-pages`, refuses a version mike already
   lists, and runs `mike deploy --push --update-aliases vX.Y.Z latest` and
   `mike set-default --push latest`. The new version is published at
   `/vX.Y.Z/`, and `/latest/` now redirects to it. The job needs
   `contents: write`. `crates-io` stays last, the one step that cannot be
   undone, and runs only once the documentation is published.

- `hql reference` is a hidden command of the binary: the binary is the only
  program with HQL's parser. It is hidden from `hql --help` because it writes
  this repository's pages.
- Between releases, `main`'s pages may lag its doc strings. Nothing is
  published from `main`, so a reader sees only pages a release checked.

### 5. What changes in `doc/stack.md`

In the documentation-site section:

- The bullet "The site describes the latest release only; there is no archive
  of older versions. Before 1.0 a release may break the previous one."
  becomes: every release's documentation is published under `/vX.Y.Z/` by
  mike, built once from the tag; `/latest/` is the newest release and holds
  the permanent URLs; before 1.0 a release may break the previous one, and
  each version's pages describe that version.
- The deployment paragraph ("deploys from `main` as a Pages artifact; there is
  no `gh-pages` branch", Source *GitHub Actions*) becomes: `pages.yml` builds
  pull requests and deploys nothing, and the `docs` job of `release.yml`
  deploys each release with mike to `gh-pages`, Source *Deploy from a
  branch*.
- A bullet states that every standard primitive has a page under
  `doc/public/reference/std/`, at its permanent `/latest/` URL.

In the release section: the release checklist gains `hql reference`
before the commit, the `build` job gains `hql reference --check`, and a
`docs` job publishes the version with mike between `release` and
`crates-io`.

### 6. What changes in issue 0054

- Its opening says the site "always reflects the latest release, with no
  archive". It becomes: each release's documentation is published under its
  version, and `/latest/` is the newest.
- Work step 4, *Write Reference from the user's side*: the standard-library
  part of Reference is a page per primitive, by this record. The language,
  I/O and CLI pages remain work under 0054.

## Backwards Compatibility

- Site URLs move under `/vX.Y.Z/` and `/latest/`. The site root redirects to
  `/latest/`; an unversioned page URL from the current site is not
  redirected.
- `doc/public/reference.md` keeps the command table, and its
  standard-library paragraph links `reference/std/index.md` instead of the
  repository folder.

## Security Considerations

- The `docs` job has `contents: write` on the repository to push
  `gh-pages`. It runs only on a pushed release tag, and pull-request builds
  keep read-only permissions.

## Deployment / Activation

1. `hql reference` and `--check`, and the reference pages it creates, with
   their manuals written.
2. mike, the `docs` job and the `--check` step in `release.yml`; switch the
   Pages source to `gh-pages`. The next release is the first versioned one.
3. Update `doc/stack.md` and issue 0054 as
   [What changes in `doc/stack.md`](#5-what-changes-in-docstackmd) and
   [What changes in issue 0054](#6-what-changes-in-issue-0054) state.

## Reference Implementation

- **The reference command** — `src/reference.rs` and the hidden `reference`
  command with `--check`, reading the parsed declarations of HQL-0042.
  `tests/reference.rs`: the declaration part of a page, a created page, and
  each `--check` failure with its message.
- **Reference pages** — `doc/public/reference/std/` with each manual, the
  `mkdocs.yml` nav, and the link in `doc/public/reference.md`.
- **Versioned publishing** — `pyproject.toml` and `uv.lock` (mike),
  `mkdocs.yml` (plugin and `extra.version`), `release.yml` (`--check` in
  `build`, the `docs` job, `CNAME`), the Pages source setting, `doc/stack.md`,
  issue 0054.

## Test Plan

- `tests/reference.rs` covers writing a declaration part, creating a page,
  and each `--check` failure — a stale declaration part, a missing page, a
  page naming no primitive, an empty manual — with the exact message.
- `uv run mkdocs build --strict` on every pull request, and
  `hql reference --check` in the release `build` job.
- After the first versioned release: `/vX.Y.Z/reference/std/seq/range/` and
  `/latest/reference/std/seq/range/` both serve the page, and the site root
  redirects to `/latest/`.

## Changelog

- 2026-10-25: drafted
