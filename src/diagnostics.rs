//! The diagnostic: a structured event describing a failure, or a fact about
//! one, that is not data (HQL-0045).
//!
//! A diagnostic is produced by a phase — the loader, the parser, the checker,
//! the planner, the evaluator, the renderer — and never by a program. Its four
//! axes, `phase`, `kind`, `severity` and `disposition`, are fields set by the
//! producer that knows them; none is derived from another. Spans are
//! zero-based, half-open UTF-8 byte ranges into `origin`.

use std::collections::BTreeMap;
use std::ops::Range;

/// When a diagnostic occurred: the phase of its producer (HQL-0044).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Phase {
    /// Reading the documents of a vault.
    Load,
    /// Building the syntax tree.
    Parse,
    /// Resolving names and types over the whole program.
    Check,
    /// A backend planning a checked program.
    Plan,
    /// Evaluating a checked program.
    Execute,
    /// Writing a document's cells.
    Render,
}

impl Phase {
    /// Every phase, in the order a run passes through them.
    pub const ALL: [Self; 6] = [
        Self::Load,
        Self::Parse,
        Self::Check,
        Self::Plan,
        Self::Execute,
        Self::Render,
    ];

    /// The phase a lowercase word names.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|phase| phase.label() == label)
    }

    /// The lowercase word a diagnostic is written with.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Parse => "parse",
            Self::Check => "check",
            Self::Plan => "plan",
            Self::Execute => "execute",
            Self::Render => "render",
        }
    }
}

/// What the problem is, by name: a kebab-case word stable across releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    /// A list, set, map or record literal is not closed.
    UnclosedCollection,
    /// The source is outside the grammar where a token was read.
    UnexpectedToken,
    /// A name, type, module or function does not resolve.
    UnknownName,
    /// A pipeline step does not exist.
    UnknownStep,
    /// A record or value has no such field.
    UnknownField,
    /// An expression has a type its position does not accept.
    TypeMismatch,
    /// A match does not cover every value of its subject.
    NonExhaustiveMatch,
    /// An implementation lacks a function its trait requires.
    MissingImplementation,
    /// A trait is implemented twice for one type.
    DuplicateImplementation,
    /// A type does not satisfy a bound a call needs.
    UnsatisfiedBound,
    /// More than one declaration fits a call.
    AmbiguousCall,
    /// Modules import each other in a cycle.
    ImportCycle,
    /// A value does not have the shape a step needs.
    ShapeMismatch,
    /// A value does not read as the type it is read as: `x | T`.
    ContractViolation,
    /// A division or remainder by zero.
    DivisionByZero,
    /// A number left the range of its type.
    NumericOverflow,
    /// Reading or writing outside the program failed.
    IoFailure,
    /// A source refused what was asked of it.
    SourceRefused,
    /// Text did not parse in the format it was read in.
    MalformedData,
    /// An argument's value is outside what the operation takes.
    InvalidArgument,
    /// A document reference does not resolve in the vault.
    UnresolvedLink,
    /// A document was not loaded.
    SkippedDocument,
    /// A backend refused a pipeline.
    BackendRefusal,
    /// Two cells of a document share an id, or a cell's attributes are wrong.
    DuplicateCell,
    /// The cells of a document read each other in a cycle.
    CellCycle,
    /// An output in a document has no cell above it.
    OrphanedOutput,
    /// A type test holds for every value it can see.
    AlwaysTrueTest,
    /// A declaration shadows a step of the same name.
    ShadowedStep,
    /// A semantic index was built from other documents.
    StaleIndex,
    /// A name departs from the convention for what it names.
    NamingConvention,
    /// A module imports its own child.
    ChildImport,
    /// A boundary mapped a value with a loss worth saying.
    BoundaryMapping,
    /// The signature checking inferred for a declaration.
    Signature,
    /// What a function reads or writes outside its result.
    Effect,
}

