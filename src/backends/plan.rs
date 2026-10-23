//! The planner: what makes a checked program a plan a backend accepts.
//!
//! A plan is the checked program itself with every step node carrying a
//! stage annotation; the steps, their order and their operations are
//! unchanged. The planner derives each annotation from the operation type a
//! step invokes, its shape and its purity, and from what the backend
//! executes. Nothing in the program names a stage, and an annotation never
//! changes the value a pipeline produces. See
//! `doc/proposals/HQL-0005/README.md`.

use crate::diagnostics::Diagnostic;
use crate::execution::{OperationType, Program};
use crate::extensions::{Purity, Shape};

/// How the steps of one stage are executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// In process, one step after another, each run to completion.
    InProcess,
    /// Pure elementwise steps fused into one traversal, closed by at most
    /// one reduction: any split of the input may run separately.
    Parallel,
    /// The input repartitioned by a key, and the pure steps after it run
    /// per group.
    Partitioned,
    /// The whole input assembled before the step runs: a barrier.
    Assembled,
    /// An effectful step, run once, where it is written.
    Effect,
    /// A presentation of the value the pipeline produced.
    Presentation,
}

impl Strategy {
    /// The words `hql plan` prints for this strategy.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::InProcess => "in process",
            Self::Parallel => "parallel",
            Self::Partitioned => "partitioned",
            Self::Assembled => "assembled",
            Self::Effect => "effect",
            Self::Presentation => "presentation",
        }
    }
}

/// A step's stage annotation: the stage of its pipeline it belongs to,
/// counted from `0`, and how that stage is executed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage {
    /// Which stage of the pipeline, in pipeline order.
    pub index: usize,
    /// How the stage is executed.
    pub strategy: Strategy,
}

/// The dataflow topology of an operation (HQL-0037): `fan-out` for one
/// input read by several branches, `fan-in` for several inputs joined into
/// one, and none for an ordinary `1 → 1` step.
#[must_use]
pub fn topology(operation: &str) -> Option<&'static str> {
    match operation {
        "fork" | "tee" => Some("fan-out"),
        "merge" | "zip" | "concat" | "join" | "left_join" => Some("fan-in"),
        _ => None,
    }
}

/// What a backend executes, which the planner reads to cut stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Execution {
    /// The whole pipeline in process, one step after another: one stage.
    InProcess,
    /// Stages split by shape: adjacent pure elementwise steps fused, a
    /// partition starting a stage, and a barrier, an effect or a
    /// presentation each a stage of its own.
    Partitioned,
}

/// The stage of each step of one pipeline, first to last. The steps of one
/// stage are contiguous.
#[must_use]
pub(crate) fn stages(steps: &[&OperationType], execution: Execution) -> Vec<Stage> {
    match execution {
        Execution::InProcess => steps
            .iter()
            .map(|_| Stage {
                index: 0,
                strategy: Strategy::InProcess,
            })
            .collect(),
        Execution::Partitioned => partitioned(steps),
    }
}

/// Cut a pipeline by the permissions its shapes grant: a run of pure
/// elementwise steps is one stage, which one pure reduction may close; a
/// partition opens a stage the pure elementwise and reduction steps after it
/// join; a barrier, an effectful step and a presentation end whatever stage
/// is open and are each a stage of their own.
fn partitioned(steps: &[&OperationType]) -> Vec<Stage> {
    let mut stages: Vec<Stage> = Vec::new();
    // The stage the next step may join, if any is still open.
    let mut open: Option<Stage> = None;
    for step in steps {
        let pure = step.purity == Purity::Pure && !step.presentation;
        let joins = match (open, step.shape) {
            (Some(stage), Shape::Elementwise | Shape::Reduction) if pure => {
                matches!(stage.strategy, Strategy::Parallel | Strategy::Partitioned)
            }
            _ => false,
        };
        if let (true, Some(stage)) = (joins, open) {
            stages.push(stage);
            // A reduction closes a parallel run: its splits are combined.
            if stage.strategy == Strategy::Parallel && step.shape == Shape::Reduction {
                open = None;
            }
            continue;
        }
        let index = stages.last().map_or(0, |last| last.index + 1);
        let strategy = if step.presentation {
            Strategy::Presentation
        } else if step.purity == Purity::Effectful {
            Strategy::Effect
        } else {
            match step.shape {
                Shape::Elementwise | Shape::Reduction => Strategy::Parallel,
                Shape::Partition => Strategy::Partitioned,
                Shape::Barrier => Strategy::Assembled,
            }
        };
        let stage = Stage { index, strategy };
        stages.push(stage);
        open = match (strategy, step.shape) {
            (Strategy::Parallel, Shape::Elementwise) | (Strategy::Partitioned, _) => Some(stage),
            _ => None,
        };
    }
    stages
}

