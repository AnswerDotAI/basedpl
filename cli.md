

# Command line

[Home](index.ipynb) · [Install](getting-started.ipynb#install) ·
[REPL](repl.qmd)

## Expressions and files

`-e` evaluates one expression. Quote it to protect spaces and glyphs
from the shell.

``` bash
bapl -e '+/⍳10'       # 45
```

Pass a UTF-8 APL file to run it as one source. Names persist throughout
the file.

``` bash
bapl lesson.apl
```

Expressions display results; assignments are silent. Use `⎕←` for
explicit output. Diagnostics go to stderr and include source locations.

## Pipes

With no arguments, piped input behaves like the REPL without prompts.
Names persist between expressions.

``` bash
printf 'v←⍳10\n+/v\n' | bapl
```

`-` reads all stdin as one source before evaluation.

``` bash
printf 'v←⍳10\n+/v\n' | bapl -
```

Use actual glyphs in files, pipes and `-e`; backtick names are a [REPL
input method](repl.qmd#typing-glyphs).

## Other modes

<table>
<thead>
<tr>
<th>Command</th>
<th>Mode</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>bapl</code></td>
<td>Interactive REPL when connected to a terminal</td>
</tr>
<tr>
<td><code>bapl --help</code></td>
<td>Usage</td>
</tr>
<tr>
<td><code>bapl --version</code></td>
<td>Version</td>
</tr>
<tr>
<td><code>bapl --json</code></td>
<td>Persistent JSON-string requests</td>
</tr>
<tr>
<td><code>bapl --worker</code></td>
<td>Interruptible structured requests</td>
</tr>
</tbody>
</table>

See [process interfaces](processes.qmd) for machine-readable output.
Exit status: 0 success, 1 evaluation/I/O error, 2 invalid command
arguments. Output produced before an error is retained.
