# Projects and vaults: what a run has without HyperMarkdown, and with it

The cases follow [HQL-0059](../../doc/proposals/HQL-0059/index.hmd). A
**project** is a directory holding `.hql/config.toml`: it configures how a
program runs. A **vault** is a directory holding `.hmd/`: its documents are
cards. The two are found apart, and the builtin package `hmd` binds the
vault only in a run that imports one of its modules.

| Directory | What it is |
| --- | --- |
| [library](library/.hql/config.toml) | a project and no vault: `[modules] roots = ["lib"]`, with the module [`shelf`](library/lib/shelf/mod.hql) and the file `titles.txt` beside it |
| [notes](notes/.hmd/config.toml) | a vault and no project: the cards `owl` and `heron` |
| [pinned](pinned/.hql/config.toml) | a project that declares the package `ml` under `[packages] dependencies`, pinned in [`lock.toml`](pinned/.hql/lock.toml), and has not installed it |

## Plain HQL and `hmd`

| Case | What it shows | Result |
| --- | --- | --- |
| [cards without hmd](queries/invalid-cards-without-hmd.hql) | `cards` in a run where `hmd` is not active | a name error naming `import hmd` |
| [reference without hmd](queries/invalid-reference-without-hmd.hql) | `@[[owl]]` in a run where `hmd` is not active | a name error naming `import hmd` |
| [import hmd](queries/import-hmd.hql) | `import hmd` in the program, over the vault `notes` | `["heron", "owl"]` |
| [plain refuses cards](sessions/plain-refuses-cards.session) | `--vault` names the vault and activates nothing | both programs refused |
| [import flag](sessions/import-flag.session) | `--import hmd` on the command line, the programs unchanged | `2`, then `owl` |
| [import without a vault](sessions/import-without-a-vault.session) | `--import hmd` with no vault | a name error naming `--vault` |

## The source directory of a module

| Case | What it shows | Result |
| --- | --- | --- |
| [module source directory](queries/module-source-dir.hql) | `fs.source_dir()` called in `shelf` | `shelf`, the module's directory |
| [module relative path](queries/module-relative-path.hql) | `file("titles.txt")` written in `shelf` | the lines of the file beside the module |
| [from the command line](sessions/module-source-dir.session) | both programs under `--project` | the same values |

## A declared package

| Case | What it shows | Result |
| --- | --- | --- |
| [package not installed](queries/invalid-package-not-installed.hql) | `import ml` where no package provides `ml` | a name error |
| [declared package](sessions/declared-package.session) | the same program in the project that declares `ml` | the run fails before the program is checked, naming `hql package install` |

`pinned/.hql/` commits `config.toml` and `lock.toml` and ignores `packages/`
and `cache/`. `hql --project examples/projects/pinned package install`
installs `ml` at the pinned version; the session above describes the project
before that command has run.

From the repository root:

```sh
cargo build --locked
target/debug/hql --vault examples/projects/notes run examples/projects/queries/invalid-cards-without-hmd.hql               # refused
target/debug/hql --vault examples/projects/notes --import hmd run examples/projects/queries/invalid-cards-without-hmd.hql  # 2
target/debug/hql --project examples/projects/library run examples/projects/queries/module-relative-path.hql
target/debug/hql --project examples/projects/pinned run examples/projects/queries/invalid-package-not-installed.hql        # refused
```