impl Kind {
    /// Every kind, the set published with the reference.
    pub const ALL: [Self; 34] = [
        Self::UnclosedCollection,
        Self::UnexpectedToken,
        Self::UnknownName,
        Self::UnknownStep,
        Self::UnknownField,
        Self::TypeMismatch,
        Self::NonExhaustiveMatch,
        Self::MissingImplementation,
        Self::DuplicateImplementation,
        Self::UnsatisfiedBound,
        Self::AmbiguousCall,
        Self::ImportCycle,
        Self::ShapeMismatch,
        Self::ContractViolation,
        Self::DivisionByZero,
        Self::NumericOverflow,
        Self::IoFailure,
        Self::SourceRefused,
        Self::MalformedData,
        Self::InvalidArgument,
        Self::UnresolvedLink,
        Self::SkippedDocument,
        Self::BackendRefusal,
        Self::DuplicateCell,
        Self::CellCycle,
        Self::OrphanedOutput,
        Self::AlwaysTrueTest,
        Self::ShadowedStep,
        Self::StaleIndex,
        Self::NamingConvention,
        Self::ChildImport,
        Self::BoundaryMapping,
        Self::Signature,
        Self::Effect,
    ];

    /// The kind a kebab-case name names.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.label() == label)
    }

    /// The kebab-case name.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::UnclosedCollection => "unclosed-collection",
            Self::UnexpectedToken => "unexpected-token",
            Self::UnknownName => "unknown-name",
            Self::UnknownStep => "unknown-step",
            Self::UnknownField => "unknown-field",
            Self::TypeMismatch => "type-mismatch",
            Self::NonExhaustiveMatch => "non-exhaustive-match",
            Self::MissingImplementation => "missing-implementation",
            Self::DuplicateImplementation => "duplicate-implementation",
            Self::UnsatisfiedBound => "unsatisfied-bound",
            Self::AmbiguousCall => "ambiguous-call",
            Self::ImportCycle => "import-cycle",
            Self::ShapeMismatch => "shape-mismatch",
            Self::ContractViolation => "contract-violation",
            Self::DivisionByZero => "division-by-zero",
            Self::NumericOverflow => "numeric-overflow",
            Self::IoFailure => "io-failure",
            Self::SourceRefused => "source-refused",
            Self::MalformedData => "malformed-data",
            Self::InvalidArgument => "invalid-argument",
            Self::UnresolvedLink => "unresolved-link",
            Self::SkippedDocument => "skipped-document",
            Self::BackendRefusal => "backend-refusal",
            Self::DuplicateCell => "duplicate-cell",
            Self::CellCycle => "cell-cycle",
            Self::OrphanedOutput => "orphaned-output",
            Self::AlwaysTrueTest => "always-true-test",
            Self::ShadowedStep => "shadowed-step",
            Self::StaleIndex => "stale-index",
            Self::NamingConvention => "naming-convention",
            Self::ChildImport => "child-import",
            Self::BoundaryMapping => "boundary-mapping",
            Self::Signature => "signature",
            Self::Effect => "effect",
        }
    }
}

/// How serious a diagnostic is for the person reading it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// A fact worth seeing only when looking closely.
    Debug,
    /// A fact the run states.
    Info,
    /// The work stands, and something is worth saying.
    Warning,
    /// The result is wrong.
    Error,
    /// No result could be produced.
    Failure,
}

impl Severity {
    /// The lowercase word used in output and in configuration.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warning => "warning",
            Self::Error => "error",
            Self::Failure => "failure",
        }
    }

    /// The severity a lowercase word names, `failure` included.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        [
            Self::Debug,
            Self::Info,
            Self::Warning,
            Self::Error,
            Self::Failure,
        ]
        .into_iter()
        .find(|severity| severity.label() == label)
    }

    /// Parse a configured level name. `failure` is not one: a failure is
    /// always shown, so hiding everything below it is not a level.
    #[must_use]
    pub fn parse_level(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warning" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// What a diagnostic means for the work of the phase that produced it: the
/// producer's statement of what it did next.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Disposition {
    /// The work stands.
    Warning,
    /// The unit of work the diagnostic is about is lost, and the producer
    /// went on.
    Recoverable,
    /// The producer stopped.
    Fatal,
}

impl Disposition {
    /// The disposition a lowercase word names.
    #[must_use]
    pub fn from_label(label: &str) -> Option<Self> {
        [Self::Warning, Self::Recoverable, Self::Fatal]
            .into_iter()
            .find(|disposition| disposition.label() == label)
    }

    /// The lowercase word a diagnostic is written with.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Warning => "warning",
            Self::Recoverable => "recoverable",
            Self::Fatal => "fatal",
        }
    }
}

/// A secondary place a diagnostic points at, with a note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// What the span is in, when it is not the diagnostic's own origin.
    pub origin: Option<String>,
    /// The place.
    pub span: Range<usize>,
    /// What is there.
    pub text: String,
}

