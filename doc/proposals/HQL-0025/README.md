# HQL-0025: Execution contexts and running modes

**Status**: drafted
**Created**: 2026-10-22

## Scope

This ADR Supplementary defines how an HQL invocation establishes its **execution context**.

It answers:

- whether the invocation runs as plain standalone HQL or inside a workspace;
- how an HQL workspace is discovered;
- how a HyperMarkdown vault becomes an HQL workspace;
- what the current working directory means;
- what user-wide and system-wide configuration contributes;
- how `~/.hql/**` participates even without a workspace;
- how named contexts are selected and changed;
- how environment variables and command-line options affect context construction;
- which configuration sources contribute module roots, extensions and other runtime resources.

This ADR does **not** define module semantics, package resolution, extension ABI, import syntax or HyperMarkdown event semantics.

Those systems consume the execution context produced here.

---

# 1. Principle

HQL must support two equally valid ways of being used.

The first is lightweight, standalone execution:

```bash
cat data.json | hql '...'
```

or:

```bash
hql script.hql
```

This should feel closer to `jq`, `awk`, Python or another ordinary command-line language. It must not require creating a project, vault or workspace.

The second is contextual execution inside a larger HQL or HyperMarkdown environment:

```text
my-vault/
    .hmd/
    notes/
    scripts/
    ...
```

where the program should automatically inherit the vault, its configuration, modules, extensions, indexes and other contextual resources.

Neither mode is secondary.

The execution-context system exists to make the same `hql` executable behave correctly in both.

---

# 2. Core concepts

Four concepts must remain distinct:

```text
Process working directory
        ≠
Execution anchor
        ≠
Workspace root
        ≠
Vault root
```

They may happen to point to the same directory, but they mean different things.

## Working directory

The **working directory** is the operating-system process working directory.

For example:

```bash
cd ~/projects/foo/scripts
hql test.hql
```

gives:

```text
cwd = ~/projects/foo/scripts
```

It does not automatically mean that `scripts/` is an HQL workspace.

---

## Execution anchor

The **execution anchor** is the filesystem location from which automatic context discovery begins.

When executing a file:

```bash
hql /projects/foo/scripts/test.hql
```

the anchor is:

```text
/projects/foo/scripts
```

even if `hql` was invoked from another directory.

When evaluating stdin or an expression without a source file, the anchor is normally the current working directory.

When HQL is invoked by a host such as HyperMarkdown, the host may provide the execution anchor explicitly.

---

## Workspace

A **workspace** is the root of the HQL environment associated with an invocation.

A workspace establishes the local boundary within which workspace configuration and workspace-owned resources are interpreted.

The workspace can be discovered automatically or supplied explicitly.

The current working directory may be several levels below the workspace:

```text
repo/                       <- workspace
    hql.toml
    src/
        reports/
            daily/
                report.hql  <- execution anchor
```

Running:

```bash
cd repo/src/reports/daily
hql report.hql
```

still operates in:

```text
workspace = repo/
```

---

## Vault

A **vault** is a HyperMarkdown workspace identified by `.hmd`.

For HQL execution purposes, a discovered HyperMarkdown vault is also an HQL workspace.

For example:

```text
knowledge/
    .hmd/
    notes/
        architecture/
            review.hql
```

has:

```text
vault root     = knowledge/
workspace root = knowledge/
execution root = knowledge/notes/architecture/
```

The vault and workspace therefore normally coincide in this mode.

The fact that execution begins in a deeply nested folder does not create a nested workspace.

---

# 3. Workspace markers

HQL recognizes two workspace boundaries.

An ordinary HQL workspace is marked by:

```text
hql.toml
```

A HyperMarkdown vault is marked by:

```text
.hmd/
```

For example:

```text
project/
    hql.toml
```

is an HQL workspace.

And:

```text
vault/
    .hmd/
```

is a HyperMarkdown vault and therefore also an HQL workspace.

A directory may contain both:

