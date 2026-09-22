# Names and help

[Home](index.md) · [Python](python.md)

`•nc N` gives the class of each name: `¯1` invalid, `0` undefined, `2` value, `3` function/hybrid, `4` operator. A character atom or vector names one binding. An array of strings retains its shape.

```apl
v←1 2 3 ⋄ mean←{(+/⍵)÷≢⍵}
•nc 'v'                   ⍝ 2x
•nc 'mean' 'absent'        ⍝ 3x 0x
```

`P •nl C` lists visible user names of classes `C`, beginning with prefix `P` (default `''`). Results are sorted string vectors. Shadowed names appear once; implicit argument/operand names are omitted.

```apl
mean←{(+/⍵)÷≢⍵}
'me' •nl 3                ⍝ ,⊂'mean'
'absent' •nl 2 3 4        ⍝ 0⍴⊂''
```

`•src N` returns definition text, including comments. Primitives and derived functions use their APL representation.

```apl
f←{⍵+1} ⋄ add←+
•src 'f'                  ⍝ '{⍵+1}'
•src 'add'                ⍝ ,'+'
```

`•ex N` erases the nearest binding. It returns `1` for removal or an already-absent name, `0` for an invalid/protected name. Erasing a local can reveal an outer binding. Retained function handles remain usable.

```apl
v←1
•ex 'v'                   ⍝ 1x
•ex 'v'                   ⍝ 1x
•ex '•a'                  ⍝ 0x
```

Errors: `•src` undefined name → VALUE; array → DOMAIN. Non-string names or unsupported `•nl` classes → DOMAIN. Rank >1 class list → RANK. `•nc`, `•src`, `•ex` are monadic.

## Interactive help

In the REPL and notebooks, `]help mean` shows help and `]help mean -source` shows source. A leading `⍝` comment block inside a definition supplies its help text. Primitive help comes from the glyph documentation.

```apl
mean←{⍝ Mean of a vector
    (+/⍵)÷≢⍵}
```

Enter `]help mean` to read “Mean of a vector”, or `]help mean -source` to see the dfn. Glyphs work too:

```apl
]help ?
```

In the APL kernel, Shift-Tab inspects the name or glyph at the cursor. Tab completes user names, system names and backtick glyph names.

Inspection reads definitions without executing them. Arbitrary dfns show the one-/two-argument calling convention without inferring their supported valence.

In IPython, importing `basedpl` enables `f?` and `f??` for Python `Function` objects, plus name completion inside `apl["…"]`, `apl("…")` and `apl.fn("…")`. `%load_ext basedpl.ipython` enables the adapter explicitly when needed.

```python
from basedpl import Session, plus

with Session() as apl:
    apl('mean←{⍝ Mean of a vector\n(+/⍵)÷≢⍵}')
    mean = apl.fn('mean')
    assert 'Mean of a vector' in mean.__doc__
    assert '(+/⍵)÷≢⍵' in mean.source
    assert apl.names('me') == ['mean']
    assert apl.inspect('mean')['kind'] == 'function'
    assert '•src' in apl.complete('•s')
```
