

# Behavior of APL primitives in a system with depth-1 scalars (APL+1)

**Jacob Brickman**

Examples for the Minnowbrook 2017 talk “The Mathematical Foundations of
Arrays”.

Source: *APL – Journal* 1/2/2019, printed pages 3–13 (PDF pages 5–15 of
`APL_Journal_2019_12.pdf`). Manually transcribed from the rendered
pages. Section headings below group the examples; the APL notation,
dialect distinctions and display diagrams follow the article. The talk’s
square epsilon is rendered as `⋿`.

## Array definition

In the talk, an array of rank *r* was defined as the pair
((*s**h*), *I*), where (*s**h*) is the shape tuple
(*n*<sub>1</sub>, …, *n*<sub>*r*</sub>), and *I* is the index function,
defined on the index set
*S*<sub>*n*<sub>1</sub></sub> × ⋯ × *S*<sub>*n*<sub>*r*</sub></sub>,
where *S*<sub>*k*</sub> = {1, …, *k*}. For scalars, *r* = 0, so the
index set becomes the singleton set {*E*<sub>⌀</sub>}, where
*E*<sub>⌀</sub> : ⌀ → ⌀ is the empty function to the empty set. The
definition agrees with APL, except in some scalar cases. In all
dialects, for any X, `⊂X↔(X X)[1]`, and the definition confirms this.
Some of the consequences of the definition are the existence of depth-1
scalars, indexing a simple vector with a scalar (`v[1]`) returns a
depth-1 scalar, enclosing a simple scalar returns a depth-1 scalar
(`2≠⊂2`), reduction of a simple vector returns a depth-1 scalar,
non-collapsing scalar towers and consistent depth behavior. The set
member function (`∊`) is misleading when used with arrays because arrays
are not sets – they can have repeated values and order is significant,
none of which hold for sets. The definition also implies that `2∊2`
should return 0, not 1, and that `(⊂2 3)∊⊂⊂2 3` should return 1, not 0.
A different symbol should be used, and the talk introduced ⋿ (square
epsilon), where, for any X, `X⋿X` is false and `X⋿⊂X` is true.

APL+1 is different from Sharp APL, where `(2 2)[1]≠<2`. The box function
`<` is unrelated to the array definition. It produces scalars (boxes),
which are equivalent to single-member C structs, so `<` creates new
datatypes, not new arrays. From the point of view of array definition,
boxes are user-defined depth-0 scalars, so Sharp APL has a flat array
system extended laterally with boxes. It is possible to define `⊂` in
Sharp APL in the same way, and in that system we would still have
`⊂<2↔<2`, so Sharp APL has the same behavior as APL2 on an even larger
class of simple scalars, because of the common origin of the dialects.
If in Sharp APL we had `(2 2)[1]=<2`, then `<` would be the same as `⊂`
in APL+1. Pervasive functions work the same way in APL+1, because
pervasiveness is a property of the function, not the data.

**TAPL** = Traditional APL (without depth-1 scalars); **both APLs** =
TAPL and APL+1.

Primitives affected (APL2 definition):
`, ⊂ ⊃ ↑ ⍷ ∊ / \ + [] ~ ← ⌷ ⍴ ⌽ ⍉ ⊥ .`

## Indexing and scalar depth

*Printed page 3.*

``` apl
      ]display v←2 3    ⍝ both APLs
.→--.
|2 3|
'~--'
```

``` apl
      ]display v[1]    ⍝ depth-0 scalar; TAPL
2
```

``` apl
      ]display v[1]    ⍝ depth-1 scalar; APL+1
.-.
|2|
'~'
```

``` apl
      ]display v[,1]   ⍝ both APLs
.→.
|2|
'~'
```

``` apl
      ]display ((2 3)(4 5))[1]    ⍝ both APLs
.-----.
|.→--.|
||2 3||
|'~--'|
'∊----'
```

## Enclose, First, Take, Enlist and Pick

*Printed page 4.*

``` apl
      ]display 2       ⍝ depth-0 scalar; both APLs
2
```

``` apl
      ]display ⊂2      ⍝ depth-1 scalar; same as v[1]; APL+1
.-.
|2|
'~'
```

