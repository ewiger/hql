// status: valid-now
// feature: a declared Date, Instant or Duration field reads its ISO 8601 text
// implementation: implemented
// environment: pure
// io: sandbox
// expected-type: Seq<Data>
// expected: [{at: 2026-10-24T07:30:00Z, name: "launch", took: PT1H30M}, {at: 2026-10-30T15:00:00Z, name: "retro", took: PT1H}]

import time

type Event {
    name : String
    on   : Date
    at   : Instant
    took : Duration
}

events : Seq<Event> = source(file("../events.jsonl"))
events
| filter(e => e.on.truncate(Month) == Date("2026-10-01"))
| sort(by = e => e.at)
| map(e => {name: e.name, at: e.at, took: e.took})
