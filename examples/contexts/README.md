# Contexts: what a project's configuration gives a program

The cases follow [HQL-0025](../../doc/proposals/HQL-0025/index.hmd) and
[HQL-0059](../../doc/proposals/HQL-0059/index.hmd). An
invocation runs in an **execution context**: a mode, a project or none,
and the configuration gathered for it. The same program text checks the
same way in every context; the context sets the module roots, the
extensions imported for every program, and whether the prelude arrives.
Each case runs in one of two projects, each a folder with an `.hql/config.toml`
and no documents:

| Project | Its `.hql/config.toml` |
| --- | --- |
| [project](project/.hql/config.toml) | `[modules] roots = ["lib"]`, so `club.shelf` in [lib/club/shelf.hql](project/lib/club/shelf.hql) is importable; `[modules] import = ["stats"]`, so every program has `stats` |
| [strict](strict/.hql/config.toml) | `[modules] prelude = false`, so every module is imported where it is used |

| Case | What it shows | Result |
| --- | --- | --- |
| [module root](queries/project-module-root.hql) | `import club.shelf`, found under the project's module root | `["Dune", "Middlemarch"]` |
| [configured import](queries/project-import.hql) | `mean` from `stats` with no `import` line | `3.0`, an `Option<Float>` |
| [prelude declined](queries/invalid-prelude-declined.hql) | `trim` without `import string` where the prelude is declined | a name error |
| [prelude declined, imported](queries/prelude-declined-import.hql) | the same program with `import string` | `owl` |

From the repository root, `--project` names the project in place of the
upward walk from the program's folder, and `hql context` writes the context
each layer contributed:

```sh
cargo build --locked
target/debug/hql --project examples/contexts/project run examples/contexts/queries/project-module-root.hql
target/debug/hql --project examples/contexts/project context examples/contexts/queries/project-module-root.hql
target/debug/hql --project examples/contexts/strict run examples/contexts/queries/invalid-prelude-declined.hql   # refused
```
