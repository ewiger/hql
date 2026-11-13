// status: valid-now
// feature: DiagnosticText is a format a program names: delivered diagnostics read back and written as the text a run prints, without the source line
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: String
// expected-json: "check warning[naming-convention]: `totalOf` is a function, written in snake_case\n --> examples/error-handling/diagnostics/warning-then-failure.hql\n  = help: write it `total_of`\n\nexecute failure[division-by-zero]: division by zero\n --> examples/error-handling/diagnostics/warning-then-failure.hql\n\n2 diagnostics: 1 failure, 0 error, 1 warning"

// The diagnostics `run.jsonl` holds are data to this run, and `DiagnosticText`
// writes them as a report: each one's heading, origin and fields, then the
// count. The source line is not shown: this run did not read the program
// the diagnostics are about.
source(file("run.jsonl"), JsonLines)
    | Diagnostic
    | format(DiagnosticText)
