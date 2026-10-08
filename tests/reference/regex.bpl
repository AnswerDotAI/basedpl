⍝⍝ Regex

⍝ regex:matches — A table of each match's text, position and groups, with replacement from the same pattern
p←•r "[0-9]+"
m←p.matches "abc123def45"
[m.text;m.position;≢¨m.text;'#' p.replace "abc123def45"]
⍝ =>
["123" "45";3 9;3 2;"abc#def#"]ₓ

⍝ regex:groups — Captures exclude the whole match; absent captures are empty strings
(•r "(a)?(b+)").matches "abbb b"
["text":["abbb" "b"] "position":[0 5]ₓ "groups":("a" "bbb"⋄"" "b")]

⍝ regex:unicode — Positions count characters, not UTF-8 bytes
p←•r "é|🐈+"
m←p.matches "aé🐈🐈z"
[m.position;≢¨m.text]   ⍝ [1 2;1 2]ₓ

⍝ regex:empty — Empty matches occur at character boundaries
p←•r "" ⋄ [(p.matches "é🐈").position;'-' p.replace "é🐈"]
[0 1 2;"-é-🐈-"]ₓ

⍝ regex:nomatch — Typed empty results and unchanged replacement
p←•r "(z)"
m←p.matches "abc"
[m.text;m.position;m.groups;'x' p.replace "abc"]
⍝ =>
[0⍴⊂"";⍬ₓ;0⍴⊂,⊂"";"abc"]

⍝ regex:replacement — The regex crate's capture expansion and literal dollar
p←•r "(?P<word>[a-z]+)([0-9]+)"
"${word}:$2:$$" p.replace "ab12 cd3"
⍝ =>
"ab:12:$ cd:3:$"

⍝ regex:each — Bound functions compose and work with Each
p←•r "(?i)cat"
f←≢∘("text"↣⊃)∘p.matches
f¨"Cat cat" "dog"   ⍝ [2 0]ₓ

⍝ regex:badpattern — Compile errors are located DOMAIN errors
•r "a(b"
⍝ error: DOMAIN ERROR

⍝ regex:lookaround — Rust regex syntax
•r "(?=a)"
⍝ error: DOMAIN ERROR

⍝ regex:input — Text required
p←•r 'a' ⋄ p.matches 1 2
⍝ error: DOMAIN ERROR

⍝ regex:valence — Replacement requires a left argument
p←•r 'a' ⋄ p.replace "abc"
⍝ error: SYNTAX ERROR

⍝ regex:searchvalence — Matching is monadic
p←•r 'a' ⋄ 'x' p.matches "abc"
⍝ error: SYNTAX ERROR

⍝ regex:callback-empty — No match does not call the APL operand
[regex_replace]←•load "lib/regex.bpl"
'z' ({÷0} regex_replace) "abc"   ⍝ "abc"

⍝ regex:callback-empty-match — Retain intervening Unicode text
[regex_replace]←•load "lib/regex.bpl"
"" ({'-'} regex_replace) "é🐈"   ⍝ "-é-🐈-"
