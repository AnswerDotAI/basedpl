# Process interfaces

[Home](index.md) · [Command line](cli.md) · [Python](python.md)

## JSON lines

Run `bapl --json`. Send one JSON-encoded APL string per line; each receives one flushed JSON response. Names persist.

```json
"v←⍳10"
"+/v"
```

The second response is:

```json
{"value":55.0,"output":["55"],"error":null}
```

Use `json.dumps(code)` or `JSON.stringify(code)`, followed by a newline. Quotes are required; source newlines are escaped inside the string. Stdout contains only responses. The session remains usable after errors. EOF ends it.

Atoms are encoded directly. Arrays carry shape, row-major data and prototype, including rank-zero arrays. Nested elements use the same encoding.

| Element | JSON |
|---|---|
| Exact integer | Integer, including arbitrary precision |
| Approximate real | Number with decimal point or exponent |
| Character | String |
| Rational | `{"rational":["1","3"]}` |
| Complex | `{"complex":[1.0,2.0]}` |
| Infinity | `{"infinity":1}` or `{"infinity":-1}` |

Ordinary JavaScript `JSON.parse` can round large integers; use an integer-preserving parser. Error spans are UTF-8 byte ranges into the supplied source; call sites accompany the original location.

## Interruptible worker

Python's `Worker` manages a persistent subprocess with deadlines and a hard-kill fallback.

```python
from basedpl.worker import Worker

with Worker() as w:
    w.eval('v←⍳10')
    r = w.eval('+/v', timeout=2)
    assert r['value'] == 55
```

`w.interrupt()` cancels from another thread. Ctrl-C requests cancellation too. Cooperative cancellation preserves the session and completed assignments. After the grace period (default one second), an unresponsive process is killed and its session is lost. Requests are never replayed. `w.diagnostics` holds recent stderr.

## Worker protocol

`bapl --worker` accepts JSON objects with request IDs. Keep one evaluation outstanding.

```json
{"id":1,"code":"+/⍳10","timeout_ms":2000,"echo":false}
```

Replies have `{"id":1,"result":...}`. `{"interrupt":1}` cancels that request without a separate reply. `echo:false` suppresses implicit display, not explicit output.

`bindings` maps names to encoded values. Use `call` with one or two encoded `args` to apply a function without generating APL source.

```json
{"id":2,"call":"+","args":[2,3],"echo":false}
```

`Worker.request(payload, timeout=...)` supplies the ID for Python callers. Integer inputs remain exact. Invalid operations/arrays return `REQUEST ERROR`; malformed JSON or invalid control fields terminate the worker. Use tagged infinities, never raw NaN/Infinity JSON tokens.
