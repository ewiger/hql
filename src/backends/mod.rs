//! Execution backends: what a checked program is run by, and which one runs
//! it.
//!
//! A backend receives the checked execution tree and nothing earlier, and is
//! held to the value, the type, the sequence positions and the diagnostic
//! `localhost` gives. Backends are compiled in and registered in one table,
//! as extensions are. Which backend runs a program is host configuration —
//! the query's `--backend`, the vault's `[execution] backend`, the user's
//! `HQL_BACKEND`, then the default — and never syntax, because the answer
//! does not depend on it. See `doc/proposals/HQL-0004/README.md`.
//!
//! Before a backend accepts a program, the planner annotates every step with
//! its stage, from the step's shape and purity and from what the backend
//! executes, and a backend refuses a pipeline a step of which carries no
//! annotation. See `doc/proposals/HQL-0005/README.md`.

mod localhost;
mod plan;

pub use plan::{Execution, Stage, Strategy, topology};

use crate::diagnostics::Diagnostic;
use crate::execution::Program;
use crate::reporting::{CONFIG_FILE, Mode};
use crate::types::Value;
use crate::vault::Vault;
use std::path::Path;
use std::{env, fs};

/// What executes a checked program.
pub(crate) trait Backend: Sync {
    /// What it executes, which the planner reads to cut stages.
    fn execution(&self) -> Execution;

    /// Evaluate a checked program against a vault, with further names bound:
    /// the value, or the diagnostic `localhost` would report, and the log the
    /// run wrote.
    fn run(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
        mode: Mode,
    ) -> (Result<Value, Diagnostic>, Vec<Diagnostic>);
}

/// One registered backend: its name, what it is, and what runs a program on
/// it.
pub struct Registered {
    /// The name `--backend`, `hql.toml` and `HQL_BACKEND` write.
    pub name: &'static str,
    /// One sentence about how it executes a program.
    pub summary: &'static str,
    backend: &'static dyn Backend,
}

impl std::fmt::Debug for Registered {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Registered")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl Registered {
    /// Plan a checked program for this backend, then run it.
    pub(crate) fn run(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
        mode: Mode,
    ) -> (Result<Value, Diagnostic>, Vec<Diagnostic>) {
        self.plan(program);
        self.execute(program, vault, bound, mode)
    }

    /// Annotate every step of a checked program with its stage on this
    /// backend.
    pub(crate) fn plan(&self, program: &Program) {
        plan::plan(program, self.backend.execution());
    }

    /// What this backend executes.
    #[must_use]
    pub fn execution(&self) -> Execution {
        self.backend.execution()
    }

    /// Run a planned program, refusing one with a step that carries no
    /// stage annotation before any part of it runs.
    pub(crate) fn execute(
        &self,
        program: &Program,
        vault: &Vault,
        bound: Vec<(String, Value)>,
        mode: Mode,
    ) -> (Result<Value, Diagnostic>, Vec<Diagnostic>) {
        if let Some(refused) = plan::refusal(program, self.name) {
            return (
                Err(refused
                    .in_phase(crate::diagnostics::Phase::Plan)
                    .with_kind(crate::diagnostics::Kind::BackendRefusal)),
                Vec::new(),
            );
        }
        self.backend.run(program, vault, bound, mode)
    }
}

/// The standard backend: the in-memory evaluator, always registered, the
/// default and the oracle every other backend is measured against.
pub static LOCALHOST: Registered = Registered {
    name: "localhost",
    summary: "The in-memory evaluator: each step run to completion before the next, \
              one traversal per step, with no fusion, partitioning or parallelism.",
    backend: &localhost::Localhost,
};

/// Every backend compiled into this binary, `localhost` first.
pub static REGISTERED: &[&Registered] = &[&LOCALHOST];

/// The registered backend of a name.
#[must_use]
pub fn named(name: &str) -> Option<&'static Registered> {
    REGISTERED
        .iter()
        .copied()
        .find(|backend| backend.name == name)
}

/// The environment variable a user sets for their own default backend.
pub const ENVIRONMENT: &str = "HQL_BACKEND";

