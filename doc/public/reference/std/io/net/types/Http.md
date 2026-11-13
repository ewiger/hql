<!-- hql reference: std/io/net.Http -->
# `Http`

In [`std/io/net`](../index.md).

```hql
type Http : {Replayable, Writable} { url : String method : String // "GET" unless written headers : Data // a map of header names to values }
```

An HTTP resource at `url`. Read, it is a `GET` whose response body is
its bytes; the host reads it once per evaluation, so it is
`Replayable`. Written, it is a request by `method` whose body is the
document, and the response is not read beyond its status. Both send
`headers`, a map of header names to values. Authentication belongs in a
configured endpoint, not in the program. A connection not made within
10 seconds fails, and so does a response that sends nothing for 30
seconds, or for the `timeout_ms` given; each failure names the URL.
<!-- hql reference: end -->
