// status: valid-now
// implementation: implemented
// environment: pure
// io: sandbox
// feature: Read a file as lines, transform them, and write another file whole
// expected-type: Int
// expected: 3

// `Lines` frames text one record per line: the terminator belongs to the
// representation, so each record is the line without it, and writing puts
// the terminators back. `target` replaces the whole file; a relative path
// resolves against the directory of this program. The file written is
// `data/shout.txt`.
shouted = source(file("../data/haiku.txt"), Lines) | map(l => l.upper())

shouted | target(file("../data/shout.txt"), Lines)

shouted.size()
