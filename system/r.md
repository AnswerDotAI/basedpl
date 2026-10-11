

# `•r` — Regular expressions

`•r pattern` compiles a regular expression. It returns a keyed vector of
two functions that share the pattern. `p.matches text` gives a table of
the matches, as a record of columns: `text`, the matched text;
`position`, its position in characters from 0; and `groups`, the text of
each group, which is empty for a group that took no part.
`template p.replace text` replaces each match with `template`, expanded
with the match’s groups.

Natively the pattern is a Rust regex, with [the regex crate’s
syntax](https://docs.rs/regex/latest/regex/#syntax). Flags go in the
pattern, as in `(?i)`, and templates use `$1`, `${name}` and `$$`. In
the browser the pattern is a JavaScript regular expression, with
[JavaScript’s
syntax](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Guide/Regular_expressions),
and templates use `$1`, `$<name>` and `$$`. [Regular
expressions](../regex.ipynb) covers each function.

``` bpl
p←•r "([a-z]+)([0-9]+)"
p.matches "ab12 cd3"           ⍝ ["text":["ab12" "cd3"] "position":[0 5]ₓ "groups":("ab" "12"⋄"cd" "3")]
"$2:$1" p.replace "ab12 cd3"   ⍝ "12:ab 3:cd"
```

Errors: `DOMAIN` for invalid patterns and non-text arguments; `SYNTAX`
for the wrong valence; `LIMIT` for oversized results.