```text
vault/
    .hmd/
    hql.toml
```

This represents one workspace, not two nested contexts.

`.hmd` establishes the vault identity.

`hql.toml` supplies HQL-specific workspace configuration.

A vault does not require `hql.toml`. If only `.hmd` exists, HQL uses the vault with default HQL workspace configuration plus system and user configuration.

---

# 4. Automatic workspace discovery

Unless disabled or overridden, HQL searches upward from the execution anchor.

For example:

```text
/a/
    .hmd/
    b/
        c/
            script.hql
```

executing:

```bash
hql /a/b/c/script.hql
```

discovers:

```text
/a/.hmd
```

and establishes:

```text
workspace = /a
vault     = /a
```

The current working directory is irrelevant to this discovery when the executed source file provides the anchor.

---

# 5. Nearest boundary wins

Automatic discovery walks ancestors upward until it reaches the nearest directory containing either:

```text
.hmd/
```

or:

```text
hql.toml
```

That directory becomes the workspace boundary.

Discovery stops there.

For example:

```text
outer/
    .hmd/

    inner/
        hql.toml

        src/
            program.hql
```

`program.hql` belongs to:

```text
inner/
```

as an HQL workspace.

The outer vault is not automatically merged into that workspace.

This prevents unrelated ancestor configuration from leaking through explicit workspace boundaries.

---

# 6. `.hmd` and `hql.toml` at the same boundary

If both are found in the same directory:

```text
project/
    .hmd/
    hql.toml
```

they describe one environment.

The mode is a HyperMarkdown vault workspace with explicit HQL configuration.

Conceptually:

```text
.hmd
    establishes vault/workspace identity

hql.toml
    configures HQL inside that workspace
```

There is no ambiguity and no precedence between the two.

---

# 7. Bare mode

HQL must also operate without any workspace semantics.

This is **bare mode**.

Examples include:

```bash
echo '{"a": 10}' | hql '...'
```

```bash
hql --bare script.hql
```

or ordinary invocation from a directory having no discoverable workspace boundary.

Bare mode has:

```text
workspace = none
vault     = none
```

It therefore performs no `.hmd` or workspace-relative discovery.

However:

> Bare mode does not mean an empty HQL installation.

System-wide and user-wide HQL resources still participate.

In particular, HQL still discovers its user environment under:

```text
~/.hql/
```

or the configured equivalent HQL home.

Therefore bare HQL can still use user-installed modules, extension registrations, configuration and other global resources.

This is deliberate.

A user should be able to configure HQL once and then use:

```bash
hql ...
```

anywhere without first creating a workspace.

---

# 8. HQL home

HQL has a user-level home directory.

By default:

```text
~/.hql/
```

Conceptually it may contain resources such as:

```text
~/.hql/
    config.toml
    modules/
    extensions/
    packages/
    ...
```

The precise storage layout of future package management is specified separately.

The important rule here is:

> User-level HQL configuration and resources exist independently of workspace discovery.

Therefore:

```text
Bare mode
```

still loads the applicable contents of:

```text
~/.hql/**
```

while:

```text
Workspace mode
```

combines those user resources with workspace resources.

`~/.hql` is not itself a workspace.

It is the user's HQL environment.

---

# 9. System configuration

HQL may also have machine-wide configuration.

Conceptually:

```text
system configuration
        ↓
user configuration
        ↓
workspace configuration
        ↓
invocation configuration
```

The actual system path is platform-dependent and is not part of HQL language semantics.

System configuration may define, for example:

```text
module roots
extension locations
named contexts
policy
defaults
```

System configuration applies in both bare and workspace execution unless explicitly superseded by a higher configuration layer.

---

# 10. Configuration layering

An execution context is assembled from several layers.

Conceptually:

```text
builtin defaults
        ↓
system HQL configuration
        ↓
user HQL configuration (~/.hql)
        ↓
selected named context
        ↓
discovered workspace configuration
        ↓
environment variables
        ↓
command-line options
```

