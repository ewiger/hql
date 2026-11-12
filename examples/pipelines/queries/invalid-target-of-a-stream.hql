// status: invalid
// feature: a stream has no last value, so `target` of one is a type error; `sink` writes it
// implementation: implemented
// environment: knowledge-v1
// vault: pipelines/shop
// phase: check
// kind: type-mismatch
// severity: error

type Event {
    id       : Int
    kind     : String
    customer : String
    amount   : Decimal
    at       : String
}

events : Stream<Event> = tap("order_events")
events | target("orders")
