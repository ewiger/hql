// status: invalid
// feature: Data is parsed only in a format whose decoder reads Data
// implementation: implemented
// environment: pure
// phase: check
// kind: type-mismatch
// severity: error

// `Lines` decodes String records, not trees: read the text as lines with
// `split`, or parse it in Json, JsonLines, Yaml or Csv.
"owl\nbat" | Data(Lines)
