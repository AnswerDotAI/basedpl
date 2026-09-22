⍝ pattern (f regex_replace) text — transform each match, retaining intervening text.
regex_replace←{
    p←•r ⍺ ⋄ s←⍵
    starts←p.position s
    0=≢starts:s
    ends←starts+p.length s
    gaps←(1,ends){(⍵-⍺)↑(⍺-1)↓s}¨starts,1+≢s
    replacements←⍶¨p.match s
    ∊gaps,¨replacements,⊂''
}
