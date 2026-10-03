

# `'` `"` — Character and string literals

`'c'` is one character. `"text"` is a string, a vector of characters of
any length.

``` bpl
⍴'a'               ⍝ ⍬ₓ
⍴"a"               ⍝ [1]ₓ
⍴""                ⍝ [0]ₓ
```

`'''` is the quote character. Inside a string, an apostrophe needs no
escaping, and a double quote is written twice.

``` bpl
•ucs '''           ⍝ 39ₓ
≢"say ""hi"""      ⍝ 8ₓ
```

Literals separated by spaces form one vector: `'a' 'b'` is `"ab"`, and
`"aa" "bb"` is a vector of two strings. See [Characters and
strings](../strings.ipynb).

## Errors

- `SYNTAX`: single quotes around more or fewer than one character, an
  unclosed literal
