

# Process interfaces

[Home](index.ipynb) · [Command line](cli.qmd) · [Python](python.ipynb)

## JSON lines

Run `bapl --json`. Send one JSON-encoded APL string per line; each
receives one flushed JSON response. Names persist.

``` json
"v←⍳10"
"+/v"
```

The second response is:

``` json
{"value":45.0,"output":[{"kind":"display","data":{"text/plain":"45"}}],"error":null}
```

`output` contains ordered events: `kind` is `display` or `explicit`, and
`data` is a MIME bundle with `text/plain`. SVG display adds
`image/svg+xml`. Returned values carry contents and axis labels. Keyed
entries that hold functions, such as `_mime_` renderers, are omitted.
Other functions in a result give a `DOMAIN` error.

Use `json.dumps(code)` or `JSON.stringify(code)`, followed by a newline.
Quotes are required; source newlines are escaped inside the string.
Stdout contains only responses. The session remains usable after errors.
EOF ends it.

Atoms are encoded directly. Arrays carry shape, row-major data and
prototype, including rank-zero arrays. Nested elements use the same
encoding. A [keyed array](keyed.qmd) adds `axis_keys`: one string list
or `null` per axis. All-unkeyed arrays omit it.

Named dimensions add `axis_names`: one string or `null` per axis,
e.g. `["city", "month"]`. Arrays with no named axes omit it. Both fields
survive worker input and output.

<table>
<thead>
<tr>
<th>Element</th>
<th>JSON</th>
</tr>
</thead>
<tbody>
<tr>
<td>Exact integer</td>
<td>Integer, including arbitrary precision</td>
</tr>
<tr>
<td>Approximate real</td>
<td>Number with decimal point or exponent</td>
</tr>
<tr>
<td>Character</td>
<td>String</td>
</tr>
<tr>
<td>Rational</td>
<td><code>{"rational":["1","3"]}</code></td>
</tr>
<tr>
<td>Complex</td>
<td><code>{"complex":[1.0,2.0]}</code></td>
</tr>
<tr>
<td>Infinity</td>
<td><code>{"infinity":1}</code> or <code>{"infinity":-1}</code></td>
</tr>
</tbody>
</table>

Ordinary JavaScript `JSON.parse` can round large integers; use an
integer-preserving parser. Error spans are UTF-8 byte ranges into the
supplied source; call sites accompany the original location.

## Interruptible worker

Python’s `Worker` manages a persistent subprocess with deadlines and a
hard-kill fallback.

``` python
from basedpl.worker import Worker

with Worker() as w:
    w.eval('v←⍳10')
    r = w.eval('+/v', timeout=2)
    assert r['value'] == 45
```

`w.interrupt()` cancels from another thread. Ctrl-C requests
cancellation too. Cooperative cancellation preserves the session and
completed assignments. After the grace period (default one second), an
unresponsive process is killed and its session is lost. Requests are
never replayed. `w.diagnostics` holds recent stderr.

## Worker protocol

`bapl --worker` accepts JSON objects with request IDs. Keep one
evaluation outstanding.

``` json
{"id":1,"code":"+/⍳10","timeout_ms":2000,"echo":false}
```

Replies have `{"id":1,"result":...}`. `{"interrupt":1}` cancels that
request without a separate reply. `echo:false` suppresses implicit
display, not explicit output.

`bindings` maps names to encoded values. Use `call` with one or two
encoded `args` to apply a function without generating APL source.

``` json
{"id":2,"call":"+","args":[2,3],"echo":false}
```

`Worker.request(payload, timeout=...)` supplies the ID for Python
callers. Integer inputs remain exact. Invalid operations/arrays return
`REQUEST ERROR`; malformed JSON or invalid control fields terminate the
worker. Use tagged infinities, never raw NaN/Infinity JSON tokens.