/// The backend in force, and which layer supplied it.
#[derive(Debug, Clone, Copy)]
pub struct Selection {
    /// The backend that runs programs.
    pub backend: &'static Registered,
    /// Which layer named it, for `hql config`.
    pub origin: Origin,
}

/// The layer a backend selection came from, most specific first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// `--backend` on this query.
    Query,
    /// The vault's `hql.toml`, `[execution] backend`.
    Vault,
    /// `HQL_BACKEND` in the environment.
    Environment,
    /// Nothing named one.
    Default,
}

impl Origin {
    /// How `hql config` and a refusal name the layer.
    #[must_use]
    pub fn label(self) -> String {
        match self {
            Self::Query => "--backend".to_owned(),
            Self::Vault => format!("the vault's {CONFIG_FILE}"),
            Self::Environment => format!("${ENVIRONMENT}"),
            Self::Default => "the default".to_owned(),
        }
    }
}

impl Selection {
    /// `localhost`, selected by default.
    #[must_use]
    pub fn standard() -> Self {
        Self {
            backend: &LOCALHOST,
            origin: Origin::Default,
        }
    }

    /// Resolve the backend from the query, the vault and the environment,
    /// with the precedence the reporting settings use: the most specific
    /// layer that names one wins.
    ///
    /// # Errors
    ///
    /// A name that is not registered fails, naming the layer that supplied
    /// it, so that every program run under it fails rather than running
    /// somewhere else.
    pub fn resolve(query: Option<&str>, vault_root: Option<&Path>) -> Result<Self, String> {
        let from_vault = || {
            let text = fs::read_to_string(vault_root?.join(CONFIG_FILE)).ok()?;
            let parsed: toml::Value = toml::from_str(&text).ok()?;
            Some(
                parsed
                    .get("execution")?
                    .get("backend")?
                    .as_str()?
                    .to_owned(),
            )
        };
        let (name, origin) = if let Some(name) = query {
            (name.to_owned(), Origin::Query)
        } else if let Some(name) = from_vault() {
            (name, Origin::Vault)
        } else if let Ok(name) = env::var(ENVIRONMENT) {
            (name, Origin::Environment)
        } else {
            return Ok(Self::standard());
        };
        Self::of(&name, origin)
    }

