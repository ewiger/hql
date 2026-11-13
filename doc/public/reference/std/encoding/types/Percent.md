<!-- hql reference: std/encoding.Percent -->
# `Percent`

In [`std/encoding`](../index.md).

```hql
type Percent : Encoding
```

A text-safe encoding of octets. `Percent` leaves RFC 3986's unreserved
characters `A-Z a-z 0-9 - . _ ~` as they are and writes every other
octet as `%XX` in uppercase hex.
<!-- hql reference: end -->
