// status: valid-now
// feature: a module found under the module root the workspace's hql.toml names, imported by its dotted name
// implementation: implemented
// environment: knowledge-v1
// vault: contexts/workspace
// expected-type: List<String>
// expected: ["Dune", "Middlemarch"]

import club.shelf

shelf.unread(["Dune", "Emma", "Middlemarch"], ["Emma"])
