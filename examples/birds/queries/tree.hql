// status: valid-now
// feature: Data is a union: a leaf, a sequence of trees, or a string-keyed map of trees
// implementation: implemented
// environment: pure
// expected-type: List<Data>
// expected-json: [3,"barn-owl",true,["dusk","dawn"],{"entries":[{"key":"barn-owl","value":2},{"key":"common-raven","value":1}],"type":"Map<String, Int>"},{"observer":"field-notes","species":"barn-owl"}]

count : Scalar = 3
active : Data = ["dusk", "dawn"]
tally : Data = Map(["barn-owl", "common-raven"], [2, 1])
note = { species: "barn-owl", observer: "field-notes" }

[count, "barn-owl", true, active, tally, note]
