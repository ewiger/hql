// status: valid-now
// feature: a configured JSON endpoint is the elements of its array, in order
// implementation: implemented
// environment: knowledge-v1
// vault: endpoints/station
// expected-type: Seq<Option<Data>>
// expected-json: ["owl","kite"]

// `s.species` is an `Option<Data>`: a `Data` tree holds whichever keys
// its document wrote.
source("sightings")
| filter(s => match s.hour { some(Int h) => h >= 12, _ => false })
| map(s => s.species)
