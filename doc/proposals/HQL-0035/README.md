# HQL-0035: The schema boundary of a configured source or target

**Status**: drafted
**Created**: 2026-10-23
**Source**: [HQL-0012](../HQL-0012/README.md), whose `[sources.<id>]`
configuration names the systems a program reads;
[HQL-0031](../HQL-0031/README.md), whose formats leave how a decimal is
carried to the boundary's schema

## Abstract

A configured source or target in `hql.toml` has a schema boundary: the
place where HQL's declared types meet an external system's own schema. The
boundary states how a declared record type maps to a SQL table's columns, a
`Decimal` to a JSON string or a `NUMERIC(p, s)`, an optional field to a
nullable column. It is what lets a `Table<R>` be written to SQLite or
Postgres, and what overrides a format's default carriage of a type, so a
program declares only HQL types and the configuration states the external
schema they meet.

## Motivation

A program that writes `Table<User>` to a database needs the table's columns,
their SQL types and its keys. A program that writes `Decimal` amounts as
JSON for a consumer that reads JSON numbers as binary floats needs them
written as strings. Neither belongs in the program: both are facts about
the external system, and the configuration already holds that system's
connection.

## Specification

*Placeholder; specified separately.* To be specified: the configuration
keys of a schema boundary, the mapping of each HQL type to each target
system's types, how `Table<R>` is created, appended to and replaced in a
SQL database, and how a boundary overrides a format's carriage of a type.

## Changelog

- 2026-10-23: drafted as a stub
