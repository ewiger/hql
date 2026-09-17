// status: design-question
// feature: disambiguated graph presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: node-and-relation presentation of the projected graph
// note: Keeps graph as the semantic projection and moves presentation into the render namespace, matching render.hmd and render.d2. Costs one stage, but projection and view never share a name.
// alternatives: presentation/graph.hql, outputs/hmd-graph.hql

knowledge
| graph
| render.graph
