⍝ pattern (f regex_replace) text — transform each match, retaining intervening text.
regex_replace←{
    s←⍵ ⋄ p←•r ⍺ ⋄ m←p.matches s
    starts←m.position
    0ₓ=≢starts?s;
    ends←starts+≢¨m.text
    gaps←(0ₓ,ends){⍺↓⍵↑s}¨starts,≢s
    replacements←⍶¨m.text
    ∊gaps,¨replacements,⊂""
}
