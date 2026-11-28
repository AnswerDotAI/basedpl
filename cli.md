

# Command line

## Expressions and files

`-e` evaluates one expression. Quote it to protect spaces and glyphs
from the shell.

``` bash
bpl -e '+/ι10'       # 45
```

Pass a UTF-8 BPL file to run it as one program. Its statements run in
order, names persist throughout the file, and the first error ends the
run.

``` bash
bpl lesson.bpl
```

The result of an expression is displayed. An assignment displays
nothing. Use `⎕←` for explicit output. `bpl` writes each output
immediately, without waiting for the rest of the source. Diagnostics go
to stderr and include source locations.

To run a file as a script, start it with `#!/usr/bin/env bpl`. `bpl`
ignores a first line that starts with `#!`.

## Pipes

When you pipe input to `bpl` with no arguments, `bpl` behaves like the
REPL without prompts. Names persist between lines, and an error in one
line does not stop the lines after it.

``` bash
printf 'v←ι10\n+/v\n' | bpl
```

[`-`](https://answerdotai.github.io/basedpl/glyphs/minus.html) reads all
of stdin first, then runs it as it runs a file.

``` bash
printf 'v←ι10\n+/v\n' | bpl -
```

When the program that reads the output closes the pipe, `bpl` stops the
evaluation and exits with status 0.

A program run with `-e` or from a file can read standard input through
[`⎕`](glyphs/quad.qmd#input) and `•nget "-"`. This program parses piped
CSV:

``` bash
printf 'name,qty\npen,2\ncup,5\n' | bpl -e '+/(•csv •nget "-").qty'   # 7ₓ
```

When you pass
[`-`](https://answerdotai.github.io/basedpl/glyphs/minus.html), or pipe
input without arguments, the program comes from standard input. Reading
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) or
`•nget "-"` then raises an `IO` error. In the interactive REPL, reading
[`⎕`](https://answerdotai.github.io/basedpl/glyphs/quad.html) gives a
line from the terminal. See [`⎕`](glyphs/quad.qmd#input).

Use actual glyphs in files, pipes and `-e`; backtick names are a [REPL
input method](repl.qmd#typing-glyphs).

## Other modes

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Command</th>
<th>Mode</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>bpl</code></td>
<td>Interactive REPL when connected to a terminal</td>
</tr>
<tr>
<td><code>bpl --help</code></td>
<td>Usage</td>
</tr>
<tr>
<td><code>bpl --version</code></td>
<td>Version</td>
</tr>
<tr>
<td><code>bpl --worker</code></td>
<td>JSON-lines requests with deadlines and interruption</td>
</tr>
<tr>
<td><code>bpl --costs PATH</code></td>
<td>Times the cases in a speed file in the format of basedpl’s
<code>tests/speed/*.bpl</code>, or in a directory of them, as the speed
test does. Prints each cost with this machine’s budget, then each case
still over its budget after the retries, marked <code>over</code>. A
case over its budget leaves the exit status at 0</td>
</tr>
</tbody>
</table>

See [process interfaces](processes.qmd) for machine-readable output. The
exit status is 0 on success, 1 after an evaluation or I/O error, and 2
for invalid command arguments. An error doesn’t discard the output
written before it.
