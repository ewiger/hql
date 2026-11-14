// status: valid-now
// feature: `import hmd` activates HyperMarkdown for the run: `cards` is bound and `@[[..]]` resolves in the vault
// implementation: implemented
// environment: knowledge-v1
// vault: projects/notes
// expected-type: List<Option<String>>
// expected: ["heron", "owl"]

import hmd

[@[[heron]].name, @[[owl]].name]
