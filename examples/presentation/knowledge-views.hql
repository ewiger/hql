// status: design-question
// feature: multiple views of one value
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: the graph presentation is the program result; t and j are retained presentation bindings
// note: Three views of the same Knowledge value, not three different values. Adding timeline, matrix or diagram presenters must not change the underlying semantic types. Compare the projection reading, where the same stages produce Graph, HmdTable and Diagram values.
// alternatives: knowledge/projections.hql, presentation/graph-view.hql

k = knowledge

g = k | graph
t = k | table
j = k | json

g
