# 0046 — Renovate `std/**` under the module, package and extension model

Status: backlog
Related: [`std/`](../../std/)

Renovate the current `std/**` HQL standard library according to the latest language design.

Treat the existing stdlib as an early draft, not as an architecture that must be preserved. The goal is to reorganize it into a coherent standard-library package using the current HQL concepts of **modules, types, constraints/capabilities, receiver association, pipeline behaviors, packages, and extensions**.

Use the latest ADRs/EDRs as authoritative, especially those covering:

- modules and module roots;
- package resolution and the standard library as a package;
- extensions and how Rust implementations provide HQL module contracts;
- type constraints/capabilities;
- receiver modules such as `module for String` or generic receiver modules;
- imports, prelude visibility, module identity and name resolution;
- execution behaviors such as `elementwise`, `partition`, `reduction`, etc.;
- the distinction between ordinary expression behavior and planner-visible pipeline behavior.

Do **not** preserve old structures merely because they currently exist in `std/mod.hql`. That file accumulated types, collection operations, joins, reductions, planner annotations, constructors and implementation notes before the module/package model was mature.

The renovation should determine the proper ownership of every public declaration.

For each existing type/function/operation, decide:

1. Which HQL module owns it?
2. Is it a value type, abstract type/capability/constraint, receiver behavior, ordinary function, or pipeline operation?
3. Does generic code actually need a reusable constraint, or is a receiver module sufficient?
4. Should the declaration live in the root std module or in a more specific module?
5. Is its implementation written in HQL or supplied by an extension?
6. Which extension/package provides the implementation?
7. How does the declaration become visible: prelude, implicit standard receiver visibility, or explicit import?
8. Does its current inheritance/type relationship still make sense under the newer module and constraint system?

Important principles:

- HQL declarations define the public contract. Rust extensions implement those contracts; the public API must not be reverse-engineered from Rust.
- The standard library is itself a package using the same module semantics as other packages.
- Built-in modules and third-party modules should differ mainly in provisioning, not in language semantics.
- Do not equate arbitrary source files with modules. Follow the adopted module-root rules.
- Do not invent `