    /// The registered backend of a name, supplied by a layer.
    ///
    /// # Errors
    ///
    /// The name is not registered; the message names the layer.
    pub fn of(name: &str, origin: Origin) -> Result<Self, String> {
        let backend = named(name).ok_or_else(|| {
            let known: Vec<String> = REGISTERED
                .iter()
                .map(|backend| format!("`{}`", backend.name))
                .collect();
            format!(
                "no backend is named `{name}`, which {} names; the registered backends are {}",
                origin.label(),
                known.join(", ")
            )
        })?;
        Ok(Self { backend, origin })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localhost_is_registered_first_and_is_the_default() {
        assert_eq!(REGISTERED[0].name, "localhost");
        assert!(named("localhost").is_some());
        assert!(named("threads").is_none());
        let selection = Selection::standard();
        assert_eq!(selection.backend.name, "localhost");
        assert_eq!(selection.origin, Origin::Default);
    }

    #[test]
    fn an_unregistered_name_fails_naming_the_layer_that_supplied_it() {
        for (origin, layer) in [
            (Origin::Query, "--backend"),
            (Origin::Vault, "hql.toml"),
            (Origin::Environment, "$HQL_BACKEND"),
        ] {
            let refused = Selection::of("postgres", origin).unwrap_err();
            assert!(refused.contains("`postgres`"), "{refused}");
            assert!(refused.contains(layer), "{refused}");
            assert!(refused.contains("`localhost`"), "{refused}");
        }
    }

    fn checked(source: &str) -> Program {
        crate::checker::check(&crate::parser::parse(source).unwrap(), &Vault::empty()).unwrap()
    }

    /// A backend that cuts stages by shape and evaluates as `localhost`
    /// does: what is planned differs, and nothing else.
    struct Splitting;

    impl Backend for Splitting {
        fn execution(&self) -> Execution {
            Execution::Partitioned
        }

        fn run(
            &self,
            program: &Program,
            vault: &Vault,
            bound: Vec<(String, Value)>,
            mode: Mode,
        ) -> (Result<Value, Diagnostic>, Vec<Diagnostic>) {
            crate::evaluation::evaluate_with(program, vault, bound, mode)
        }
    }

    static SPLITTING: Registered = Registered {
        name: "splitting",
        summary: "Stages cut by shape, evaluated in memory.",
        backend: &Splitting,
    };

    const PROGRAMS: [&str; 4] = [
        "[3, 1, 2] | filter(n => n > 1) | map(n => n * 2) | sort | take(1)",
        r#"["owl", "raven", "owl"] | group(by = w => w) | map(g => size(g.items)) | sum"#,
        "doubled : Elementwise<Int, Int> = n => n * 2\n[1, 2, 3] | doubled | sum",
        "[1, 2] | map(n => [n, n] | map(m => m + 1) | sum) | markdown",
    ];

    #[test]
    fn a_backend_refuses_a_pipeline_that_is_not_planned() {
        let program = checked("[1, 2, 3] | map(n => n + 1) | size");
        let (value, _) = LOCALHOST.execute(&program, &Vault::empty(), Vec::new(), Mode::Strict);
        let refused = value.unwrap_err().to_string();
        assert!(refused.contains("`localhost`"), "{refused}");
        assert!(refused.contains("stage annotation"), "{refused}");
        // Planned, the same program runs.
        LOCALHOST.plan(&program);
        let (value, _) = LOCALHOST.execute(&program, &Vault::empty(), Vec::new(), Mode::Strict);
        assert_eq!(value.unwrap(), Value::Int(3));
    }

    #[test]
    fn the_localhost_plan_annotates_every_step_with_one_stage() {
        for source in PROGRAMS {
            let program = checked(source);
            LOCALHOST.plan(&program);
            let ends = program.pipelines();
            assert!(!ends.is_empty(), "{source}");
            for end in ends {
                for plan in end.steps().0 {
                    assert_eq!(
                        plan.stage.get(),
                        Some(&Stage {
                            index: 0,
                            strategy: Strategy::InProcess
                        }),
                        "{source}: {}",
                        plan.invokes.name
                    );
                }
            }
        }
    }

    #[test]
    fn annotations_do_not_change_the_value_a_pipeline_produces() {
        for source in PROGRAMS {
            let on_localhost = checked(source);
            let split = checked(source);
            let (expected, _) =
                LOCALHOST.run(&on_localhost, &Vault::empty(), Vec::new(), Mode::Strict);
            let (produced, _) = SPLITTING.run(&split, &Vault::empty(), Vec::new(), Mode::Strict);
            assert_eq!(produced.unwrap(), expected.unwrap(), "{source}");
            // The plans differ, so the agreement is not a tautology.
            let stages = |program: &Program| -> Vec<Stage> {
                program
                    .pipelines()
                    .into_iter()
                    .flat_map(|end| end.steps().0)
                    .filter_map(|plan| plan.stage.get().copied())
                    .collect()
            };
            assert_ne!(stages(&on_localhost), stages(&split), "{source}");
        }
    }

    #[test]
    fn the_query_outranks_the_vault() {
        let vault = tempfile::tempdir().unwrap();
        fs::write(
            vault.path().join(CONFIG_FILE),
            "[execution]\nbackend = \"nonesuch\"\n",
        )
        .unwrap();
        let refused = Selection::resolve(None, Some(vault.path())).unwrap_err();
        assert!(refused.contains("hql.toml"), "{refused}");
        let chosen = Selection::resolve(Some("localhost"), Some(vault.path())).unwrap();
        assert_eq!(chosen.origin, Origin::Query);
        fs::write(
            vault.path().join(CONFIG_FILE),
            "[execution]\nbackend = \"localhost\"\n",
        )
        .unwrap();
        let chosen = Selection::resolve(None, Some(vault.path())).unwrap();
        assert_eq!(chosen.origin, Origin::Vault);
        assert_eq!(chosen.backend.name, "localhost");
    }
}
