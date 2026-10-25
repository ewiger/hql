# 0050 — Every module file declares its name: `module a.b`, `module a.b for T`

Status: done
Related: [HQL-0024, modules](../../proposals/HQL-0024/parts/01-modules.md),
[HQL-0024, receiver modules](../../proposals/HQL-0024/parts/04-receiver-modules.md),
[`lib/std/`](../../../lib/std/README.md),
[Mighty Types](../../../examples/mighty-types/README.md)

A module's name is derived from its file's position under a module root. A
module file now also states that name on its first line, as
[the module declaration](../../proposals/HQL-0024/parts/01-modules.md#the-module-declaration)
in HQL-0024's modules part defines, and loading the module checks the two
against each other. The declaration is a safety check on the name: a moved or
copied file, or a typing mistake in the name, fails when the module is loaded
instead of the module silently answering to another name.

```hql
module io.logging              // name only
module string for String       // name and receiver pattern
```

The receiver form `module for T` becomes `module <name> for T`, so a file has
one header line.

## Work

1. Grammar: `module <dotted.name>` with an optional `for <pattern>`, one
   `Stmt::Module` replacing `Stmt::ModuleFor`. `module for T` without a name is
   refused, and the diagnostic gives the spelling with the name.
2. Loading a workspace module (`src/checker.rs`, `load_module`) refuses a file
   whose first statement is not its declaration, whose declaration names
   another module, or which declares twice.
3. `prelude.hql` holds no declaration; one is refused.
4. The standard library: every `lib/std/**/mod.hql` opens with its
   declaration, and `library::closure` checks each name against the
   `MODULES` and `WRITTEN` tables.
5. Corpus: `examples/mighty-types/lib/mighty/*.hql` and every inline module
   source in `tests/` open with their declarations; an invalid case covers a
   declaration that names the wrong module.
6. Docs: HQL-0024 parts 1 and 4, the proposals, cards and learn pages that
   spell `module for T` write `module <name> for T`.