Later layers may override ordinary scalar configuration from earlier layers.

This precedence does **not** imply module shadowing.

For example, configuration may result in two module roots containing:

```text
foo
```

The module registry still treats that as an ambiguity unless an explicit module override selects one provider.

Configuration precedence and module identity are separate systems.

---

# 11. HQL_PATH

`HQL_PATH` contributes additional module roots to the execution context.

For example:

```bash
export HQL_PATH="$HOME/dev/hql:/opt/acme/hql"
```

This applies in both workspace and bare execution.

Therefore:

```bash
cd /tmp
hql --bare script.hql
```

may still see modules from:

```text
~/.hql/modules
$HQL_PATH
```

in addition to builtin modules.

`HQL_PATH` adds discovery locations.

It does not establish a workspace and does not imply Python-style first-match shadowing.

---

# 12. Named contexts

HQL supports **named contexts** using a model similar to `kubectl`.

A named context is a reusable configuration entry describing an HQL environment.

Conceptually:

```text
contexts:
    default:
        ...

    work:
        ...

    experimental:
        ...
```

A context may select or contribute configuration such as:

```text
module roots
module overrides
extension environment
package environment
workspace defaults
other runtime configuration
```

The exact configuration schema belongs to the individual systems being configured.

The context mechanism only defines how those settings are grouped and selected.

---

# 13. Current context

User configuration may designate one named context as the default:

```text
current-context = work
```

This is analogous to:

```bash
kubectl config use-context ...
```

The user may inspect and change it through HQL configuration commands.

Conceptually:

```bash
hql config current-context

hql config get-contexts

hql config use-context work
```

Changing the current context changes persistent user configuration.

It does not modify project source files.

---

# 14. Per-invocation context

A context may also be selected only for one invocation:

```bash
hql --context experimental script.hql
```

This does not change `current-context`.

An environment variable may provide the same temporary selection:

```bash
HQL_CONTEXT=experimental hql script.hql
```

Selection precedence is:

```text
--context
    >
HQL_CONTEXT
    >
configured current-context
    >
default context
```

Workspace discovery happens after the named context is selected unless the selected context explicitly fixes the workspace.

---

# 15. Context is not workspace

A named context and a workspace are deliberately different concepts.

A context is configuration.

A workspace is a filesystem/environment boundary.

For example:

```text
context = work
workspace = ~/projects/project-a
```

and later:

```text
context = work
workspace = ~/projects/project-b
```

are both valid.

The same named context can therefore be reused across many workspaces.

This is important because otherwise HQL contexts would become little more than saved current-directory pointers.

---

# 16. Contexts may explicitly bind a workspace

For cases where that is useful, a named context may explicitly identify a workspace.

Conceptually:

```text
context: production-data

workspace:
    /srv/hql/production
```

When an explicitly selected context binds a workspace, that workspace is authoritative and upward filesystem discovery is not used to select another one.

This supports stable named environments in the same way that a kubectl context explicitly identifies a target environment.

An automatically selected default context should normally leave the workspace unspecified so ordinary local workspace discovery continues to work.

---

# 17. Explicit workspace selection

The CLI may explicitly select a workspace without creating or changing a named context.

Conceptually:

```bash
hql --workspace /path/to/project script.hql
```

This disables automatic upward workspace selection for that invocation.

The execution anchor may still differ from the workspace root.

For example:

```bash
hql \
  --workspace /repo \
  /repo/src/jobs/report.hql
```

has:

```text
workspace root = /repo
execution anchor = /repo/src/jobs
```

The explicit workspace must contain or otherwise satisfy the requirements of the selected workspace mode.

---

# 18. Explicit bare execution

The CLI may force bare mode:

```bash
hql --bare ...
```

This suppresses automatic discovery of:

```text
.hmd
hql.toml
```

even if they exist above the execution anchor.

For example:

