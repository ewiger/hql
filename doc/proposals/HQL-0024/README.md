# HQL-0024: A complete HQL type system, and the language features it carries

- **Status**: drafted
- **Created**: 2026-10-21
- **Source**: [the HQL type system](../../wiki/hql/type-system.hmd)

## Abstract

HQL's type system is completed: every construct the
[type system](../../wiki/hql/type-system.hmd) card leaves undecided or
designed-but-unbuilt is specified and checked, so that subtyping, generics and
refinement form one closed whole. The completed system is the foundation the
language's advanced features are specified against, and each such feature
states the typing rule it relies on rather than introducing one of its own.

## Motivation

- The type system card ends with decisions it does not make, and several of
  its rules are designed but not in the checker. A feature built on an
  unsettled rule settles that rule implicitly.
- Advanced features — constructor-generic operations, typed results of
  failure, records, refinement at a call site — each need the same handful of
  typing rules. Settling those rules once is cheaper than settling them per
  feature.

## Goals

The completed type system covers:

- whether narrowing is nominal or structural
- the call-site spelling of refinement and validation
- the diagnosis of conflicting parents in a supertype set
- the shape of a parameterized capability registration such as `Renderable<T>`
- how much refinement survives an extension, a cell or a host boundary
- whether a subtype may change a field's optionality
- constructor parameters, `F<_>`, in the checker, so that an operation returns
  the container it was given
- record values, and the conversions that need them
- `Result`, and `match` arms over it
- function types beyond four parameters

## Non-goals

- No traits, and no field visibility: the card's position on both stands.
- No host-language precedent as an argument; see [the stack](../../stack.md).

## Specification

<!-- Placeholder: one numbered subsection per goal, each stating its rule with
MUST / SHOULD / MAY. -->

## Backwards Compatibility

<!-- Placeholder. -->

## Reference Implementation

<!-- Placeholder. -->

## Test Plan

<!-- Placeholder. -->

## Changelog

- 2026-10-21: drafted as a stub
