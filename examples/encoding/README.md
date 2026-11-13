# Encoding: hex, base64 and percent-encoding of text

The cases follow [HQL-0041 part 3](../../doc/proposals/HQL-0041/parts/03-digest-and-encoding.hmd).
The module is imported with `import encoding`. An encoding is a type, `Hex`,
`Base64`, `Base64Url` or `Percent`, and each reads and writes a text's UTF-8
octets. A decode that cannot succeed, for text not valid in the encoding or
for octets that are not UTF-8, gives `none`.

| Case | What it shows | Result |
| --- | --- | --- |
| [encode](queries/encode-each.hql) | each encoding of a text, `+`, `/` and `-` included | four texts |
| [decode](queries/decode-each.hql) | each encoding read back; base64url padded or not; `+` kept | six texts |
| [refused](queries/decode-refused.hql) | an odd hex digit, missing padding, `0xff`, `%ff` | `none` four times |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/encoding/queries/decode-each.hql
```
