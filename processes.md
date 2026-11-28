

# Process interfaces

## JSON lines

Run `bpl --worker`. Send one JSON request per line on stdin. Each
request gets one flushed JSON reply, in order. Names persist between
requests.

``` json
{"code":"v←ι10"}
{"id":2,"code":"+/v"}
```

The second reply is:

``` json
{"id":2,"result":{"value":45.0,"output":[{"kind":"display","data":{"text/plain":"45"}}],"error":null}}
```

`id` is optional. A reply includes its request’s `id`, or `null` if the
request has no `id`. Give an `id` to any request you might interrupt:
`{"interrupt":2}` cancels request 2 and gets no reply. Wait for an
evaluation to finish before starting another.

Build each request with a JSON encoder, such as Python’s `json.dumps` or
JavaScript’s `JSON.stringify`, so that newlines in the source are
escaped, and end it with a newline. Stdout contains only replies. Close
stdin to end the session.

Malformed JSON, an invalid field or an invalid encoded array gives an
error of kind `REQUEST` in that request’s `result`, without ending the
session.

### Requests

`code` is BPL source to evaluate. `timeout_ms` is the time allowed for
the evaluation, in milliseconds. `"echo":false` suppresses implicit
display, but not explicit output.

``` json
{"id":1,"code":"+/ι10","timeout_ms":2000,"echo":false}
```

`bindings` assigns encoded values to names in the session. `call` is a
function to apply to one or two encoded `args`, without generating BPL
source.

``` json
{"id":2,"call":"+","args":[2,3],"echo":false}
```

### Results

`output` holds the events in order. Each event’s `kind` is `display` or
`explicit`, and its `data` is a MIME bundle with `text/plain`. If the
displayed value has a renderer, `data` also includes the renderer’s MIME
types, such as `image/svg+xml`. `value` holds the returned value’s
contents and axis labels, without renderers. Keyed entries in `value`
that hold functions are omitted. A result that is a function, or an
array that still holds one, has a `null` `value`, and `output` holds its
display.

Atoms are encoded directly. All arrays, including rank-zero arrays, are
encoded with their shape, row-major data and
[prototype](terms.qmd#term-prototype). Nested items use the same
encoding. When an array has keys on at least one axis, its encoding
includes `axis_keys`, with one list of strings or `null` per axis.

When an array has at least one named axis, its encoding includes
`axis_names`, with one string or `null` per axis, such as
`["city", "month"]`. Encoded arrays in requests also accept `axis_keys`
and `axis_names`.

<table>
<thead>
<tr>
<th>Atom</th>
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
<tr>
<td>NaN</td>
<td><code>{"nan":1}</code></td>
</tr>
</tbody>
</table>

Use the tags for infinities and NaN, never raw `NaN` or `Infinity`
tokens. Integer inputs stay exact. Because JavaScript’s `JSON.parse`
rounds integers larger than 2⁵³, use a parser that keeps integers exact.
Error spans are UTF-8 byte ranges into the supplied source. Error
reports include call sites as well as the original error location.

## Python worker

Python’s `Worker` manages a persistent `bpl --worker` subprocess with
deadlines and a hard-kill fallback.

``` python
from basedpl.worker import Worker

with Worker() as w:
    w.eval('v←ι10')
    r = w.eval('+/v', timeout=2)
    assert r['value'] == 45
```

Call `w.interrupt()` from another thread to cancel an evaluation. Ctrl-C
requests cancellation too. When the process responds to the
cancellation, the session and completed assignments are kept. If the
process doesn’t respond within the grace period, one second by default,
`Worker` kills it and the session is lost. `Worker` never replays a
request. `w.diagnostics` holds recent stderr.
`Worker.request(payload, timeout=...)` sends any request and supplies
the request’s `id`.