```text
vault/
    .hmd/
    scripts/
        test.hql
```

normally executes `test.hql` in the vault workspace.

But:

```bash
hql --bare vault/scripts/test.hql
```

does not attach it to that vault.

It still loads:

```text
builtins
system HQL configuration
user HQL configuration
~/.hql resources
HQL_PATH
explicit CLI configuration
```

Bare means **no workspace**, not **no configuration**.

---

# 19. Conflicting explicit selections

Contradictory explicit execution instructions are errors rather than precedence puzzles.

For example:

```bash
hql --bare --workspace /repo script.hql
```

is invalid because one option requires a workspace while the other explicitly disables workspace semantics.

Likewise, if an explicitly selected named context requires one workspace while `--workspace` names another, HQL should report the conflict rather than silently selecting one.

The general rule is:

> Explicit contradictory context declarations are errors.

---

# 20. Execution modes

The resulting invocation can be described using three primary execution modes.

## Bare

```text
workspace = none
vault = none
```

Used for lightweight standalone HQL execution.

Example:

```bash
cat records.json | hql ...
```

---

## HQL workspace

```text
workspace = discovered or explicit
vault = none
```

Normally established by:

```text
hql.toml
```

Example:

```text
project/
    hql.toml
    src/
        job.hql
```

---

## Vault workspace

```text
workspace = vault root
vault = workspace
```

Established by:

```text
.hmd/
```

Example:

```text
knowledge/
    .hmd/
    cards/
    scripts/
```

An accompanying `hql.toml` may further configure HQL without changing the workspace identity.

---

# 21. Host-provided execution context

HQL may run inside another application, most importantly HyperMarkdown.

In that case the host already knows information that standalone filesystem discovery would otherwise need to infer.

For example, the VS Code HyperMarkdown extension may know:

```text
vault
current document
current card
cell location
workspace
```

The host may therefore provide the workspace and vault directly.

Conceptually:

```text
HyperMarkdown
      |
      | execution context
      v
     HQL
```

HQL should not walk ancestors looking for a competing vault when an authoritative host context has already supplied one.

Host-provided context is still combined with applicable system and user HQL configuration.

This keeps embedded HQL and CLI HQL on the same underlying context model.

---

# 22. The two execution universes

The distinction can be summarized as follows:

| | Bare HQL | Workspace / Vault HQL |
|---|---|---|
| Requires project | No | No explicit setup for vault; workspace marker exists |
| Workspace | None | Yes |
| `.hmd` semantics | None | Available in vault mode |
| `hql.toml` | Not discovered | Workspace configuration |
| `~/.hql/**` | Yes | Yes |
| System config | Yes | Yes |
| User config | Yes | Yes |
| `HQL_PATH` | Yes | Yes |
| Named context | Yes | Yes |
| Modules | Builtin + user/configured | Builtin + user/configured + workspace |
| Extensions | Global/configured | Global/configured + workspace |
| Relative workspace resources | None | Relative to workspace |
| Typical use | `jq`, `awk`, script | application, vault, project |

The language itself is the same in both modes.

Only the environment supplied to it differs.

---

# 23. Module discovery

This ADR does not define modules themselves.

It provides inputs to the module-discovery system.

For example, after context construction HQL may have:

```text
workspace module roots
user module roots
system module roots
HQL_PATH roots
package module roots
builtin contracts
explicit module overrides
```

Those are passed to module discovery.

Conceptually:

```text
ExecutionContext
        |
        v
effective module roots
        |
        v
Module Discovery
        |
        v
Module Registry
```

The execution-context layer does not resolve imports.

---

# 24. Extension discovery

The same applies to extensions.

An execution context may expose:

```text
builtin extensions
user extensions
workspace extensions
package-provided extensions
explicit extension configuration
```

Those are inputs to the extension subsystem.

Conceptually:

```text
ExecutionContext
        |
        v
extension locations/config
        |
        v
Extension Registry
```

