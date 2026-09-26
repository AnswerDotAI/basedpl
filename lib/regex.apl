⍝ pattern (f regex_replace) text — transform each match, retaining intervening text.
regex_replace←{
    p←•r ⍺ ⋄ s←⍵
    starts←p.position s
    0=≢starts:s
    ends←starts+p.length s
    gaps←(0,ends){(⍵-⍺)↑⍺↓s}¨starts,≢s
    replacements←⍶¨p.match s
    ∊gaps,¨replacements,⊂""
}
