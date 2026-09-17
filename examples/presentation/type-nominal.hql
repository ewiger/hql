// status: design-question
// feature: presentation typing alternative
// implementation: pending
// environment: knowledge-v1
// expected-type: TablePresentation
// expected: table presentation of all fixture cards
// note: Nominal alternative. Named subtypes let a host or type annotation demand one specific view, at the cost of a type per presenter. EmptyPresentation, TextPresentation, JsonPresentation, GraphPresentation, MarkdownPresentation and TreePresentation are the sibling candidates.
// alternatives: presentation/type-functional.hql, design/custom-types-a.hql

Presentation
TablePresentation <: Presentation

t : TablePresentation = cards | table
t