/// The element a step was working on when its function failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The step's name.
    pub step: String,
    /// The element's one-based position in the step's input.
    pub position: usize,
    /// How many elements the input holds, when it is known.
    pub of: Option<usize>,
    /// The element, as data.
    pub value: serde_json::Value,
}

/// Whether a frame of a trace is a pipeline step or a function call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameKind {
    /// A step of a pipeline.
    Step,
    /// A call of a function.
    Call,
}

/// One frame of the path evaluation took to a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// A step or a call.
    pub kind: FrameKind,
    /// The step's or the function's name.
    pub name: String,
    /// What the span is in.
    pub origin: Option<String>,
    /// Where the frame is written.
    pub span: Range<usize>,
    /// A step's one-based position in its pipeline.
    pub position: Option<usize>,
    /// The element the frame was applying a function to.
    pub element: Option<serde_json::Value>,
}

/// Where a value a diagnostic is about came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    /// The file.
    pub path: String,
    /// Its one-based line, when known.
    pub line: Option<usize>,
    /// The key the value was read from, when known.
    pub key: Option<String>,
}

/// A structured event describing a failure, or a fact about one, that is not
/// data. A field that does not apply is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// When it occurred.
    pub phase: Phase,
    /// What the problem is.
    pub kind: Kind,
    /// How serious it is.
    pub severity: Severity,
    /// What the producer did next.
    pub disposition: Disposition,
    /// One sentence for a person; nothing a field carries is formatted
    /// into it.
    pub message: String,
    /// What was being read: a path, `<expression>`, `<repl:N>`, a cell.
    pub origin: Option<String>,
    /// The place in `origin`; an empty range at the end means end of input.
    pub span: Range<usize>,
    /// Every other field, held apart so that a `Result` carrying a
    /// diagnostic stays small.
    pub context: Box<Context>,
}

/// The fields of a diagnostic beyond its axes, its message and its place.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    /// Secondary places, each with a note.
    pub labels: Vec<Label>,
    /// What to write instead: a nearest name.
    pub help: Vec<String>,
    /// A type, or the shape a step needed.
    pub expected: Option<String>,
    /// The value or type that was there, as data.
    pub found: Option<serde_json::Value>,
    /// The element a step was working on.
    pub element: Option<Element>,
    /// The bindings in scope at the span.
    pub bindings: BTreeMap<String, serde_json::Value>,
    /// How evaluation reached it, outermost first.
    pub trace: Vec<Frame>,
    /// Where the data came from.
    pub source: Option<Provenance>,
    /// For a contract read, what `validate` would have given.
    pub violations: Vec<serde_json::Value>,
}