The execution-context system does not define the Rust ABI or implementation mechanics of an extension.

---

# 25. Packages

Package management is similarly downstream.

A future package environment may be selected by:

```text
user configuration
named context
workspace configuration
```

The package resolver then determines the concrete packages available to the invocation.

Those packages may contribute:

```text
module contracts
extensions
other resources
```

The package manager does not determine workspace identity.

Conversely, workspace discovery does not itself resolve packages.

---

# 26. Relative paths

Relative paths should be interpreted relative to the configuration source that declares them.

For example, in:

```text
/repo/hql.toml
```

a configured module root:

```text
./lib
```

means:

```text
/repo/lib
```

A relative path in:

```text
~/.hql/config.toml
```

is relative to the applicable user configuration location.

Command-line filesystem arguments remain relative to the process working directory unless their option explicitly documents different behavior.

This avoids configuration changing meaning depending on the directory from which HQL happens to be invoked.

---

# 27. Context construction

At a high level, standalone HQL constructs an execution context as follows:

```text
invocation
    |
    v
determine execution anchor
    |
    v
load builtin defaults
    |
    v
load system configuration
    |
    v
load ~/.hql user environment
    |
    v
select named context
    |
    v
determine execution mode
    |
    +---- --bare -----------> no workspace
    |
    +---- explicit workspace
    |
    `---- automatic upward discovery
              |
              +-- nearest hql.toml -> HQL workspace
              |
              +-- nearest .hmd     -> Vault workspace
              |
              `-- none             -> Bare
    |
    v
load workspace configuration if present
    |
    v
apply environment contributions
    |
    v
apply command-line configuration
    |
    v
ExecutionContext
```

Embedded execution follows the same model except that the host may supply the execution anchor, workspace, vault and other contextual information directly.

---

# 28. Context object

The resulting runtime object is conceptually something like:

```text
ExecutionContext {
    mode
    cwd
    anchor

    workspace
    vault

    hql_home

    config
    selected_context

    module_roots
    module_overrides

    extension_environment
    package_environment

    host_context
}
```

This is an architectural description, not a required Rust representation.

Subsystems consume only the parts they need.

For example:

```text
module loader
    -> module_roots, module_overrides

extension loader
    -> extension_environment

HyperMarkdown runtime
    -> vault, host_context

filesystem operations
    -> cwd / workspace according to operation semantics
```

---

# 29. No ambient workspace requirement

Library code must not assume:

```text
workspace != none
```

because valid HQL programs may run in bare mode.

Workspace-dependent operations must either:

```text
receive the required context
```

or fail explicitly when invoked without one.

For example, functionality meaning:

```text
search current vault
```

cannot silently reinterpret "current vault" as the current directory in bare mode.

It should instead report that no vault is active.

This keeps bare HQL predictable.

---

# 30. No implicit vault semantics outside a vault

Likewise, the existence of HQL user configuration does not create a HyperMarkdown vault.

For example:

```text
~/.hql/
```

may contain modules, extensions and configuration.

It does not imply:

```text
vault = ~/.hql
```

A vault exists only when explicitly provided by a host/context or discovered through `.hmd`.

This distinction is important because HQL is a standalone language, not merely the query language of HyperMarkdown.

---

# 31. Context inspection

Because context construction can combine several sources, it must be inspectable.

The CLI should expose commands conceptually equivalent to:

```bash
hql config current-context
hql config get-contexts
hql config use-context NAME
hql config view
```

and an inspection facility for the effective invocation context.

For example:

```bash
hql context
```

could report:

```text
mode:       vault
context:    work
workspace:  /home/user/knowledge
vault:      /home/user/knowledge
anchor:     /home/user/knowledge/projects/hql
hql-home:   /home/user/.hql
```

It may additionally show effective module roots, extension sources and configuration origins.

The exact command names are CLI design details, but inspection is a requirement.

Configuration must not become invisible ambient state.

---

