// status: valid-now
// feature: a function the project's .hql/config.toml imports with from is in scope for every program it runs, with no import line
// implementation: implemented
// environment: knowledge-v1
// vault: contexts/project
// expected-type: Option<Float>
// expected: 3.0

[1, 2, 3, 6] | mean
