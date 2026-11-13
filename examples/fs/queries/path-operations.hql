// status: valid-now
// implementation: implemented
// environment: pure
// feature: `join`, `parent`, `name`, `stem` and `extension` read a path's text and never the file system
// expected-type: List<String>
// expected: ["data/birds.csv", "birds.csv", "birds", "csv", "data", "none"]

import io

p = Path("data").join("birds.csv")
[p.text, p.name(), p.stem(), p.extension() | unwrap_or("?"), p.parent() | map(q => q.text) | unwrap_or("?"), str(Path("data").parent())]
