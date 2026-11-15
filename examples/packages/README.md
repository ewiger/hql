# Packages

This directory is a project whose `.hql/config.toml` lists the crate of the
`hello` package, [`tests/packages/hello/`](../../tests/packages/hello/),
under `[packages] paths`. `hql` builds the crate the first time a program
runs here and loads its library, so `import hello` resolves without an
install:

```sh
hql --project examples/packages eval 'import hello
hello.greet("Ada")'
```

The chapters of the book's Part V, *Extending HQL*, run their programs in
this project. The package itself is the worked example of chapter 34 and
the fixture of `tests/package.rs`; the design is
[HQL-0057](../../doc/proposals/HQL-0057/index.hmd).
