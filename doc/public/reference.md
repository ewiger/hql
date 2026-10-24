# Reference

## Commands at a glance

| Command | What it does |
| --- | --- |
| `hql eval <program>` | check and evaluate an argument |
| `hql check <file>` | print the type without evaluating |
| `hql run <file>` | check and evaluate a file |
| `hql repl` | read, evaluate and print, keeping bindings |
| `hql render <file>` | run the HQL cells in a document and build the Markdown document they yield |
| `hql builtins` | list the steps |
| `hql config` | the reporting mode and shown level in force, and where each came from |
| `hql doctor` | check the semantic-search chain and name the first missing part |
| `hql ai` | list the model requests waiting under `--ai host`, and answer one (next release) |

`hql '<program>'` is `hql eval '<program>'` (next release). `-` reads standard
input wherever a file is taken. Success is exit `0`, a
language or I/O failure `1`, a usage error `2`.

## The standard library

HQL's types are declared in HQL itself, under
[`lib/std/`](https://github.com/ewiger/hql/tree/main/std): `core`, `collections`,
`doc`, `graph`, `knowledge` and `retrieval`, joined in the next release by `io`
(endpoints, sources, targets, channels and logging) and `ai` (agents, contexts,
prompts, tools and runs). The checker reads those
declarations rather than assuming them.