``` apl
      ]display ⊂⊂2     ⍝ depth-2 scalar tower; APL+1
.---.
|.-.|
||2||
|'~'|
'∊--'
```

``` apl
      ]display ⊂,2     ⍝ both APLs
.---.
|.→.|
||2||
|'~'|
'∊--'
```

``` apl
      ]display v[⊂1]   ⍝ depth-1 scalar; same as v[1]; APL+1
.-.
|2|
'~'
```

``` apl
      ]display ↑⊂2     ⍝ get depth-0 scalar from depth-1 scalar; APL+1
2
```

``` apl
      ]display ↑⊂⊂2    ⍝ get depth-1 scalar from depth-2 scalar; APL+1
.-.
|2|
'~'
```

``` apl
      ]display 2↑⊂2    ⍝ overtake depth-1 scalar; APL+1
.→--.
|2 0|
'~--'
```

``` apl
      ]display 2↑⊂⊂2   ⍝ overtake depth-2 scalar tower; APL+1
.→-----.
|.-..-.|
||2||0||
|'~''~'|
'∊-----'
```

``` apl
      ]display ∊2      ⍝ both APLs
.→.
|2|
'~'
```

``` apl
      ]display ∊⊂2     ⍝ APL+1
.→.
|2|
'~'
```

``` apl
      ]display ∊⊂⊂2    ⍝ APL+1
.→.
|2|
'~'
```

``` apl
      ]display (⊂⍳0)⊃2    ⍝ both APLs
2
```

``` apl
      ]display (⊂⍳0)⊃⊂2   ⍝ get depth-0 scalar from depth-1 scalar; APL+1
2
```

## Pervasion and catenation

*Printed page 5.*

``` apl
      ]display 2+⊂2    ⍝ APL+1
.-.
|4|
'~'
```

``` apl
      ]display 2+⊂⊂2   ⍝ APL+1
.---.
|.-.|
||4||
|'~'|
'∊--'
```

``` apl
      ]display (⊂2)+⊂⊂2    ⍝ APL+1
.---.
|.-.|
||4||
|'~'|
'∊--'
```

``` apl
      ]display (⊂⊂⊂⊂2)+⊂⊂2    ⍝ APL+1
.-------.
|.-----.|
||.---.||
|||.-.|||
||||4||||
|||'~'|||
||'∊--'||
|'∊----'|
'∊------'
```

``` apl
      ]display (⊂⊂⊂,2)+⊂,2    ⍝ both APLs
.-------.
|.-----.|
||.→--.||
|||.→.|||
||||4||||
|||'~'|||
||'∊--'||
|'∊----'|
'∊------'
```

``` apl
      ]display (⊂⊂⊂,2)+⊂⊂,2   ⍝ both APLs
.-------.
|.-----.|
||.→--.||
|||.→.|||
||||4||||
|||'~'|||
||'∊--'||
|'∊----'|
'∊------'
```

``` apl
      ]display 2,3     ⍝ both APLs
.→--.
|2 3|
'~--'
```

``` apl
      ]display (⊂2),⊂3    ⍝ joining depth-1 scalars; APL+1
.→--.
|2 3|
'~--'
```

## Strands, Each and reduction

*Printed page 6.*

``` apl
      ]display (⊂,2),⊂,3    ⍝ both APLs
.→-----.
|.→..→.|
||2||3||
|'~''~'|
'∊-----'
```

``` apl
      ]display (,2)(,3)     ⍝ both APLs
.→-----.
|.→..→.|
||2||3||
|'~''~'|
'∊-----'
```

``` apl
      ]display (⊂2)(⊂3)     ⍝ APL+1
.→-----.
|.-..-.|
||2||3||
|'~''~'|
'∊-----'
```

``` apl
      ]display ⊂¨2 3    ⍝ APL+1, same as ⊂[⍳0]
.→-----.
|.-..-.|
||2||3||
|'~''~'|
'∊-----'
```

``` apl
      ]display ⊂¨2 3    ⍝ TAPL, same as ⊂[⍳0]
.→--.
|2 3|
'~--'
```

``` apl
      ]display +/⊂2     ⍝ APL+1
.-.
|2|
'~'
```

