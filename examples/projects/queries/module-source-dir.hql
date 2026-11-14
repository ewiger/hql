// status: valid-now
// feature: `fs.source_dir()` in a module is the directory holding the module, not the directory of the program that imports it
// implementation: implemented
// environment: knowledge-v1
// vault: projects/library
// expected-type: String
// expected: shelf

import shelf

shelf.home()