# 32. Context selection must be deterministic

Given the same:

```text
source location
filesystem boundaries
system configuration
user configuration
named context
workspace configuration
environment
CLI arguments
```

HQL must construct the same execution context.

There must be no dependency on arbitrary directory enumeration order or module search order.

All automatically discovered decisions should be explainable through context inspection.

---

# 33. Interaction with `.hmd` inheritance

HyperMarkdown may later define cascading or inherited behavior inside a vault.

For example, folders may contribute local handlers, event rules, indexes or other HMD semantics.

That is separate from HQL workspace discovery.

This ADR only determines:

```text
which vault owns this execution?
```

Once the vault is known, HyperMarkdown may independently determine contextual behavior within that vault.

Walking from a card toward its vault root for HMD rules is therefore not the same operation as walking ancestors to discover the workspace boundary.

The two mechanisms must not be conflated.

---

# 34. Current state

Current HQL behavior does not yet expose this complete execution-context model.

HyperMarkdown already has the concept of a vault represented by `.hmd`.

HQL has standalone CLI execution and an extension registry, but workspace discovery, named contexts and unified configuration layering are not yet formalized as one compiler/runtime abstraction.

Existing `init.hql`-based namespace/event ideas belong to module or HyperMarkdown semantics and should not be responsible for discovering the execution workspace.

---

# 35. Decisions

This ADR therefore establishes the following model.

1. HQL supports standalone bare execution and workspace-aware execution as first-class modes.

2. `.hmd` identifies a HyperMarkdown vault and also establishes an HQL workspace boundary.

3. `hql.toml` identifies/configures a plain HQL workspace.

4. If `.hmd` and `hql.toml` occur in the same directory, they describe one vault workspace.

5. Automatic workspace discovery walks upward from the execution anchor and stops at the nearest workspace boundary.

6. The source file's directory is the normal discovery anchor when running a source file; CWD is not implicitly the workspace root.

7. HQL can be forced into bare mode.

8. Bare mode disables workspace/vault discovery but still loads system configuration, user configuration, `~/.hql/**`, `HQL_PATH` and other explicitly configured global resources.

9. User and system configuration exist independently of workspaces.

10. Named contexts provide kubectl-like reusable environment configuration.

11. A persistent `current-context` may select the default named context.

12. `--context` or its environment equivalent may select another context for one invocation.

13. Context and workspace are independent concepts. A context normally configures an environment rather than naming one filesystem project.

14. A context may optionally pin an explicit workspace when that behavior is intentionally required.

15. Configuration layering does not imply module shadowing. Module ambiguity remains the responsibility of the module registry.

16. Hosts such as HyperMarkdown may provide authoritative context directly instead of relying on filesystem discovery.

17. Module discovery, extension discovery and package resolution consume the execution context but remain separate subsystems.

---

# 36. Work units

### Execution context

Introduce an internal representation for:

```text
ExecutionContext
ExecutionMode
Workspace
Vault
ExecutionAnchor
HqlHome
NamedContext
```

### Workspace discovery

Implement upward discovery for:

```text
.hmd/
hql.toml
```

with nearest-boundary semantics.

### Bare mode

Support explicit bare execution and ensure user/system HQL resources remain available.

### User environment

Formalize:

```text
~/.hql/
```

as HQL user home and define which resource categories are discoverable from it.

### Configuration

Introduce configuration loading and layering for:

```text
system
user
named context
workspace
environment
CLI
```

### Named contexts

Provide kubectl-style operations for:

```text
list
inspect
select
current
```

contexts.

### Module integration

Expose from `ExecutionContext` the effective:

```text
module roots
HQL_PATH contributions
module overrides
```

to the module registry builder.

### Extension integration

Expose the effective extension configuration to the extension registry.

### Host integration

Allow HyperMarkdown and other hosts to construct or supplement an execution context explicitly.

### Diagnostics

Provide an inspection command capable of explaining how the effective context was constructed.