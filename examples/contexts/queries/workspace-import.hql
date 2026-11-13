// status: valid-now
// feature: an extension the workspace's hql.toml imports is in scope for every program it runs, with no import line
// implementation: implemented
// environment: knowledge-v1
// vault: contexts/workspace
// expected-type: Option<Float>
// expected: 3.0

[1, 2, 3, 6] | mean
