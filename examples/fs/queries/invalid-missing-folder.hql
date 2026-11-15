// status: invalid
// feature: listing a folder that does not exist is an evaluation error naming its path
// implementation: implemented
// environment: pure
// phase: execute
// kind: io-failure
// severity: failure
// expected-type: Seq<Entry>

from io.fs import *

folder("no-such-folder") | entries
