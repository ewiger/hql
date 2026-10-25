# `hql` on the command line

With `source` and `sink`, `hql` is a structured-data tool for the shell. A
first argument that names no command is a program, so `hql '<program>'` is
`hql eval '<program>'`:

```text
$ cat examples/io/sightings.jsonl | hql 'source(stdin, Json) | filter(s => s.seen == "morning") | map(s => s.species)'
"wren"
"kingfisher"
```

The result is a `Stream<String>`, so the host writes each value as it
arrives, in HQL, the format a result is written in unless `--to` names
another ([writing results out](../11-output.md)).

## `--to` and `--compact`

`--to json` presents each document of the result pretty-printed, as `jq .`
does, and `--compact` gives one line each, as `jq -c` does:

```text
$ head -1 examples/io/sightings.jsonl | hql 'source(stdin, Json)' --to json
{
  "seen": "dusk",
  "site": "meadow",
  "species": "barn owl"
}
```

```text
$ head -2 examples/io/sightings.jsonl | hql 'source(stdin, Json)' --to json --compact
{"seen":"dusk","site":"meadow","species":"barn owl"}
{"seen":"morning","site":"hedge","species":"wren"}
```

Concatenated JSON values are what `source(…, Json)` reads, so the output of
one `hql` is the input of the next. `sink(e, Json)` writes one document per
line whatever the flags: a program writing a file says what the file is, and
`--compact` is about the terminal.

Converting between formats is a source and a `--to`:

```text
$ hql 'source(file("examples/io/sightings.jsonl")) | map(s => s.site)' --to yaml
- meadow
- hedge
- barn
- river
```

## `--from`

`--from` names the format of standard input once, on the command line, so the
program reads `stdin` without naming one; a format the program names wins:

```text
$ cat examples/io/sightings.jsonl | hql --from json 'source(stdin) | map(s => s.site)'
"meadow"
"hedge"
"barn"
"river"
```

## Input and output stay in the program

The command line names no input and no output of a program's own. Input is
`source` or `tap` in the program, output is `target` or `sink` in the
program, and `--to` only chooses how the host writes the final result. A
program whose result is `Unit` prints nothing:

```text
$ cat examples/io/sightings.jsonl | hql 'source(stdin, Json) | filter(s => s.seen == "morning") | map(s => s.species) | sink(stdout, Text)'
wren
kingfisher
```

`hql run -` reads the program from standard input, so such a program cannot
also read `stdin` as data, and `source(stdin, …)` in it is a type error.
`--log-level` sets every logger's threshold ([logging](logging.md)).