``` apl
      ]display +/v      ⍝ APL+1
.-.
|5|
'~'
```

``` apl
      ]display +/v      ⍝ TAPL
5
```

``` apl
      ]display +/(1 2)(3 4)(5 6)(7 8)    ⍝ both APLs
.-------.
|.→----.|
||16 20||
|'~----'|
'∊------'
```

``` apl
      ]display +\(1 2)(3 4)(5 6)(7 8)    ⍝ both APLs
.→----------------------.
|.→--..→--..→---..→----.|
||1 2||4 6||9 12||16 20||
|'~--''~--''~---''~----'|
'∊----------------------'
```

## Scan, replicate and empty prototypes

*Printed page 7.*

``` apl
      ]display +\,¨2 3 4    ⍝ both APLs
.→--------.
|.→..→..→.|
||2||5||9||
|'~''~''~'|
'∊--------'
```

``` apl
      ]display +\⊂¨,¨2 3 4    ⍝ both APLs
.→--------------.
|.---..---..---.|
||.→.||.→.||.→.||
|||2||||5||||9|||
||'~'||'~'||'~'||
|'∊--''∊--''∊--'|
'∊--------------'
```

``` apl
      ]display +\⊂¨2 3 4    ⍝ APL+1
.→--------.
|.-..-..-.|
||2||5||9||
|'~''~''~'|
'∊--------'
```

``` apl
      ]display 1/2    ⍝ both APLs
.→.
|2|
'~'
```

``` apl
      ]display 1/⊂2   ⍝ APL+1
.→.
|2|
'~'
```

``` apl
      ]display 0/2    ⍝ both APLs
.⊖.
|0|
'~'
```

``` apl
      ]display 0/⊂2   ⍝ APL+1
.⊖.
|0|
'~'
```

``` apl
      ]display 0/⊂,2  ⍝ both APLs
.⊖--.
|.→.|
||0||
|'~'|
'∊--'
```

``` apl
      ]display 0/⊂⊂2  ⍝ APL+1
.⊖--.
|.-.|
||0||
|'~'|
'∊--'
```

``` apl
      ]display (1 2)(3 4)~3 4    ⍝ both APLs
.→---------.
|.→--..→--.|
||1 2||3 4||
|'~--''~--'|
'∊---------'
```

## Without, membership and Find

*Printed page 8.*

``` apl
      ]display (1 2)(3 4)~⊂3 4    ⍝ both APLs
.→----.
|.→--.|
||1 2||
|'~--'|
'∊----'
```

``` apl
      ]display (⊂3 4)~⊂3 4    ⍝ both APLs
.⊖----.
|.→--.|
||0 0||
|'~--'|
'∊----'
```

``` apl
      ]display (⊂,3)~3    ⍝ both APLs
.→--.
|.→.|
||3||
|'~'|
'∊--'
```

``` apl
      ]display (⊂,3)~,3   ⍝ both APLs
.→--.
|.→.|
||3||
|'~'|
'∊--'
```

``` apl
      ]display (,3)~3     ⍝ both APLs
.⊖.
|0|
'~'
```

``` apl
      ]display (⊂3)~3     ⍝ APL+1
.⊖.
|0|
'~'
```

``` apl
      ]display 2∊2        ⍝ should be 0
1
```

``` apl
      ]display (⊂2 3)∊⊂⊂2 3    ⍝ should be 1
0
```

``` apl
      ]display 2⍷2        ⍝ both APLs
1
```

``` apl
      ]display 2⍷,2       ⍝ both APLs
.→.
|1|
'~'
```

``` apl
      ]display (,2)⍷2     ⍝ TAPL
0
```

``` apl
      ]display (,2)⍷⊂2    ⍝ APL+1
0
```

``` apl
      ]display a←⊂⊂,2     ⍝ both APLs, depth 3
.-----.
|.---.|
||.→.||
|||2|||
||'~'||
|'∊--'|
'∊----'
```

## Pick assignment, indexing and reshape

*Printed page 9.*

``` apl
      ]display (⍳0)(⍳0)⊃a    ⍝ get innermost vector; both APLs
.→.
|2|
'~'
```

