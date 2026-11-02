⍝⍝ Regex

⍝ regex:matches — A table of each match's text, position and groups; replacement takes the pattern with a template
m←"abc123def45" •r "[0-9]+"
[m.text;m.position;≢¨m.text;"abc123def45" •replace ["[0-9]+" "#"]]
⍝ =>
["123" "45";3 9;3 2;"abc#def#"]ₓ

⍝ regex:groups — Captures exclude the whole match; absent captures are empty strings
"abbb b" •r "(a)?(b+)"
["text":["abbb" "b"] "position":[0 5]ₓ "groups":("a" "bbb"⋄"" "b")]

⍝ regex:unicode — Positions count characters, not UTF-8 bytes
m←"aé🐈🐈z" •r "é|🐈+"
[m.position ≢¨m.text]   ⍝ [1 2;1 2]ₓ

⍝ regex:empty — Empty matches occur at character boundaries
[("é🐈" •r "").position;"é🐈" •replace ["" "-"]]
[0 1 2;"-é-🐈-"]ₓ

⍝ regex:nomatch — Typed empty results and unchanged replacement
m←"abc" •r "(z)"
[m.text;m.position;m.groups;"abc" •replace ["(z)" "x"]]
⍝ =>
[0⍴⊂"" ⍬ₓ 0⍴⊂,⊂"" "abc"]

⍝ regex:replacement — Capture expansion and literal dollar
"ab12 cd3" •replace ["(?P<word>[a-z]+)([0-9]+)" "${word}:$2:$$"]
⍝ =>
"ab:12:$ cd:3:$"

⍝ regex:each — A bound pattern composes and works with Each
f←≢∘("text"↣⊃)∘(•r↢"(?i)cat")
f¨"Cat cat" "dog"   ⍝ [2 0]ₓ

⍝ regex:badpattern — Compile errors are located DOMAIN errors
"abc" •r "a(b"
⍝ error: DOMAIN ERROR

⍝ regex:lookaround — Lookahead finds positions without consuming text
("banana" •r "(?=a)").position   ⍝ [1 3 5]ₓ

⍝ regex:input — Text required
1 2 •r 'a'
⍝ error: DOMAIN ERROR

⍝ regex:valence — Matching needs a text on the left
•r "a"
⍝ error: SYNTAX ERROR

⍝ regex:replacevalence — Replacement needs a text on the left
•replace "a" "b"
⍝ error: SYNTAX ERROR

⍝ regex:rule — Replacement takes a pattern and a template
"abc" •replace "a"
⍝ error: DOMAIN ERROR

⍝ regex:callback-empty — No match does not call the APL operand
[regex_replace:]←•load "lib/regex.bpl"
'z' ({÷0} regex_replace) "abc"   ⍝ "abc"

⍝ regex:callback-empty-match — Retain intervening Unicode text
[regex_replace:]←•load "lib/regex.bpl"
"" ({'-'} regex_replace) "é🐈"   ⍝ "-é-🐈-"
