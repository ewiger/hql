# Scripts: `#!`, arguments and the exit status

The cases follow [HQL-0062](../../doc/proposals/HQL-0062/index.hmd). Each
is an executable file whose first line is `#!/usr/bin/env hql`, so the shell
runs it by its path. The words after the path are its positional arguments,
read with `io.env.argv()`, and its result gives the exit status: `1` when
the result is an `err`, `0` otherwise.

| Case | What it shows | Result |
| --- | --- | --- |
| [todo](todo.hql) | the first argument picks a status; a status no book has is an `err` | `ok(["The Hobbit", "Leaves of Grass"])`, exit `0`; `err(…)`, exit `1` |
| [lsx](lsx.hql) | each folder named on the command line, joined to the working directory, or that directory alone | one name per line |

From the repository root, with `hql` on the `PATH`:

```sh
cargo build --locked
export PATH="$PWD/target/debug:$PATH"
examples/scripts/todo.hql reading            # ok(["Dune"])
examples/scripts/todo.hql lent; echo $?     # err("no book is lent"), then 1
examples/scripts/lsx.hql examples/fs/tree    # assets, data
```

A relative path in a script resolves against the script's folder, as it
does in every HQL program. A path given on the command line is named from
where the user stands, so `lsx.hql` joins it to `fs.working_dir()`, and
`join` keeps an absolute path as it is.
