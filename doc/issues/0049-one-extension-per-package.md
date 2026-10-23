# 0049 — One extension per package: fold the descriptors into `std` and `search`

Status: todo
Related: [packages](../status/packages.hmd),
[HQL-0024, modules](../proposals/HQL-0024/parts/01-modules.md),
[HQL-0024, packages and extensions](../proposals/HQL-0024/parts/09-packages-and-extensions.md),
[issue 0048](0048-builtin-packages-under-lib.md)

A package carries one extension, named as the package is, and more only when
parts of it are enabled or loaded independently. The binary carries two
builtin packages, so it registers two internal extensions: `std`, providing
every module of `lib/std/` that has an external declaration, and `search`,
providing `lexical` and `semantic`.

`src/extensions/mod.rs` registers fourteen `Extension` entries instead, one
per Rust source module, and visibility hangs on them: `core` is always
visible, an entry named in `lib/std/prelude.hql` is visible through the
prelude, and `import <name>` binds an entry of that name. Folding the entries
is therefore a change to visibility first: a module, not an extension, is
what is always visible, in the prelude or imported.

## Work

1. Visibility per module. Availability (always, prelude, import), the
   imported set of a unit and the "write `import …`" diagnostic name the
   module whose declaration a step satisfies, read from the binding, not
   the `Extension` entry that registers the step. `import net` and
   `import logging` keep resolving as the last segments of `io.net` and
   `io.logging`.
2. Two descriptors. `STD` and `SEARCH` replace the fourteen statics; their
   `provides` lists are the two rows of the Extensions table in
   [packages](../status/packages.hmd). The Rust source modules
   (`string.rs`, `seq.rs`, …) stay as they are and contribute their step
   tables to one descriptor; `lexical`'s metadata contribution moves to
   `SEARCH`.
3. An implementation kind on the descriptor: internal for both, with
   external reserved for an installed package's extension.
4. `hql builtins` names `std` or `search` as the provider, and the module
   beside it. Tests and snapshots that print a provider follow.
5. Comments: the `//!` headers of the Rust source modules stop calling
   themselves extensions (for example `format.rs`); the module
   headers under `lib/std/` already name `std`.
6. [Learn, retrieval](../public/learn/08-retrieval.md) quotes the
   diagnostic "`lexical` is provided by the `lexical` extension"; it is
   updated with the message.
