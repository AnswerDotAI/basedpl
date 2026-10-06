⍝ pattern (f regex_replace) text — transform each match, retaining intervening text.
regex_replace←{
    s←⍵ ⋄ p←•r ⍺ ⋄ m←p.matches s
    starts←m.position
    0=≢starts?s;
    ends←starts+≢¨m.text
    gaps←(0,ends){⍺↓⍵↑s}¨starts,≢s
    replacements←⍶¨m.text
    ∊gaps,¨replacements,⊂""
}
