⍝⍝ Regex

⍝ regex:matches — Bound functions share a compiled pattern
p←•r '[0-9]+' ⋄ (p.match 'abc123def45' ⋄ p.position 'abc123def45' ⋄ p.length 'abc123def45' ⋄ '#' p.replace 'abc123def45')
(('123' ⋄ '45') ⋄ 4x 10x ⋄ 3x 2x ⋄ 'abc#def#')

⍝ regex:groups — Captures exclude the whole match; absent captures are empty strings
p←•r '(a)?(b+)' ⋄ p.groups 'abbb b'
(((,'a') ⋄ 'bbb') ⋄ ('' ⋄ ,'b'))

⍝ regex:unicode — Positions count characters, not UTF-8 bytes
p←•r 'é|🐈+' ⋄ (p.position 'aé🐈🐈z' ⋄ p.length 'aé🐈🐈z')
(2x 3x ⋄ 1x 2x)

⍝ regex:empty — Empty matches occur at character boundaries
p←•r '' ⋄ (p.position 'é🐈' ⋄ '-' p.replace 'é🐈')
(1x 2x 3x ⋄ '-é-🐈-')

⍝ regex:nomatch — Typed empty results and unchanged replacement
p←•r '(z)' ⋄ (p.match 'abc' ⋄ p.position 'abc' ⋄ p.groups 'abc' ⋄ 'x' p.replace 'abc')
((0⍴⊂'') ⋄ (0⍴0x) ⋄ (0⍴⊂,⊂'') ⋄ 'abc')

⍝ regex:replacement — Rust capture expansion and literal dollar
p←•r '(?P<word>[a-z]+)([0-9]+)' ⋄ '${word}:$2:$$' p.replace 'ab12 cd3'
'ab:12:$ cd:3:$'

⍝ regex:each — Bound functions compose and work with Each
p←•r '(?i)cat' ⋄ f←≢∘p.match ⋄ f¨'Cat cat' 'dog'
2x 0x

⍝ regex:badpattern — Compile errors are located DOMAIN errors
•r 'a(b'
⍝ error: DOMAIN ERROR

⍝ regex:lookaround — Rust regex syntax
•r '(?=a)'
⍝ error: DOMAIN ERROR

⍝ regex:input — Text required
p←•r 'a' ⋄ p.match 1 2
⍝ error: DOMAIN ERROR

⍝ regex:valence — Replacement requires a left argument
p←•r 'a' ⋄ p.replace 'abc'
⍝ error: SYNTAX ERROR

⍝ regex:searchvalence — Search is monadic
p←•r 'a' ⋄ 'x' p.match 'abc'
⍝ error: SYNTAX ERROR

⍝ regex:callback-empty — No match does not call the APL operand
•load 'lib/regex.apl' ⋄ 'z' ({÷0} regex_replace) 'abc'
'abc'

⍝ regex:callback-empty-match — Retain intervening Unicode text
•load 'lib/regex.apl' ⋄ '' ({'-'} regex_replace) 'é🐈'
'-é-🐈-'
