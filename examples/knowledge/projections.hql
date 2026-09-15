// status: design-question
// feature: knowledge projections
// implementation: pending
// environment: knowledge-v1
// expected-type: Graph
// expected: only g is the program result; t/l/d are retained bindings
// note: Candidate t:HmdTable, l:HmdList, d:Diagram. Projection/conversion names do not imply printing; graph remains richer-data-to-structure projection only.

g = knowledge | graph
t = knowledge | table
l = knowledge | list
d = knowledge | diagram

g
