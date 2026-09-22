# Regex

`p←•r pattern` compiles a [Rust regex](https://docs.rs/regex/latest/regex/#syntax). `p` is a keyed vector of functions sharing the compiled pattern.

```apl
p←•r '[0-9]+'
p.match 'abc123def45'       ⍝ ('123' ⋄ '45')
p.position 'abc123def45'    ⍝ 4ₓ 10ₓ
p.length 'abc123def45'      ⍝ 3ₓ 2ₓ
'#' p.replace 'abc123def45' ⍝ 'abc#def#'
```

Search returns a vector, including zero or one match. Matches are non-overlapping. Positions are 1-origin character indices; lengths count characters.

```apl
p←•r 'é|🐈+'
p.position 'aé🐈🐈z'   ⍝ 2ₓ 3ₓ
p.length 'aé🐈🐈z'     ⍝ 1ₓ 2ₓ
p.match 'abc'         ⍝ 0⍴⊂''
```

`groups` returns captured strings per match, excluding the whole match. An unmatched optional capture gives `''`.

```apl
p←•r '([a-z]+)([0-9]+)'
p.groups 'ab12 cd3'   ⍝ (('ab' ⋄ '12') ⋄ ('cd' ⋄ ,'3'))
```

Replacement expands `$0` (whole match), `$1`, `${name}` and `$$` (literal `$`). An unknown or unmatched capture expands to `''`.

```apl
p←•r '(?P<word>[a-z]+)([0-9]+)'
'${word}:$2:$$' p.replace 'ab12'   ⍝ 'ab:12:$'
```

Put flags in the pattern: `(?i)` ignores case, `(?m)` gives line anchors, `(?s)` lets `.` match newlines. Use Each for multiple strings.

```apl
p←•r '(?i)cat'
(≢∘p.match)¨'Cat cat' 'dog'   ⍝ 2ₓ 0ₓ
```

Empty matches occur at character boundaries.

```apl
p←•r ''
p.position 'é🐈'     ⍝ 1ₓ 2ₓ 3ₓ
'-' p.replace 'é🐈'  ⍝ '-é-🐈-'
```

`lib/regex.apl` supplies `pattern (f regex_replace) text`. It applies `f` to each matched string and retains intervening text.

```apl
•load 'lib/regex.apl'
'[0-9]+' ({⍕2×•json ⍵} regex_replace) 'a12 b3'   ⍝ 'a24 b6'
```

Python uses the same keyed function vector:

```python
from basedpl import Session

with Session() as apl:
    p = apl.fn('•r')(r'([a-z]+)([0-9]+)')
    assert p['replace']('$2:$1', 'ab12').py == '12:ab'
```

Errors: DOMAIN for invalid patterns (including look-around/backreferences) or non-text arguments; SYNTAX for wrong valence; LIMIT for oversized results.
