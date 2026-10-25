<!-- hql reference: std/io/net.WebSocket -->
# `WebSocket`

In [`std/io/net`](../index.md).

```hql
type WebSocket : {Readable, Writable} { url : String }
```

A WebSocket connection to `url`: each message is a document, read as it
arrives and written as it is sent.
<!-- hql reference: end -->
