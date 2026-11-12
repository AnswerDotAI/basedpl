⍝ Array dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/array/array.apl; Apache-2.0, see LICENSE-april

⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_contents.htm into April APL

⍝⍝ Array processing

⍝ From http://dfns.dyalog.com/n_alists.htm

alpush ← {  ⍝ Association list α prefixed with (key value) pair ω.
  α,⍨↢⊂¨ω  ⍝ :: list ← list ∇ key val
}

alpop ← {  ⍝ Leftmost value for key ω from list α
  [keys vals]←α  ⍝ keys vector and corresponding values
  indx←keysι⊂ω  ⍝ index of first key ω in list α
  val←indx⊃vals  ⍝ value for key ω
  list←⊂indx≠ιρkeys #¨ α  ⍝ reduced list
  [val list]  ⍝ :: val list ← list ∇ key
}

alget ← {  ⍝ Value for key ω in association list α.
  [keys vals]←α  ⍝ keys vector and corresponding values
  keysι⊂ω ⊃ vals  ⍝ :: val ← list ∇ key
}

alset ← {  ⍝ Assoc list α with (key value) pair ω replaced.
  [key val]←ω  ⍝ key and new value
  {val@ αι⊂key ω}\α
}

⍝ From http://dfns.dyalog.com/c_acc.htm

acc ← {⍶{[α⍶↑⍬ρω],ω}/1↓{ω,⊂⍬ρω}¯1⌽ω}  ⍝ Accumulating reduction.

