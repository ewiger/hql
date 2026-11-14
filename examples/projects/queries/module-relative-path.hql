// status: valid-now
// feature: a relative path a module writes resolves against the module's own directory: `titles.txt` is the file beside `shelf`, not one beside this program
// implementation: implemented
// environment: knowledge-v1
// vault: projects/library
// expected-type: List<String>
// expected: ["Dune", "Emma", "Middlemarch"]

import shelf

shelf.titles()