``` apl
      ((⍳0)(⍳0)⊃a)←2    ⍝ replace inner vector with simple scalar; both APLs
```

``` apl
      ]display a        ⍝ TAPL, collapsing tower
2
```

``` apl
      ]display a        ⍝ APL+1, depth-2 tower
.---.
|.-.|
||2||
|'~'|
'∊--'
```

``` apl
      ]display (⍳0)⌷2       ⍝ both APLs
2
```

``` apl
      ]display (⍳0)⌷⊂,2     ⍝ both APLs
.---.
|.→.|
||2||
|'~'|
'∊--'
```

``` apl
      ]display 1⌷2 3    ⍝ TAPL, depth-0 scalar, same as v[1]
2
```

``` apl
      ]display 1⌷2 3    ⍝ APL+1, depth-1 scalar, same as v[1]
.-.
|2|
'~'
```

``` apl
      ]display (⊂1)⌷2 3    ⍝ APL+1, depth-1 scalar, same as v[1]
.-.
|2|
'~'
```

``` apl
      ]display (,1)⌷2 3    ⍝ TAPL, depth-0 scalar, same as v[1]
2
```

``` apl
      ]display (,1)⌷2 3    ⍝ APL+1, depth-1 scalar, same as v[1]
.-.
|2|
'~'
```

``` apl
      ]display (⊂,1)⌷2 3   ⍝ both APLs, same as v[,1]
.→.
|2|
'~'
```

``` apl
      ]display (⊂1 2)⌷2 3  ⍝ both APLs, same as v[1 2]
.→--.
|2 3|
'~--'
```

``` apl
      ]display (⍳0)⍴2 3    ⍝ TAPL, depth-0 scalar
2
```

``` apl
      ]display (⍳0)⍴2 3    ⍝ APL+1, depth-1 scalar, depth preserving
.-.
|2|
'~'
```

## Reshape, Rotate, Transpose and Mix

*Printed page 10.*

``` apl
      ]display (⍳0)⍴(2 3)(4 5)    ⍝ both APLs, depth preserving
.-----.
|.→--.|
||2 3||
|'~--'|
'∊----'
```

``` apl
      1⌽2 3 4       ⍝ both APLs
3 4 2
```

``` apl
      (,1)⌽2 3 4    ⍝ both APLs
3 4 2
```

``` apl
      (⊂1)⌽2 3 4    ⍝ APL+1
3 4 2
```

``` apl
      1⍉2 3 4       ⍝ both APLs
2 3 4
```

``` apl
      (,1)⍉2 3 4    ⍝ both APLs
2 3 4
```

``` apl
      (⊂1)⍉2 3 4    ⍝ APL+1
2 3 4
```

``` apl
      ]display ,¨2 3 4    ⍝ both APLs
.→--------.
|.→..→..→.|
||2||3||4||
|'~''~''~'|
'∊--------'
```

``` apl
      ]display ⊃,¨2 3 4   ⍝ both APLs
.→.
↓2|
|3|
|4|
'~'
```

``` apl
      ]display ⊂¨,¨2 3 4    ⍝ both APLs
.→--------------.
|.---..---..---.|
||.→.||.→.||.→.||
|||2||||3||||4|||
||'~'||'~'||'~'||
|'∊--''∊--''∊--'|
'∊--------------'
```

``` apl
      ]display ⊃⊂¨,¨2 3 4    ⍝ both APLs
.→--------.
|.→..→..→.|
||2||3||4||
|'~''~''~'|
'∊--------'
```

``` apl
      ]display ⊂¨⊂¨,¨2 3 4    ⍝ both APLs
.→--------------------.
|.-----..-----..-----.|
||.---.||.---.||.---.||
|||.→.||||.→.||||.→.|||
||||2||||||3||||||4||||
|||'~'||||'~'||||'~'|||
||'∊--'||'∊--'||'∊--'||
|'∊----''∊----''∊----'|
'∊--------------------'
```

## Mix and inner product

*Printed page 11.*

