// status: invalid
// feature: a Kafka topic does not end, so `source` of one is a type error naming `tap`
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// phase: check
// kind: type-mismatch
// severity: error

// The kind in `hql.toml` states the type, so the checker refuses this
// without connecting to a broker.
source("order_events")
