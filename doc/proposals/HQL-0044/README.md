# HQL-0044: Type checking is the semantic boundary; compilation is an execution strategy

**Status**: accepted, implemented
**Created**: 2026-10-25
**Source**: the [core architecture](../../models/domain/architecture.md),
[checking](../../models/behavior/expressions.md#checking) in the expressions
model, and `run_on` in `src/lib.rs`

## Companion notes

- [Compilation](../../wiki/hql/compilation.hmd) — the wiki card that states
  the phases and the vocabulary this record fixes.

## Abstract

An HQL program is parsed, then checked, then executed by a backend. Checking
resolves every name, import and module, infers and checks every type, and
verifies every trait implementation, bound and coherence rule; a program the
checker rejects has no meaning and no part of it executes. Execution is a
strategy a backend chooses: `localhost` evaluates the checked execution tree,
and a backend that compiles or JIT-compiles it is an execution strategy of the
same kind, bound to accept exactly what the checker accepts and to compute
what backend-compute can run. HQL therefore has no compile time: a rule the
documentation attaches to a phase names checking, the parser, or execution. A
trait is a **capability contract** over a type, and the checker verifies it.

## Motivation

- Every page that called a trait a "compile-time contract" described HQL as an
  ahead-of-time compiled language. No HQL program is compiled: `hql check`,
  `hql run`, `hql render` and `hql repl` all check and then evaluate.
- A rule stated against compilation would change meaning the day a compiling
  backend exists, or would have to be restated for evaluation. A rule stated
  against checking holds for every backend.
- Documentation that says a program "is refused" leaves the reader to guess
  which part of HQL refused it and when. The diagnostic already names the
  kind, `syntax failure`, `name error` or `type error`, and the prose can say
  the same thing.

## Non-goals

- No compiling backend is specified. This record fixes what one MUST preserve.
- "Statically typed", the language's tagline, is unchanged: it states that
  every value's type is known before the program runs, which is true.
- The Rust implementation compiles. "An extension compiled into the binary"
  speaks of the `hql` binary and stays as it is.

## Specification

### 1. The phases

A program passes through these phases, in this order; each consumes only
what the one before it produced.

```text
parse    source text        → syntax tree              rejects: syntax failure
check    syntax tree        → checked program          rejects: name error, type error
lower    checked program    → execution tree
plan     execution tree     → stages per step
execute  planned tree       → value and reports        fails:   evaluation failure
```

- Checking MUST run over the whole program before any of it executes.
  Under `collect` every statement is checked even after one fails, and a
  program with a check error still executes nothing.
- A document is checked as a whole, every cell with the types of the cells
  it reads, before any cell runs.
- `hql check` runs parse and check and stops; it is the same checking every
  other command runs, not a separate analysis.

### 2. Checking is the boundary

What a program means is decided by the parser and the checker. Execution
computes it.

- A backend MUST receive only a checked program. Lowering, planning and
  execution MUST NOT resolve a name, infer a type or verify a contract
  again.
- A backend MUST NOT accept a program the checker rejects, and MUST NOT
  reject a program on a ground the checker owns.
- A failure that depends on the values, an overflow, a missing file, a model
  that does not answer, is an evaluation failure. It is reported by the run
  and is not a type error.

### 3. Compilation is an execution strategy

A backend that compiles is one more way to execute a checked program.

- A compiling or JIT backend MUST produce, for every checked program, the
  value and reports `localhost` produces, which is the oracle every backend
  is tested against.
- Its compilation step belongs to execution: it runs after checking, and a
  failure in it is a failure of that backend, never a diagnostic about the
  program.

### 4. Traits are capability contracts

A **trait** is a capability contract over a type: a named set of functions
that an implementing type provides, each taking that type as `self`.

- The checker verifies each implementation against its trait, the trait's
  bounds wherever they are used, and one implementation per trait and type.
  Each is a type error.
- A trait has no runtime representation. No value carries one, and no trait
  object or existential type is introduced. A call to a trait function runs
  the implementation of the value's own type, as
  [HQL-0039](../HQL-0039/README.md) states.

### 5. Vocabulary

The documentation names the phase that acts.

- A rule enforced before the program executes is stated against the
  checker or the parser: "the checker rejects …", "… is a type error",
  "… is a name error", "the parser rejects …". "Before the program runs"
  MAY accompany it.
- A sentence about a rejection MUST name what rejects it or the diagnostic
  kind it gives; "… is refused" alone does not.
- "Compile time", "compile-time" and "pre-compilation" MUST NOT describe an
  HQL rule. "Static" stays in "statically typed" and is not used to name a
  phase.

## Backwards Compatibility

No behaviour changes; the implementation already checks before it executes.
The record amends [HQL-0024](../HQL-0024/README.md), its abstract and
[part 5](../HQL-0024/parts/05-traits.md), [HQL-0027](../HQL-0027/README.md)
and [HQL-0039](../HQL-0039/README.md), where each called a trait
compile-time.

## Reference Implementation

- `src/lib.rs`, `run_on`: parsing, then checking, and a backend reached only
  with a checked program.
- `src/execution.rs` lowers checked facts; `src/evaluation.rs` consumes only
  the execution tree.
- The documentation: [the Learn book](../../public/learn/index.md), the
  [type system](../../wiki/hql/type-system.hmd) and
  [design status](../../wiki/hql/design-status.hmd) cards, and the
  repository `README.md`.

## Test Plan

The boundary is held by the existing suites: `tests/extension_contracts.rs`
runs programs with a check error under `strict` and `collect` and asserts that
no value is produced, and `tests/tutorial.rs` runs every program in the Learn
book against the binary and compares what it prints, diagnostics included.

```bash
cargo test --locked
```

## Changelog

- 2026-10-25: accepted; the documentation states checking as the boundary