impl Diagnostic {
    /// A diagnostic with its four axes and nothing else.
    #[must_use]
    pub fn new(
        phase: Phase,
        kind: Kind,
        severity: Severity,
        disposition: Disposition,
        span: Range<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            phase,
            kind,
            severity,
            disposition,
            message: message.into(),
            origin: None,
            span,
            context: Box::default(),
        }
    }

    /// The source range the diagnostic points at.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// The explanation, without the axes or the place.
    #[must_use]
    pub fn message(&self) -> String {
        self.message.clone()
    }

    /// The same diagnostic with another kind.
    #[must_use]
    pub fn with_kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    /// The same diagnostic with another severity.
    #[must_use]
    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// The same diagnostic with another disposition.
    #[must_use]
    pub fn with_disposition(mut self, disposition: Disposition) -> Self {
        self.disposition = disposition;
        self
    }

    /// The same diagnostic, produced by `phase`.
    #[must_use]
    pub fn in_phase(mut self, phase: Phase) -> Self {
        self.phase = phase;
        self
    }

    /// The same diagnostic with a line of help.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.context.help.push(help.into());
        self
    }

    /// The same diagnostic with a secondary place.
    #[must_use]
    pub fn with_label(mut self, span: Range<usize>, text: impl Into<String>) -> Self {
        self.context.labels.push(Label {
            origin: None,
            span,
            text: text.into(),
        });
        self
    }

    /// The same diagnostic with the type a step needed.
    #[must_use]
    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.context.expected = Some(expected.into());
        self
    }

    /// The same diagnostic about the element a step was working on, unless
    /// an inner step already named its own.
    #[must_use]
    pub fn at_element(
        mut self,
        step: &str,
        position: usize,
        of: Option<usize>,
        value: serde_json::Value,
    ) -> Self {
        if self.context.element.is_none() {
            self.context.element = Some(Element {
                step: step.to_owned(),
                position,
                of,
                value,
            });
        }
        self
    }

    /// The same diagnostic reached through a call of `name` at `span`: the
    /// frame goes first, since the trace is outermost first and a failure
    /// leaves the innermost call first.
    #[must_use]
    pub fn called_from(mut self, name: &str, span: Range<usize>) -> Self {
        self.context.trace.insert(
            0,
            Frame {
                kind: FrameKind::Call,
                name: name.to_owned(),
                origin: None,
                span,
                position: None,
                element: None,
            },
        );
        self
    }

    /// The same diagnostic with the bindings in scope where it failed,
    /// unless an inner scope already gave its own.
    #[must_use]
    pub fn bound(
        mut self,
        bindings: impl IntoIterator<Item = (String, serde_json::Value)>,
    ) -> Self {
        if self.context.bindings.is_empty() {
            self.context.bindings = bindings.into_iter().collect();
        }
        self
    }

    /// The same diagnostic about a value read from the document at `path`,
    /// unless an inner scope already named where its data came from.
    #[must_use]
    pub fn sourced(mut self, path: Option<String>) -> Self {
        if self.context.source.is_none()
            && let Some(path) = path
        {
            self.context.source = Some(Provenance {
                path,
                line: None,
                key: None,
            });
        }
        self
    }

    /// Whether a step may go on past the element this failure is about: a
    /// failure of execution that leaves the rest of the input whole. An
    /// overflow or an I/O failure stops under every mode.
    #[must_use]
    pub fn steppable(&self) -> bool {
        self.phase == Phase::Execute
            && !matches!(self.kind, Kind::NumericOverflow | Kind::IoFailure)
    }

    /// Whether it makes the result untrustworthy.
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.severity >= Severity::Error
    }

    /// A syntax failure: nothing after it has a structure.
    pub(crate) fn syntax(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(
            Phase::Parse,
            Kind::UnexpectedToken,
            Severity::Failure,
            Disposition::Fatal,
            span,
            message,
        )
    }

    /// A type the checker refuses.
    pub(crate) fn typing(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(
            Phase::Check,
            Kind::TypeMismatch,
            Severity::Error,
            Disposition::Fatal,
            span,
            message,
        )
    }

    /// A name the checker cannot resolve.
    pub(crate) fn name(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(
            Phase::Check,
            Kind::UnknownName,
            Severity::Error,
            Disposition::Fatal,
            span,
            message,
        )
    }

    /// A failure of a checked program while it executes.
    pub(crate) fn runtime(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::new(
            Phase::Execute,
            Kind::InvalidArgument,
            Severity::Failure,
            Disposition::Fatal,
            span,
            message,
        )
    }

    /// A number that left the range of its type.
    pub(crate) fn overflow(span: Range<usize>) -> Self {
        Self::runtime(span, "numeric overflow").with_kind(Kind::NumericOverflow)
    }

    /// Reading or writing outside the program failed.
    pub(crate) fn io_failure(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::runtime(span, message).with_kind(Kind::IoFailure)
    }

    /// A `Data` value whose shape is not the one an operation reads: a
    /// wrong value read, an `error` of the execute phase.
    pub(crate) fn shape_mismatch(span: Range<usize>, message: impl Into<String>) -> Self {
        Self::runtime(span, message)
            .with_kind(Kind::ShapeMismatch)
            .with_severity(Severity::Error)
    }

    /// A division or a remainder by zero.
    pub(crate) fn division_by_zero(span: Range<usize>) -> Self {
        Self::runtime(span, "division by zero").with_kind(Kind::DivisionByZero)
    }

    /// Something worth saying that did not stop the producer.
    pub(crate) fn warning(
        phase: Phase,
        kind: Kind,
        span: Range<usize>,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            phase,
            kind,
            Severity::Warning,
            Disposition::Warning,
            span,
            message,
        )
    }

    /// The JSON form: every field that applies, by its name.
    #[must_use]
    pub fn to_json(&self) -> serde_json::Value {
        let span =
            |span: &Range<usize>| serde_json::json!({ "start": span.start, "end": span.end });
        let mut entry = serde_json::json!({
            "phase": self.phase.label(),
            "kind": self.kind.label(),
            "severity": self.severity.label(),
            "disposition": self.disposition.label(),
            "message": self.message,
            "span": span(&self.span),
        });
        if let Some(origin) = &self.origin {
            entry["origin"] = origin.clone().into();
        }
        if !self.context.labels.is_empty() {
            entry["labels"] = self
                .context
                .labels
                .iter()
                .map(|label| {
                    let mut written =
                        serde_json::json!({ "span": span(&label.span), "text": label.text });
                    if let Some(origin) = &label.origin {
                        written["origin"] = origin.clone().into();
                    }
                    written
                })
                .collect();
        }
        if !self.context.help.is_empty() {
            entry["help"] = self.context.help.clone().into();
        }
        if let Some(expected) = &self.context.expected {
            entry["expected"] = expected.clone().into();
        }
        if let Some(found) = &self.context.found {
            entry["found"] = found.clone();
        }
        if let Some(element) = &self.context.element {
            let mut written = serde_json::json!({
                "step": element.step,
                "position": element.position,
                "value": element.value,
            });
            if let Some(of) = element.of {
                written["of"] = of.into();
            }
            entry["element"] = written;
        }
        if !self.context.bindings.is_empty() {
            entry["bindings"] = self.context.bindings.clone().into_iter().collect();
        }
        if !self.context.trace.is_empty() {
            entry["trace"] = self
                .context
                .trace
                .iter()
                .map(|frame| {
                    let mut written = serde_json::json!({
                        "kind": match frame.kind { FrameKind::Step => "step", FrameKind::Call => "call" },
                        "name": frame.name,
                        "span": span(&frame.span),
                    });
                    if let Some(origin) = &frame.origin {
                        written["origin"] = origin.clone().into();
                    }
                    if let Some(position) = frame.position {
                        written["position"] = position.into();
                    }
                    if let Some(element) = &frame.element {
                        written["element"] = element.clone();
                    }
                    written
                })
                .collect();
        }
        if let Some(source) = &self.context.source {
            let mut written = serde_json::json!({ "path": source.path });
            if let Some(line) = source.line {
                written["line"] = line.into();
            }
            if let Some(key) = &source.key {
                written["key"] = key.clone().into();
            }
            entry["source"] = written;
        }
        if !self.context.violations.is_empty() {
            entry["violations"] = self.context.violations.clone().into();
        }
        entry
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} {}[{}] at bytes {:?}: {}",
            self.phase.label(),
            self.severity.label(),
            self.kind.label(),
            self.span,
            self.message
        )?;
        for help in &self.context.help {
            write!(f, "; help: {help}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}

impl Diagnostic {
    /// A diagnostic read back from its JSON form: the axes, the message,
    /// the place, and the fields a text form shows. What it cannot place,
    /// a label's or a frame's span, is kept as written.
    ///
    /// # Errors
    /// Names an axis that is missing or not one of its words.
    pub fn from_json(json: &serde_json::Value) -> Result<Self, String> {
        // An optional field a record left absent is written `null`.
        let field = |key: &str| json.get(key).filter(|value| !value.is_null());
        let text = |key: &str| field(key).and_then(serde_json::Value::as_str);
        let axis = |key: &str| text(key).ok_or_else(|| format!("a diagnostic has no `{key}`"));
        let unknown = |key: &str, word: &str| format!("`{word}` is not a {key}");
        let phase = axis("phase")?;
        let kind = axis("kind")?;
        let severity = axis("severity")?;
        let disposition = axis("disposition")?;
        let at = |span: Option<&serde_json::Value>| {
            let bound = |key: &str| {
                span.and_then(|span| span.get(key))
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|offset| usize::try_from(offset).ok())
                    .unwrap_or(0)
            };
            bound("start")..bound("end")
        };
        let mut diagnostic = Self::new(
            Phase::from_label(phase).ok_or_else(|| unknown("phase", phase))?,
            Kind::from_label(kind).ok_or_else(|| unknown("kind", kind))?,
            Severity::from_label(severity).ok_or_else(|| unknown("severity", severity))?,
            Disposition::from_label(disposition)
                .ok_or_else(|| unknown("disposition", disposition))?,
            at(field("span")),
            text("message").unwrap_or_default(),
        );
        diagnostic.origin = text("origin").map(str::to_owned);
        let context = &mut diagnostic.context;
        let list = |key: &str| {
            field(key)
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_default()
        };
        context.help = list("help")
            .iter()
            .filter_map(|help| help.as_str().map(str::to_owned))
            .collect();
        context.labels = list("labels")
            .iter()
            .map(|label| Label {
                origin: text_in(label, "origin").map(str::to_owned),
                span: at(label.get("span")),
                text: text_in(label, "text").unwrap_or_default().to_owned(),
            })
            .collect();
        context.expected = text("expected").map(str::to_owned);
        context.found = field("found").cloned();
        context.element = field("element").map(|element| Element {
            step: text_in(element, "step").unwrap_or_default().to_owned(),
            position: count_in(element, "position").unwrap_or(0),
            of: count_in(element, "of"),
            value: element.get("value").cloned().unwrap_or_default(),
        });
        if let Some(serde_json::Value::Object(bound)) = field("bindings") {
            context.bindings = bound.clone().into_iter().collect();
        }
        context.trace = list("trace")
            .iter()
            .map(|frame| Frame {
                kind: if text_in(frame, "kind") == Some("step") {
                    FrameKind::Step
                } else {
                    FrameKind::Call
                },
                name: text_in(frame, "name").unwrap_or_default().to_owned(),
                origin: text_in(frame, "origin").map(str::to_owned),
                span: at(frame.get("span")),
                position: count_in(frame, "position"),
                element: frame.get("element").cloned(),
            })
            .collect();
        context.source = field("source").map(|source| Provenance {
            path: text_in(source, "path").unwrap_or_default().to_owned(),
            line: count_in(source, "line"),
            key: text_in(source, "key").map(str::to_owned),
        });
        context.violations = list("violations");
        Ok(diagnostic)
    }
}

/// The text at `key` of a JSON object, when it holds text.
fn text_in<'j>(json: &'j serde_json::Value, key: &str) -> Option<&'j str> {
    json.get(key).and_then(serde_json::Value::as_str)
}

