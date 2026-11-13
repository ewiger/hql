<!-- hql reference: std/io/net.WebSocket -->
# `WebSocket`

In [`std/io/net`](../index.md).

```hql
type WebSocket : {Readable, Writable} { url : String }
```

A WebSocket connection to `url`: each message is a document, read as it
arrives and written as it is sent. A connection not made within 10
seconds fails, and so does a handshake the server answers nothing to
for 30 seconds, naming the URL; once open, a read waits for the next
message without limit.
<!-- hql reference: end -->
