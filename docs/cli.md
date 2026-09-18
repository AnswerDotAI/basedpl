# Command line

[Home](index.md) · [Install](getting-started.md#install) · [REPL](repl.md)

## Expressions and files

`-e` evaluates one expression. Quote it to protect spaces and glyphs from the shell.

```bash
miniapl -e '+/⍳10'       # 55
```

Pass a UTF-8 APL file to run it as one source. Names persist throughout the file.

```bash
miniapl lesson.apl
```

Expressions display results; assignments are silent. Use `⎕←` for explicit output. Diagnostics go to stderr and include source locations.

## Pipes

With no arguments, piped input behaves like the REPL without prompts. Names persist between expressions.

```bash
printf 'v←⍳10\n+/v\n' | miniapl
```

`-` reads all stdin as one source before evaluation.

```bash
printf 'v←⍳10\n+/v\n' | miniapl -
```

Use actual glyphs in files, pipes and `-e`; backtick names are a [REPL input method](repl.md#typing-glyphs).

## Other modes

| Command | Mode |
|---|---|
| `miniapl` | Interactive REPL when connected to a terminal |
| `miniapl --help` | Usage |
| `miniapl --version` | Version |
| `miniapl --json` | Persistent JSON-string requests |
| `miniapl --worker` | Interruptible structured requests |

See [process interfaces](processes.md) for machine-readable output. Exit status: 0 success, 1 evaluation/I/O error, 2 invalid command arguments. Output produced before an error is retained.