/// Annotate every step of every pipeline of a program for a backend. A
/// module's program shared between importers keeps the annotation its first
/// plan wrote.
pub(crate) fn plan(program: &Program, execution: Execution) {
    for end in program.pipelines() {
        let (steps, _) = end.steps();
        let invoked: Vec<&OperationType> = steps.iter().map(|plan| &plan.invokes).collect();
        for (plan, stage) in steps.iter().zip(stages(&invoked, execution)) {
            let _ = plan.stage.set(stage);
        }
    }
}

/// Why a backend refuses a program: the first step of a pipeline that
/// carries no stage annotation. A pipeline is accepted for execution only
/// once planned; there is no unplanned execution path.
pub(crate) fn refusal(program: &Program, backend: &str) -> Option<Diagnostic> {
    program.pipelines().into_iter().find_map(|end| {
        let mut at = end;
        while let Some((plan, input)) = at.as_step() {
            if plan.stage.get().is_none() {
                return Some(Diagnostic::runtime(
                    at.span.clone(),
                    format!(
                        "the `{backend}` backend refuses this pipeline: its step `{}` carries \
                         no stage annotation, and a pipeline runs only once it is planned",
                        plan.invokes.name
                    ),
                ));
            }
            at = input;
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(name: &str, shape: Shape, purity: Purity, presentation: bool) -> OperationType {
        OperationType {
            name: name.to_owned(),
            shape,
            purity,
            presentation,
        }
    }

    #[test]
    fn in_process_execution_is_one_stage() {
        let steps = [
            operation("filter", Shape::Elementwise, Purity::Pure, false),
            operation("sort", Shape::Barrier, Purity::Pure, false),
            operation("markdown", Shape::Barrier, Purity::Pure, true),
        ];
        let steps: Vec<&OperationType> = steps.iter().collect();
        let planned = stages(&steps, Execution::InProcess);
        assert_eq!(planned.len(), 3);
        assert!(planned.iter().all(|stage| *stage
            == Stage {
                index: 0,
                strategy: Strategy::InProcess
            }));
    }

    #[test]
    fn partitioned_execution_fuses_elementwise_steps_and_a_barrier_ends_a_stage() {
        let steps = [
            operation("filter", Shape::Elementwise, Purity::Pure, false),
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("group", Shape::Partition, Purity::Pure, false),
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("sort", Shape::Barrier, Purity::Pure, false),
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("size", Shape::Reduction, Purity::Pure, false),
            operation("write", Shape::Barrier, Purity::Effectful, false),
            operation("markdown", Shape::Barrier, Purity::Pure, true),
        ];
        let steps: Vec<&OperationType> = steps.iter().collect();
        let planned: Vec<(usize, Strategy)> = stages(&steps, Execution::Partitioned)
            .into_iter()
            .map(|stage| (stage.index, stage.strategy))
            .collect();
        assert_eq!(
            planned,
            [
                (0, Strategy::Parallel),
                (0, Strategy::Parallel),
                (1, Strategy::Partitioned),
                (1, Strategy::Partitioned),
                (2, Strategy::Assembled),
                (3, Strategy::Parallel),
                (3, Strategy::Parallel),
                (4, Strategy::Effect),
                (5, Strategy::Presentation),
            ]
        );
    }

    /// `partition(by = …)` is a stage boundary (HQL-0036): the elementwise
    /// steps before it are one stage and those after it join its own.
    #[test]
    fn partition_by_key_opens_a_stage() {
        let steps = [
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("partition", Shape::Partition, Purity::Pure, false),
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("sum", Shape::Reduction, Purity::Pure, false),
        ];
        let steps: Vec<&OperationType> = steps.iter().collect();
        let planned: Vec<(usize, Strategy)> = stages(&steps, Execution::Partitioned)
            .into_iter()
            .map(|stage| (stage.index, stage.strategy))
            .collect();
        assert_eq!(
            planned,
            [
                (0, Strategy::Parallel),
                (1, Strategy::Partitioned),
                (1, Strategy::Partitioned),
                (1, Strategy::Partitioned),
            ]
        );
    }

    #[test]
    fn an_impure_elementwise_step_is_not_fused() {
        let steps = [
            operation("map", Shape::Elementwise, Purity::Pure, false),
            operation("log", Shape::Elementwise, Purity::Effectful, false),
            operation("map", Shape::Elementwise, Purity::Pure, false),
        ];
        let steps: Vec<&OperationType> = steps.iter().collect();
        let indices: Vec<usize> = stages(&steps, Execution::Partitioned)
            .into_iter()
            .map(|stage| stage.index)
            .collect();
        assert_eq!(indices, [0, 1, 2]);
    }
}