/// The count at `key` of a JSON object, when it holds one.
fn count_in(json: &serde_json::Value, key: &str) -> Option<usize> {
    json.get(key)
        .and_then(serde_json::Value::as_u64)
        .and_then(|count| usize::try_from(count).ok())
}

/// The most serious severity among `diagnostics`.
#[must_use]
pub fn worst(diagnostics: &[Diagnostic]) -> Option<Severity> {
    diagnostics
        .iter()
        .map(|diagnostic| diagnostic.severity)
        .max()
}

/// Whether any of `diagnostics` is at or above `level`.
#[must_use]
pub fn reaches(diagnostics: &[Diagnostic], level: Severity) -> bool {
    worst(diagnostics).is_some_and(|worst| worst >= level)
}

/// How many of `diagnostics` have `severity`.
#[must_use]
pub fn count(diagnostics: &[Diagnostic], severity: Severity) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == severity)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_axes_are_set_by_the_producer_and_not_derived() {
        let contract = Diagnostic::runtime(0..1, "no")
            .with_kind(Kind::ContractViolation)
            .with_severity(Severity::Error);
        assert_eq!(contract.phase, Phase::Execute);
        assert_eq!(contract.severity, Severity::Error);
        assert_eq!(
            Diagnostic::division_by_zero(0..1).severity,
            Severity::Failure
        );
        assert_eq!(Diagnostic::typing(0..1, "no").phase, Phase::Check);
    }

    #[test]
    fn the_json_form_reads_back_as_the_same_diagnostic() {
        let written = Diagnostic::division_by_zero(4..9)
            .with_help("divide by a count that is not zero")
            .at_element("map", 2, Some(3), serde_json::json!(0))
            .bound([("n".to_owned(), serde_json::json!(0))])
            .called_from("share", 0..3);
        assert_eq!(Diagnostic::from_json(&written.to_json()), Ok(written));
        assert!(Diagnostic::from_json(&serde_json::json!({"phase": "lunch"})).is_err());
    }

    #[test]
    fn the_json_form_leaves_out_what_does_not_apply() {
        let json = Diagnostic::syntax(3..3, "no").to_json();
        assert_eq!(json["phase"], "parse");
        assert_eq!(json["kind"], "unexpected-token");
        assert_eq!(json["disposition"], "fatal");
        assert!(json.get("help").is_none());
    }
}
