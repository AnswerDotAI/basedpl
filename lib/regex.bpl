⍝ pattern (f regex_replace) text — transform each match, retaining intervening text.
regex_replace←{
    s←ω ◇ m←s •r α
    starts←m.position
    0ₓ=≢starts?s;
    ends←starts+≢¨m.text
    gaps←0ₓ,ends {α↓ω↑s}¨ starts,≢s
    replacements←⍶¨m.text
    ∊gaps,¨replacements,⊂""
}
