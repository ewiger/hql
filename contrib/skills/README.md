# Agent skills

Skills that teach an agent host, such as Claude Code, to drive HQL. They are
text the host loads, not code the crate builds, and `cargo test` never reads
them.

| Skill | What it does |
| --- | --- |
| [`hmd-run`](hmd-run/SKILL.md) | Executes a card's HQL cells and answers its AI cells' requests itself, under `--ai host` |

## Installing

Copy a skill's directory into a skills directory the host reads: the vault's
`.claude/skills/` to offer it in that vault only, or `~/.claude/skills/` to
offer it everywhere.

```bash
cp -r contrib/skills/hmd-run <vault>/.claude/skills/
```

The skill calls `hql` through the host's shell tool, and its `allowed-tools`
line names the three commands it runs and nothing broader.

## Tests

A skill is instructions, so what is tested is what it instructs. Every command
`hmd-run` names, in the order it names them, is exercised by
`tests/ai.rs`, in the test of the host loop:
render under `--ai host`, list the pending requests, answer each, and render
again until nothing is pending, ending in a byte-identical replay.

See [the sessions companion of HQL-0022](../../doc/proposals/HQL-0022/sessions.md).
