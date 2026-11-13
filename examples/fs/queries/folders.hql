// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: `entries` lists one folder by name, and `kind` tells a folder from a file
// expected-type: Seq<String>
// expected: ["assets", "data"]

import io.fs

folder("../tree") | entries | filter(e => e.kind == "folder") | map(e => e.name)
