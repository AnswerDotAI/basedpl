⍝ String dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/string/string.apl; Apache-2.0, see LICENSE-april
⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_contents.htm into April APL

⍝⍝ String processing

⍝ From http://dfns.dyalog.com/c_just.htm

just ← {  ⍝ Justify text array.
  α←¯1ₓ  ⍝ left justify by default.
  α=¯1ₓ?+/∧\' '=ω ⌽ ω;             ⍝ │left        │
  α=1ₓ?-+/∧\' '=⌽ω ⌽ ω;             ⍝ │       right│
  (⌈0.5× +/∧\' '=ω - +/∧\' '=⌽ω)⌽ω  ⍝ │   centre   │
}

⍝ From http://dfns.dyalog.com/n_mtrim.htm

mtrim ← {⌽∨\⌽∨⌿ω≠' ' #₋₁ ω}  ⍝ Trim trailing blank cols from simple character matrix.

⍝ From http://dfns.dyalog.com/c_ss.htm

ss ← {  ⍝ Approx alternative to xutils' ss.
  [srce find repl]←,¨ω  ⍝ source, find and replace vectors.
  mask←find⍷srce  ⍝ mask of matching strings.
  prem←ρfind ↑ $t  ⍝ leading pre-mask.
  cvex←prem,mask ⊂ find,srce  ⍝ partitioned at find points.
  ρrepl ↓ ∊{repl , ρfind ↓ ω}¨cvex  ⍝ collected with replacements.
}

⍝ From http://dfns.dyalog.com/s_ssmat.htm

ssmat ← {  ⍝ Matrix search/replace.
  [cmat find repl]←ω  ⍝ Char matrix, find & replace vectors.
  {ss[ω find repl]}⍤1 cmat
}

⍝ From http://dfns.dyalog.com/c_squeeze.htm

squeeze ← {~"  "⍷ω # ω}  ⍝ Compress multiple blanks.

⍝ From http://dfns.dyalog.com/c_timestamp.htm

timestamp ← {  ⍝ Time-stamped message.
  join←{⊃⍶{α,⍶,ω}/ω}
  fmt←{-2⌈ρ⍬⍕ω ↑ '0',⍬⍕ω}¨  ⍝ format number with leading zeros.
  date←'-'join fmt 3↑α  ⍝ date: YYYY-MM-DD
  time←':'join fmt 3↑3↓α  ⍝ time: HH:MM:SS
  ' 'join[date time ω]  ⍝ blank-separated date, time and message.
}

⍝ From http://dfns.dyalog.com/c_htx.htm

htx ← {  ⍝ Extract html segments.
  1ₓ≠≡,ω?α∇{α,' ',ω}/ω;
  xtags←{seg sep squeeze vec ω}  ⍝ extract tags, where:
  seg←{0ₓ=2ₓ|ιρω # ω}
  sep←{(fm⍷ω ∨ to⍷ω)⊂ω}  ⍝ html separated at tags.
  vec←{(~ω∊•ucs 8 10 13){α#⁻α#ω}ω}
  rlt←{≥+/∧\'>'≠ω ↓ ω}  ⍝ remove leading tag.
  att←{ω,to,'>'}  ⍝ append trailing tag.
  [fm to]←'<' "</",¨⊂α~"<>"  ⍝ opening and closing html tags.
  '<'=↑α?att¨xtags,ω;
  rlt¨xtags,ω  ⍝ untagged segments.
}


⍝⍝ Line vector processing

⍝ From http://dfns.dyalog.com/c_ltov.htm

ltov ← {  ⍝ Lines to nested vector.
  α←•ucs 10 13 133
  1↓¨α{  ⍝ remove leading separators.
    ω∊α ⊂ ω  ⍝ split at separator.
  }¯1⌽ω,(~ ¯1↑ω ∊ α)#1↑α  ⍝ ensure trailing separator.
}

⍝ From http://dfns.dyalog.com/c_vtol.htm

vtol ← {  ⍝ Nested vector to lines.
  α←•ucs 10
  ,/ω,¨⊂α
}

⍝ From http://dfns.dyalog.com/c_wrap.htm

wrap ← {  ⍝ Wrap word vector at α cols.
  α←102ₓ  ⍝ default 102-wrap.
  α≥ρω?ω;  ⍝ short enough vector: finished.
  gaps←⍸ ' ' = ≥α ↑ ω
  take←¯1↑α,(α≥gaps)#gaps  ⍝ chars to take.
  drop←take+' '=take⊃ω
  head←take↑ω , •ucs10
  head,α∇drop↓ω  ⍝ wrapped following lines.
}

⍝ From http://dfns.dyalog.com/s_wrap.htm

wrap_2 ← {α←102ₓ  ⍝ α-wrap (Bob Smith).
  v←' ',ω,' '  ⍝ blanks required at start and end
  j←v=' ' # ιρv  ⍝ indices of blanks
  p←j+≥α <⊗ j
  m←p<1⌽p  ⍝ mark last blank that fits on the line
  i←(ρm)ρ1,(1↓ρm)ρ0
  c←⌹i-m  ⍝ compute transitive closure of m
  v.[c₀#j]←•ucs 10
  1↓¯1↓v  ⍝ drop the extra blanks
}

⍝ From http://dfns.dyalog.com/s_wrap.htm

wrap_3 ← {α←102ₓ  ⍝ α-wrap (John Daintree).
  α≥ρ,ω?,⊂ω;  ⍝ out if short enough
  sze←ω∊"-?., " # ≥ιρω  ⍝ length of each choice
  len←↑⌽ α≥sze # sze
  len←↑ len∊sze ⌽ [α len]
  [len↑ω],α∇len↓ω  ⍝ at valid wrap point
}

⍝ From http://dfns.dyalog.com/c_unwrap.htm

unwrap ← {(~ω∊•ucs 10 13 133){α#⁻α#ω}ω}  ⍝ Replace <LF> with blanks.

⍝ From http://dfns.dyalog.com/c_justify.htm

justify ← {  ⍝ Justify line-vector to width α.
  segs←{≤ω{ω,ρα - ¯1ₓ,ω}⍸ω}
  split←{(ω|α > ιω)+⌊α÷1ₓ⌈ω}
  [lf sp]←•ucs10 32 = ⊂ω
  sizes←segs lf  ⍝ line sizes.
  α←⌈/sizes  ⍝ default width is longest line.
  blanks←segs~ lf∨sp # sp  ⍝ original number of blanks per line.
  required←blanks+α-sizes  ⍝ required      ..      ..      ..
  breps←required split¨ blanks  ⍝ blank replication vectors.
  last←1ₓ⌈≤ρsizes  ⍝ last line split point.
  brep←∊ last↑breps , ×last↓breps  ⍝ blank replication vector.
  (~sp + sp#⁻∊brep)#ω  ⍝ α-justified line-vector.
}

⍝ From http://dfns.dyalog.com/c_vtrim.htm

vtrim ← {  ⍝ Trim trailing blanks from line-vector.
  [lf sp]←•ucs 10 32
  1↓¯1↓{  ⍝ without linefeeds brackets,
    types←[lf sp]ιω
    mask←~[1 1]ₓ⍷types  ⍝ mask of non-(duplicate blanks).
    comp←mask#types  ⍝ ignoring duplicate blanks,
    csl←[2 1 0]ₓ⍷comp  ⍝ sequence: ch, sp, ··· sp, lf.
    lsl←[0 1 0]ₓ⍷comp  ⍝ sequence: lf, sp, ··· sp, lf.
    from←mask#⁻csl∨lsl  ⍝ sequence: xx, sp, ··· sp, lf.
    upto←[1 0]ₓ⍷types  ⍝ sequence: ··········· sp, lf.
    ~¯1⌽≠\from∨upto # ω  ⍝ compressed out: <sp, ··· sp>, lf.
  }lf,ω,lf  ⍝ bracketed with linefeeds chars.
}

⍝ From http://dfns.dyalog.com/c_wrapnote.htm

wrapnote ← {                                 ⍝ Wrap text paragraphs in note vector.
  α←102ₓ                                     ⍝ by default: wrap to print width.
  split←{{1↓¨($t,1↓ω∊nls)⊂ω}' ',ω}           ⍝ partition at newline.
  join←{¯1↓α{α,⍶,ω}/ω,⊂""}                  ⍝ collect with α-separator.
  jrgt←{fm=≢ω}                               ⍝ line justified right?
  jlft←{' '≠↑ω}                              ⍝ line justified left?
  pics←{$t∊"┌┬┐├┼┤└┴┘│─"∊ω}                  ⍝ line contains box-drawing chars?
  and←{⍶ω ∧ ⍹ω}                              ⍝ test combiner.
  test←~∘pics and jlft and jrgt              ⍝ test for flowtext.
  first←{≥ωι0ₓ ⊃ ω}∘ ≢¨                    ⍝ length of first non-blank line.
  spill←{ω∨¯1⌽ω}                             ⍝ include next item to right.
  list←{squeeze dehyph' 'join ω}             ⍝ first enlist, then
  fold←{split to wrap ω}                     ⍝ re-wrap paragraph.
  dehyph←{~spill"- "⍷ω # ω}                  ⍝ re-join hyphen-separated words.

  nls←•ucs 10 13 133                         ⍝ version-proof newlines.
  nl←↑nls∩ω                                  ⍝ newline separator.
  [to fm]←2↑α,first 1↓split ω~"─"            ⍝ to and from notes width.
  vex←split ω                                ⍝ line_vector notes to vec-of-vecs.
  text←$f,1↓spill test¨vex                   ⍝ mask of lines of flowtext.
  segs←$t,1↓¯1⌽$f $t⍷text                    ⍝ partition at start of flowtext.

  vtrim nl join ,/(segs⊂text){               ⍝ re-collect lines of notes.
    $f=↑α?ω;                                  ⍝ no flowtext here: continue.
    flow←α#ω ◇ rest←~α # ω  ⍝ separate flowtext from the rest.
    vex←fold list flow                       ⍝ refold paragraph.
    wrp←to wrap ' 'join vex                  ⍝ re-re-wrap text.
    new←split to justify wrp                 ⍝ justify.
    new,rest                                 ⍝ re-collect segment.
  }¨segs⊂vex                                 ⍝ for each flowtext segment.
}

⍝ From http://dfns.dyalog.com/c_xtabs.htm

xtabs ← {  ⍝ Expand/compress HT chars.
  α←8ₓ  ⍝ default: 8-col tab stops.
  α=0ₓ?ω;  ⍝ α=0: no-op.
  chs←~ω∊•ucs 10 13 133
  α>0ₓ?α{  ⍝ +ive α: expand tabs → blanks.
    [tabs nabs]←$t $f=⊂ω∊•ucs[9]
    sync←tabs≥chs  ⍝ sync at tab and end of line.
    segs←≤{ω-¯1ₓ,¯1↓ω}⍸sync
    pads←0ₓ⌈α-α|(sync#tabs)#segs  ⍝ padding lengths.
    nabs+tabs#⁻pads # nabs#⁻nabs#ω  ⍝ padded char vector.
  }ω;
  α<0ₓ?(-α){  ⍝ -ive α: squeeze blanks → tabs.
    [bks nks]←$t $f=⊂ω=' '  ⍝ blanks and non-blanks
    runs←{ω{ω-⌈\ω×~α}+\ω}  ⍝ runs of adjacent 1s.
    tabs←bks∧chs∧0ₓ=α|runs chs  ⍝ tab positions.
    onoff←{(α≠ω){≠\α#⁻≠/↕¯1ₓ,α#ω}α-ω}
    pretab←⌽(⌽tabs)onoff⌽nks  ⍝ blanks that precede tabs.
    pretab≤tabs # (•ucs 9)@(⍸tabs) ω
  }ω
}

⍝ From http://dfns.dyalog.com/s_xtabs.htm

tabTrip ← {ω ≡ α xtabs -α xtabs ω}

tabTrips ← {∧/ ι2ₓ+ρω tabTrip¨ ⊂ω}


⍝⍝ Blank removal

⍝ Rule ⍶ applied to each item of a nested array, and to a higher-rank array as a matrix of its rows. α lists the blanks.
⍝ Shared by the blank-removal functions below.
deblank ← {
  α←" " ◇ 1ₓ<≡ω?⊂α ∇¨ ω;  ⍝ nested?
  2ₓ<ρρω?(¯1↓ρω){α,1↓ρω ρ ω}α∇,⍠ ¯1↓ιρρω ω;  ⍝ array
  α ⍶ ω
}

⍝ From http://dfns.dyalog.com/c_dlb.htm

dlb ← {  ⍝ Drop Leading Blanks.
  1ₓ≥ρρω?+/∧\ω∊α ↓ ω;  ⍝ vector
  ∨\∨⌿~ω∊α #₋₁ ω  ⍝ matrix
}deblank

⍝ From http://dfns.dyalog.com/c_dtb.htm

dtb ← {  ⍝ Drop Trailing Blanks.
  1ₓ≥ρρω?-+/∧\⌽ω∊α ↓ ω;  ⍝ vector
  ~⌽∧\⌽∧⌿ω∊α #₋₁ ω  ⍝ matrix
}deblank

⍝ From http://dfns.dyalog.com/c_deb.htm

deb ← {  ⍝ Drop Ending Blanks.
  b←ω∊α  ⍝ mask
  1ₓ≥ρρω?(∧\b ⊽ ⌽∧\⌽b)#ω;  ⍝ vector
  b∧⌿← ◇ (∧\b ⊽ ⌽∧\⌽b)#₋₁ ω  ⍝ matrix
}deblank

⍝ From http://dfns.dyalog.com/c_dmb.htm

dmb ← {  ⍝ Drop Multiple Blanks.
  2ₓ>ρρω?∨/↕$t,⍨~ω∊α # ω;
  ∨/↕$t,⍨,∨⌿~ω∊α #₋₁ ω
}deblank

⍝ From http://dfns.dyalog.com/c_dxb.htm

dxb ← {  ⍝ Drop eXtraneous Blanks.
  b←ω∊α  ⍝ mask
  1ₓ≥ρρω?1↑b ↓ b⊼1↓b,$t # ω;  ⍝ vector
  b∧⌿← ◇ 0,1↑b ↓ b⊼1↓b,$t #₋₁ ω  ⍝ matrix
}deblank

⍝ From http://dfns.dyalog.com/c_dab.htm

dab ← {  ⍝ Drop All Blanks.
  1ₓ≥ρρω?ω~α;  ⍝ vector
  ⊃ ↓ω ~¨ ⊂α  ⍝ matrix
}deblank