``` apl
      ]display ⊃⊂¨⊂¨,¨2 3 4    ⍝ both APLs
.→--------------.
|.---..---..---.|
||.→.||.→.||.→.||
|||2||||3||||4|||
||'~'||'~'||'~'||
|'∊--''∊--''∊--'|
'∊--------------'
```

``` apl
      ]display ⊂¨2 3 4    ⍝ APL+1
.→--------.
|.-..-..-.|
||2||3||4||
|'~''~''~'|
'∊--------'
```

``` apl
      ]display ⊃⊂¨2 3 4   ⍝ both APLs
.→----.
|2 3 4|
'~----'
```

APL2 Language Reference Manual:

``` apl
L f.g R ↔ f/¨ (⊂[⍴⍴L]L)∘.g ⊂[1]R
```

``` apl
Z[I;J] ↔ ⊂f/L[I;] g R[;J]
```

``` apl
      ]display 2+.×,¨10 20    ⍝ both APLs (APL2, APL+WIN)
.------.
|.----.|
||.→-.||
|||60|||
||'~-'||
|'∊---'|
'∊-----'
```

``` apl
      ]display 2+.×,¨10 20    ⍝ both APLs (Dyalog)
.----.
|.→-.|
||60||
|'~-'|
'∊---'
```

``` apl
      ]display 2+.×⊂¨,¨10 20    ⍝ both APLs (APL2, APL+WIN)
.--------.
|.------.|
||.----.||
|||.→-.|||
||||60||||
|||'~-'|||
||'∊---'||
|'∊-----'|
'∊-------'
```

``` apl
      ]display 2+.×⊂¨,¨10 20    ⍝ both APLs (Dyalog)
.------.
|.----.|
||.→-.||
|||60|||
||'~-'||
|'∊---'|
'∊-----'
```

## Inner product and Decode

*Printed page 12.*

``` apl
      ]display 2+.×⊂¨10 20    ⍝ APL+1 (APL2, APL+WIN)
.------.
|.----.|
||.--.||
|||60|||
||'~-'||
|'∊---'|
'∊-----'
```

``` apl
      ]display 2+.×⊂¨10 20    ⍝ APL+1 (Dyalog)
.----.
|.--.|
||60||
|'~-'|
'∊---'
```

``` apl
      ]display 2+.×⊂¨10 20    ⍝ TAPL
60
```

``` apl
      ]display 1 2+.×10 20    ⍝ TAPL
50
```

``` apl
      ]display 1 2+.×10 20    ⍝ APL+1 (APL2, APL+WIN), depth-2 scalar tower
.----.
|.--.|
||50||
|'~-'|
'∊---'
```

``` apl
      ]display 1 2+.×10 20    ⍝ APL+1 (Dyalog), depth-1 scalar
.--.
|50|
'~-'
```

APL2 Language Reference Manual:

``` apl
L ⊥ R ↔ ((⍴L)↑⌽1,×\⌽1↓[⍴⍴L]L) +.× R
```

``` apl
      ]display 10⊥2 3 4    ⍝ TAPL
234
```

``` apl
      ]display 10⊥2 3 4    ⍝ APL+1 (APL2, APL+WIN), depth-2 scalar tower
.-----.
|.---.|
||234||
|'~--'|
'∊----'
```

``` apl
      ]display 10⊥2 3 4    ⍝ APL+1 (Dyalog), depth-1 scalar
.---.
|234|
'~--'
```

``` apl
      ]display (,10)⊥2 3 4    ⍝ TAPL
234
```

``` apl
      ]display (,10)⊥2 3 4    ⍝ APL+1 (APL2, APL+WIN), depth-2 scalar tower
.-----.
|.---.|
||234||
|'~--'|
'∊----'
```

``` apl
      ]display (,10)⊥2 3 4    ⍝ APL+1 (Dyalog), depth-1 scalar
.---.
|234|
'~--'
```

*Printed page 13.*

``` apl
      ]display (⊂10)⊥2 3 4    ⍝ APL+1 (APL2, APL+WIN), depth-2 scalar tower
.-----.
|.---.|
||234||
|'~--'|
'∊----'
```

``` apl
      ]display (⊂10)⊥2 3 4    ⍝ APL+1 (Dyalog), depth-1 scalar
.---.
|234|
'~--'
```
