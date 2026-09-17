// status: proposed
// feature: progressive type refinement
// implementation: pending
// environment: knowledge-v1
// expected-type: Seq<String>
// expected: researcher, engineer
// note: One unchanged tree, three layers of knowledge about it. HMD reserves tags, so that path is Seq of String while header.age beside it stays Data until a schema or a structural test says more. Refinement never converts the value; it strengthens what the checker can prove about a region of it.

header : Data = resolve([[alice]]).header

// the format's own reserved keys are known to the HMD extension
header.tags
