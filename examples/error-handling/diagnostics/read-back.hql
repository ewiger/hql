// status: valid-now
// feature: A diagnostic delivered to a file is data to a later run: it reads back as the standard Diagnostic record
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Seq<String>
// expected-json: ["check warning[naming-convention]", "execute failure[division-by-zero]"]

// `run.jsonl` is what `hql --diagnostics-format json run
// warning-then-failure.hql` wrote to standard error. This run did not
// produce those diagnostics; it reads them as any data, each line as a
// `Diagnostic`.
source(file("run.jsonl"), JsonLines)
    | Diagnostic
    | map(d => fmt("{} {}[{}]", d.phase, d.severity, d.kind))