⍝ Helpers shared by disp, display, displays, displayr and dsp.
format ← {t←⊃,↓⍬⍕ω ◇ ¯2↑1 1,ρt ρ t}  ⍝ Simple array as a character matrix.
open ← {1⌈ρω ρ ω}  ⍝ Exposure of null axes.
axes ← {-2⌈ρρω ↑ ≥×ρω}  ⍝ Array axis types.
char ← {⍬≡ρω?'─';↑ω∊'¯',$d ⊃ "#~"}∘⍕  ⍝ Type character of an atom.
type ← {{1ₓ=ρω ⊃ ['+' ω]}∪,char¨ω}  ⍝ Type characters of an array.
deco ← {α←type open ω ◇ α,axes ω}  ⍝ Type and axes vector.
trim ← {~$t $t⍷∧⌿ω=' ' #₋₁ ω}  ⍝ Removal of extra blank columns.

⍝ From http://dfns.dyalog.com/c_disp.htm

disp ← { α←⍬  ⍝ Boxed sketch of nested array.
  [dec ctd]←2↑α  ⍝ 1:decorated, 1:centred.
  box←{  ⍝ Recursive boxing of nested array.
    isor ω?format⊂ω;
    1ₓ=≡,ω?dec open format dec open ω;
    mat←matr 1# dec open ω  ⍝ matrix of opened subarrays.
    [r c]←×ρmat  ⍝ non-null rows/cols.
    dec<0ₓ∊[r c]?c#₋₁ r# ∇ 1 open mat;  ⍝ undecorated null: empty result.
    subs←aligned ∇¨mat  ⍝ aligned boxed subarrays.
    ≢ρω gaps ω plane subs  ⍝ collection into single plane.
  }
  aligned←{  ⍝ Alignment and centring.
    [rows cols]←seprρ¨ω  ⍝ subarray dimensions.
    sizes←⌈/rows ,⊗ ⌈⌿cols
    ctd=0?sizes↑¨ω;  ⍝ top-left alignment.
    [v h]←sepr⌈0.5×⊃(ρ¨ω)-sizes
    v⌽₀¨h⌽¨sizes↑¨ω  ⍝ centred aligned subarrays.
  }
  gaps←{  ⍝ Gap-separated sub-planes.
    α≤2ₓ?ω;  ⍝ lowish rank: done.
    subs←≤α ∇¨ ω  ⍝ sub-hyperplanes.
    width←↑⌽ρ↑subs
    fill←[α width]-3 0 ρ ' '  ⍝ inter-plane gap.
    ⊃{α⍪fill⍪ω}/ 1 open subs
  }
  plane←{  ⍝ Boxed rank-2 plane.
    2ₓ<ρρα?α join ω;  ⍝ gap-separated sub-planes.
    odec←dec⁝shapeα outer ω  ⍝ outer type and shape decoration.
    idec←inner α  ⍝ inner type and shape decorations.
    odec,idec collect ω  ⍝ collected, formatted subarrays.
  }
  join←{  ⍝ Join of gap-separated sub-planes.
    sep←≢ω ÷ 1ₓ⌈≢α  ⍝ sub plane separation.
    split←0ₓ=sep|ι≢ω ⊂₀ ω
    ⊂⁻¹α plane¨ split  ⍝ sub-plane join.
  }
  outer←{  ⍝ Outer decoration.
    sizes←1 0{↑↓ ⍉⍣α ω}¨seprρ¨ω
    sides←sizes#¨¨"│─"  ⍝ vert and horiz cell sides.
    bords←dec↓¨"├┬"glue¨sides  ⍝ joined up outer borders.
    ,¨/[['┌' ""] α bords "└┐"]
  }
  inner←{  ⍝ Inner subarray decorations.
    deco←{typeω , 1 shape ω}  ⍝ type and shape decorators.
    sepr deco¨matr dec open ω  ⍝ decorators: tt vv hh .
  }
  collect←{  ⍝ Collected subarrays.
    [lft top tt vv hh]←α  ⍝ array and subarray decorations.
    cells←vv right 1 open [tt hh] lower ω  ⍝ cells boxed right and below.
    boxes←dec∨0∊ρω open cells  ⍝ opened to avoid ,/⍬ problem.
    lft,top⍪⍪⌿,/boxes
  }
  right←{  ⍝ Border right each subarray.
    types←⊥¨ ιρω = ⊂≤ρω
    chars←[types]⌷"┼┤┴┘"
    rgt←{ω , -≢ω ↑ [≢ω 1 1]#'│',α}  ⍝ form right border.
    (matr 1 open α),¨chars rgt¨ ω  ⍝ cells bordered right.
  }
  lower←{  ⍝ Border below each subarray.
    split←{(¯2+1⊃ρω # '─')glue α}
    bot←{ω⍪ -1⊃ρω ↑ α split ω}
    matr,¨/α bot¨ matr ω
  }
  type←{  ⍝ Type decoration char.
    dec<≡ω?'─';  ⍝ nested: '─'
    isor ω?'∇';  ⍝ ⎕or:    '∇'
    sst←{  ⍝ atom type.
      0ₓ=dec×ρρω?'─';  ⍝ undecorated or atom ⍕ω: char,
      ↑ω∊'¯',$d ⊃ "#~"
    }∘⍕  ⍝ ⍕ distinguishes type of atom.
    0ₓ=≡ω?sst ω;  ⍝ atom: type.
    {(1ₓ=ρω)⊃['+' ω]}∪,sst¨(dec open ω)
  }
  shape←{
    dec≤0ₓ=ρρω?α#¨"│─";  ⍝ no decoration or unit.
    cols←×¯1↑ρω ⊃ "⊖→"
    rsig←××/¯1↓ρω ⊃ "⌽↓"
    rows←≤3⌊ρρω ⊃ ['│' rsig '⍒']
    [rows cols]  ⍝ shape decorators.
  }
  matr←{⊃,↓ω}
  sepr←{+/¨1⊂₋₁⊃ω}
  open←{α⌈ρω ↑ ω}
  isor←{1ₓ ⍬≡[≡ω ρω]}  ⍝ is ⎕or of object?
  glue←{0ₓ=ρω?ω;α{α,⍶,ω}/ω}
  isor ω?format⊂ω;
  1ₓ=≡,ω?format ω;
  box ω  ⍝ recursive boxing of array.
}

⍝ From http://dfns.dyalog.com/c_display.htm

display ← {  ⍝ Boxed display of array.
  box←{  ⍝ box with type and axes
    [vrt hrz]←≤ρω ρ¨ "│─"  ⍝ vert. and horiz. lines
    top←"─⊖→".[¯1↑α],hrz
    bot←↑α , hrz
    rgt←"┐│",vrt,'┘'  ⍝ right side with corners
    lax←"│⌽↓".[¯1↓1↓α],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪ω⍪bot),rgt  ⍝ fully boxed array
  }
  line←{""≡0ρω ⊃ " -"}
  0ₓ=≡ω?' ' ⍪ open⁝formatω ⍪ line ω;
  1ₓ ⍬≡[≡ω ρω]?'∇' 0 0 box format ω;
  1ₓ=≡ω?decoω box open format open ω;
  '∊'decoω box trim format ∇¨open ω
}

⍝ From http://dfns.dyalog.com/c_displays.htm

displays ← {  ⍝ Boxed display of array.
  box←{  ⍝ Box with type and axes.
    [shp w]←open\ω
    [vrt hrz]←≤ρw ρ¨ "│─"  ⍝ Vert. and horiz. lines.
    top←"─⊖→".[¯1↑α],hrz
    ok←ρshp < ρhrz
    top←ρtop ↑ 2↑top , ok#shp , (2+ok×ρshp)↓top
    bot←↑α , hrz
    rgt←"┐│",vrt,'┘'  ⍝ Right side with corners.
    lax←"│⌽↓".[¯1↓1↓α],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪w⍪bot),rgt  ⍝ Fully boxed array.
  }
  trim←{[ω₀ (~$t $t⍷∧⌿' '=ω₁)#₋₁ω₁]}
  qfmt←{[⍬⍕ρα;format open ω]}
  0ₓ=≡ω?' ' ⍪ formatω ⍪ ' '≡↑0ρω ⊃ " -";
  1ₓ ⍬≡[≡ω ρω]?'∇' 0 0 box ["─";format ω];
  1ₓ=≡ω?decoω box open ω qfmt ω;  ⍝ Simple array.
  '∊'decoω box trim ω qfmt ∇¨open ω  ⍝ Nested array.
}

⍝ From http://dfns.dyalog.com/c_displayr.htm

displayr ← {  ⍝ Boxed display of array
  box←{  ⍝ box with type and axes
    [vrt hrz]←≤ρω ρ¨ "│─"  ⍝ vert. and horiz. lines
    top←(≥ρhrz)↑(↑ ¯1↑α ⌷ "─⊖",⊂⍬⍕¯1↑α₁),hrz
    bot←ρtop ↑ ↑2↓α , hrz
    rgt←"┐│",vrt,'┘'  ⍝ right side with corners
    lax←(↑¨ ¯1↓3↓α ⌷¨ -1⌈≤ρα₁ ↑ ⊂"│⌽" ,¨ (⊂⍬↣⍕)¨¯1↓0ₓ,α₁),¨⊂vrt
    lax←⊂≥ρvrt ↑¨ lax~¨⊂" " ,¨ '│'  ⍝ pad and trim
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪ω⍪bot),rgt  ⍝ fully boxed array
  }
  deco←{α←type open ω ◇ ρρω , ⊂0ₓ+ρω , α , axes ω}
  0ₓ=≡ω?' ' ⍪ open⁝formatω ⍪ ' '=↑0ρω ⊃ " -";
  1ₓ ⍬≡[≡ω ρω]?["" [0 0]ₓ '∇' 0 0] box format ω;
  1ₓ=≡ω?decoω box open' ',format open ω;
  (⊂⍬⍕≡ω deco ω)box trim' ',format ∇¨open ω
}

⍝ From http://dfns.dyalog.com/c_dist.htm

dist ← {  ⍝ Levenshtein distance.
  a←(≥n)ρ(ρα)+n←ρω  ⍝ first row of matrix
  f←ω{⌊\ω⌊(↑ω),(¯1↓ω)-≥α=⍶}
  z←a f/ ⌽α
  ↑⌽z
}

fuzzy ← {a←lcase α ◇ ↑⍋a↣dist∘lcase¨ω ⊃ ω}

⍝ From http://dfns.dyalog.com/n_dsp.htm

dsp ← {  ⍝ Reduced version of disp.
  1ₓ=≡,ω ∨ 0ₓ∊ρω?format ω;
  α←1 ◇ top←'─'⍪⍣α  ⍝ top '─' bar if α
  1ₓ≥ρρω?{  ⍝ vector or unit:
    bars←{⍪(⌊/≢¨[α ω])#'│'}/↕ω,0
    join←{⊃,/(⌈/≢¨ω)↑¨ω}
    0 ¯1↓join top¨join¨↓⍉[ω◇bars]
  } 1 ∇¨ ω;  ⍝ vector: formatted items
  subs←α ∇¨ ω  ⍝ higher rank: formatted items
  [rs cs]←+/¨1⊂₋₁⊃ρ¨subs
  dims←(mrs←⌈/rs) ,⊗ mcs←⌈/⍪csᵀ
  join←{α{α,⍶,ω}/ω}
  rows←mrs#¨'│' join¨ ↓dims↑¨subs  ⍝ complete rows with '│'-separated items
  hzs←'┼'join mcs#¨'─'  ⍝ inter-row horizontal "─┼─" separators
  cells←{α⍪hzs⍪ω}/rows  ⍝ joined rows: array of 2D planes
  gaps←⌽≥ι¯2ₓ+ρρω #¨ ' '  ⍝ increasing cell gaps for higher ranks
  cjoin←{⍪/[α],⍶,⊂ω}
  top{(α cjoin)⌿ω}/gaps,⊂cells
}

⍝ From http://dfns.dyalog.com/s_dsp.htm

Tape ← {'∘' , α↑ω , ⊂{[α ω]}/α↓ω,'∘'}  ⍝ α-window tape
Rgt ← {⊂2↑ω , 2↓¯1↓ω , ↑⌽ω}  ⍝ tape-head moves right 1 item

⍝ From http://dfns.dyalog.com/c_enlist.htm

enlist ← {  ⍝ List α-leaves of nested array.
  α←0ₓ  ⍝ default: list 0-leaves.
  α≥≤≡ω?,ω;  ⍝ all shallow leaves: finished.
  1↓,/[⊂↑↑ω],α∇¨,ω
}

⍝ From http://dfns.dyalog.com/c_from.htm

from ← {  ⍝ Select (1↓ρω)-cells from array ω.
  ~ ≢α ≡ ≢ρω?"error";  ⍝ check index length.
  indx←α
  axes←+\0ₓ,¯1↓{↑ρρω}¨α
  {
    [indx axis]←α  ⍝ index and axis for selection.
    indx≡,⊂⍬?ω;  ⍝ skip: select all items.
    vec←⊂⍠(ιρρω ~ [axis])ω  ⍝ vector along given axis.
    sel←⊃[indx]⌷vec
    pos←axis+ιρρindx
    (pos , ιρρsel ~ pos)⍉sel  ⍝ simple selection.
  }/⌽[ω],↓⍉[indx◇axes]
}

⍝ From http://dfns.dyalog.com/n_foldl.htm

foldl ← {α ⍶⍨/ ⌽ω}  ⍝ Fold (reduce) from the left.

⍝ From http://dfns.dyalog.com/c_in.htm

in ← {  ⍝ Locations of item α in array ω.
  D←≡item←α  ⍝ (depth of) sought item
  ⍬{  ⍝ α is pick-path
    item≡ω?,⊂α;  ⍝ match: path
    D≥≡ω?⍬;  ⍝ give up
    paths←α↣,⊂ ¨ιρω  ⍝ extended paths
    ,/, paths ∇¨ ω
  }ω  ⍝ ω is searched-in array
}

⍝ From http://dfns.dyalog.com/c_list.htm

list ← {{[α ω]}/ω,'∘'}  ⍝ List from vector ω, with '∘' as null.

⍝ From http://dfns.dyalog.com/c_ltrav.htm

ltrav ← {  ⍝ List traversal.
  '∘'≡[head tail]←ω?α;  ⍝ head and tail of list, else accumulator
  α⍶head ∇ tail
}

⍝ From http://dfns.dyalog.com/s_list.htm

listLength ← 0ₓ↣({≥α} ltrav)

vectFromList ← ⍬↣({α,⊂ω} ltrav)

revl ← '∘'↣({[α ω]}⍨ ltrav)

listRmDups ← {  ⍝ remove adjacent duplicates.
  α←'∘'  ⍝ null accumulator.
  [a [b tail]]←ω  ⍝ first two items.
  b≡'∘'?revl [a α];  ⍝ b null: list expired.
  a≡b?α ∇ [b tail];  ⍝ two items match: drop first one.
  [a α] ∇ [b tail]  ⍝ accumulate first, continue.
}

⍝ From http://dfns.dyalog.com/c_match.htm

match ← {  ⍝ Wildcard match.
  [p x]←{[ω '*']}⍣ 1ₓ=≡,α ,α  ⍝ pattern and wildcard.
  v←1↓¨{x↣≡¨ω ⊂ ω}[x],p
  h←⊃v⍷¨⊂ω
  r←0ₓ,¯1↓,⊃ρ¨v
  sl←{  ⍝ array shifted left.
    a←¯1↓ρω  ⍝ leading axes.
    x←⌈/α  ⍝ maximum shift.
    p←ω,(a,x)ρ$f  ⍝ 0-padded on right.
    s←⍉aρα  ⍝ sub-array of vector rotations.
    (- 0×a , x)↓s⌽p  ⍝ shifted array.
  }  ⍝ :: r ∇ a → a
  m←⌽∨\⌽(r sl h)  ⍝ shifted mask 1 .. 1 0 .. 0
  ↑{
    [[lh lm] [und rm]]←[α ω]  ⍝ left and right hits and masks.
    [lh∧rm lm]  ⍝ right-masked left hits.
  }/↓⍉⊃⊂⁻¹¨[h m]
}

⍝ From http://dfns.dyalog.com/s_match.htm

showmatch ← {marks←[(α match ω)]⌷" ¯" ◇ ,⍠ ιρρω ω {[α◇ω]}¹ marks}

⍝ From http://dfns.dyalog.com/n_nlines.htm

nlines ← {  ⍝ Number of display lines for simple array.
  {
    ×/ω + {  ⍝ # of lines of real data +
      +/+\0ₓ⌈ω-1ₓ,¯1↓ω  ⍝ # of blank lines separating different planes
    }×\¯1↓ω  ⍝ of the array
  }¯1↓ρω  ⍝ last axis affects only width of display.
}

⍝ From http://dfns.dyalog.com/s_perv.htm

perv ← {  ⍝ Pervasion
  1ₓ=≡[α ω ω]?α ⍶ ω;
  α ∇¨ ω  ⍝ (α or) ω deeper: recursive traversal.
}

⍝ From http://dfns.dyalog.com/c_pmat.htm

pmat ← {  ⍝ Permutation matrix of ιω.
  {  ⍝ perms of ιω:
    1ₓ≥ρω?⊃,↓ω;⊃⍪/ω,↢∇¨ω↣~¨,¨ω
  }ιω  ⍝ permutations of identity perm.
}

⍝ From http://dfns.dyalog.com/c_pred.htm

pred ← {⊃⍶/¨(α#≥ιρα)⊆ω}  ⍝ Partitioned reduction.

⍝ From http://dfns.dyalog.com/c_rows.htm

rows ← {  ⍝ Operand function applied to argument rows.
  1ₓ<≡ω?∇¨ω;  ⍝ nested: item-wise application
  ⍶⍤1 ω
}

⍝ From http://dfns.dyalog.com/c_sam.htm

sam ← {  ⍝ Select and modify.
  (α ⍶ ω)←⍹ α ⍶ ω
  ω
}

⍝ From http://dfns.dyalog.com/c_saw.htm

saw ← {  ⍝ Function operand applied Simple-Array-Wise.
  2ₓ≥≡[α ω ω]?α ⍶ ω;
  1ₓ≥≡ω?α ∇¨ ⊂ω;  ⍝ ω simple: traverse α.
  2ₓ≥≡[α 1]?(α ∇)¨ω;  ⍝ α simple: traverse ω.
  α ∇¨ ω  ⍝ Both nested: traverse both.
}

⍝ From http://dfns.dyalog.com/c_mscan.htm

mscan ← {  ⍝ Minus scan.
  α←¯1  ⍝ α is axis (default last).
  +\⍠α ω×⍠α(α⊃ρω)ρ[1 ¯1]ₓ
}

⍝ From http://dfns.dyalog.com/c_dscan.htm

dscan ← {  ⍝ Divide scan
  α←¯1  ⍝ α is axis (default last).
  ×\⍠α ω*⍠α(α⊃ρω)ρ[1 ¯1]ₓ
}

⍝ From http://dfns.dyalog.com/c_ascan.htm

ascan ← {  ⍝ Associative scan.
  2ₓ>0ₓ⊥ρω?ω;  ⍝ few items: done.
  lane←⍶{⌽⍶{[(↑ω)⍶α],ω}/⌽[⊂↑ω],1↓ω}  ⍝ one lane: builds the results newest first, then reverses them.
  lane¹ ω
}

⍝ From http://dfns.dyalog.com/c_ascana.htm

ascana ← {  ⍝ Higher rank associative scan.
  α←¯1  ⍝ default last axis.
  (⍶ ascan)⍠α ω
}

⍝ From http://dfns.dyalog.com/c_select.htm

select ← {α⊃¨,¨/⊂¨¨ω}  ⍝ α-selection of items of vector ω.

⍝ From http://dfns.dyalog.com/c_shannon.htm

shannon ← {-+/(2↣⊛×⊣)¨(≢⊢ ⌸÷ₐ≢)ω}  ⍝ Shannon entropy of message ω.

⍝ From http://dfns.dyalog.com/c_subvec.htm

subvec ← {0ₓ∊ρα?$t;  ⍝ Is α a subvector of ω?
  0ₓ∊ρω?$f;  ⍝ null ω: failure.
  1↓α ∇ (≥ωι1↑α)↓ω  ⍝ otherwise, check remaining items.
}

⍝ From http://dfns.dyalog.com/c_subs.htm

subs ← {  ⍝ Vector substitution.
  [fs ts]←≢¨[fm to]←α  ⍝ old and new vectors and sizes
  1ₓ≡≡α?to@[fm=] ω;  ⍝ special case: atom subs
  0ₓ=ρρω?↑(ω≡fm)⌽[ω ⊂to];
  lead←fs↑$t  ⍝ leading mask
  fm⍷ω {  ⍝ hits mask
    ~$t∊α?ω;  ⍝ early out if no match
    ts↓,/{to,fs↓ω}¨(lead,α)⊂fm,ω
  }¹ ω  ⍝ apply to vectors
}

⍝ From http://dfns.dyalog.com/c_lcase.htm and http://dfns.dyalog.com/c_ucase.htm
letters ← ["abcdefghijklmnopqrstuvwxyzåäöàæéñøü" "ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ"]  ⍝ Lower and upper case, at matching positions.
recase ← {[from to]←α ◇ [(from,,ω)ιω]⌷to,,ω}  ⍝ Simple array ω, with each letter from α's first list replaced by its second.
lcase ← ⌽letters ↣recase  ⍝ Lower-casification.
ucase ← letters↣recase  ⍝ Upper-casification.
