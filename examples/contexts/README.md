# Contexts: what a workspace's configuration gives a program

The cases follow [HQL-0025](../../doc/proposals/HQL-0025/index.hmd). An
invocation runs in an **execution context**: a mode, a workspace or none,
and the configuration gathered for it. The same program text checks the
same way in every context; the context sets the module roots, the
extensions imported for every program, and whether the prelude arrives.
Each case runs in one of two workspaces, each a folder with an `hql.toml`
and no documents:

| Workspace | Its `hql.toml` |
| --- | --- |
| [workspace](workspace/hql.toml) | `[modules] roots = ["lib"]`, so `club.shelf` in [lib/club/shelf.hql](workspace/lib/club/shelf.hql) is importable; `[extensions] import = ["stats"]`, so every program has `stats` |
| [strict](strict/hql.toml) | `[extensions] prelude = false`, so every module is imported where it is used |

| Case | What it shows | Result |
| --- | --- | --- |
| [module root](queries/workspace-module-root.hql) | `import club.shelf`, found under the workspace's module root | `["Dune", "Middlemarch"]` |
| [configured import](queries/workspace-import.hql) | `mean` from `stats` with no `import` line | `3.0`, an `Option<Float>` |
| [prelude declined](queries/invalid-prelude-declined.hql) | `trim` without `import string` where the prelude is declined | a name error |
| [prelude declined, imported](queries/prelude-declined-import.hql) | the same program with `import string` | `owl` |

From the repository root, `--workspace` names the workspace in place of the
upward walk from the program's folder, and `hql context` writes the context
each layer contributed:

```sh
cargo build --locked
target/debug/hql --workspace examples/contexts/workspace run examples/contexts/queries/workspace-module-root.hql
target/debug/hql --workspace examples/contexts/workspace context examples/contexts/queries/workspace-module-root.hql
target/debug/hql --workspace examples/contexts/strict run examples/contexts/queries/invalid-prelude-declined.hql   # refused
```
