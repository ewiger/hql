# Digest: cryptographic digests of text and of trees

The cases follow [HQL-0041 part 3](../../doc/proposals/HQL-0041/parts/03-digest-and-encoding.hmd).
The module is imported with `import crypto.digest` and reached as `digest`.
An algorithm is a type, `Sha256`, and a digest is written as lowercase hex.
`fingerprint` is the SHA-256 of a tree's canonical JSON (RFC 8785), so two
trees equal under HQL equality share one. The expected values are the
published vectors, computed with `shasum` and `md5`.

| Case | What it shows | Result |
| --- | --- | --- |
| [SHA-256](queries/sha256-abc.hql) | `"abc".digest(Sha256)` | FIPS 180-4's vector |
| [every algorithm](queries/every-algorithm.hql) | `Md5`, `Sha1`, `Sha256`, `Sha512` and `Blake3` over `"abc"` | each standard's vector |
| [equal trees](queries/fingerprint-equal-trees.hql) | key order and `1.10` against `1.1` | one fingerprint, twice |
| [not imported](queries/not-imported.hql) | `digest` without `import crypto.digest` | a check error |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/digest/queries/every-algorithm.hql
```
