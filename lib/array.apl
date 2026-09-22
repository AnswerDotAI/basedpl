⍝ Array dfns — adapted for bAsedPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/array/array.apl; Apache-2.0, see LICENSE-april

⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_contents.htm into April APL

⍝⍝ Array processing

⍝ From http://dfns.dyalog.com/n_alists.htm

alpush ← {  ⍝ Association list ⍺ prefixed with (key value) pair ⍵.
  ⍺,⍨∘⊂¨⍵  ⍝ :: list ← list ∇ key val
}

alpop ← {  ⍝ Leftmost value for key ⍵ from list ⍺
  keys vals←⍺  ⍝ keys vector and corresponding values
  indx←keys⍳⊂⍵  ⍝ index of first key ⍵ in list ⍺
  val←indx⊃vals  ⍝ value for key ⍵
  list←(⊂indx≠⍳⍴keys)/¨⍺  ⍝ reduced list
  val list  ⍝ :: val list ← list ∇ key
}

alget ← {  ⍝ Value for key ⍵ in association list ⍺.
  keys vals←⍺  ⍝ keys vector and corresponding values
  (keys⍳⊂⍵)⊃vals  ⍝ :: val ← list ∇ key
}

alset ← {  ⍝ Assoc list ⍺ with (key value) pair ⍵ replaced.
  key val←⍵  ⍝ key and new value
  {val@(⍺⍳⊂key)⊢⍵}\⍺
}

⍝ From http://dfns.dyalog.com/c_acc.htm

acc ← { ⍶{(⊂⍺ ⍶↑⍬⍴⍵),⍵}/1↓{⍵,⊂⍬⍴⍵}¯1⌽⍵ }  ⍝ Accumulating reduction.

⍝ From http://dfns.dyalog.com/c_disp.htm

disp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄ ⍺←⍬  ⍝ Boxed sketch of nested array.
  dec ctd←2↑⍺  ⍝ 1:decorated, 1:centred.
  box←{  ⍝ Recursive boxing of nested array.
    isor ⍵:format⊂⍵
    1=≡,⍵:dec open format dec open ⍵
    mat←matr 1/dec open ⍵  ⍝ matrix of opened subarrays.
    r c←×⍴mat  ⍝ non-null rows/cols.
    dec<0∊r c:c/r⌿∇ 1 open mat  ⍝ undecorated null: empty result.
    subs←aligned ∇¨mat  ⍝ aligned boxed subarrays.
    (≢⍴⍵)gaps ⍵ plane subs  ⍝ collection into single plane.
  }
  aligned←{  ⍝ Alignment and centring.
    rows cols←sepr⍴¨⍵  ⍝ subarray dimensions.
    sizes←(⌈/rows) ,⌝ ⌈⌿cols
    ctd=0:sizes↑¨⍵  ⍝ top-left alignment.
    v h←sepr⌈0.5×⊃(⍴¨⍵)-sizes
    v⊖¨h⌽¨sizes↑¨⍵  ⍝ centred aligned subarrays.
  }
  gaps←{  ⍝ Gap-separated sub-planes.
    ⍺≤2:⍵  ⍝ lowish rank: done.
    subs←(⍺-1)∇¨⍵  ⍝ sub-hyperplanes.
    width←↑⌽⍴↑subs
    fill←(⍺ width-3 0)⍴' '  ⍝ inter-plane gap.
    ⊃{⍺⍪fill⍪⍵}/1 open subs
  }
  plane←{  ⍝ Boxed rank-2 plane.
    2<⍴⍴⍺:⍺ join ⍵  ⍝ gap-separated sub-planes.
    odec←(dec shape ⍺)outer ⍵  ⍝ outer type and shape decoration.
    idec←inner ⍺  ⍝ inner type and shape decorations.
    (odec,idec)collect ⍵  ⍝ collected, formatted subarrays.
  }
  join←{  ⍝ Join of gap-separated sub-planes.
    sep←(≢⍵)÷1⌈≢⍺  ⍝ sub plane separation.
    split←(0=sep|¯1+⍳≢⍵)⊂[1]⍵
    (⊂⍤¯1⊢⍺)plane¨split  ⍝ sub-plane join.
  }
  outer←{  ⍝ Outer decoration.
    sizes←1 0{↑↓(⍉⍣⍺)⍵}¨sepr⍴¨⍵
    sides←sizes/¨¨'│─'  ⍝ vert and horiz cell sides.
    bords←dec↓¨'├┬'glue¨sides  ⍝ joined up outer borders.
    ,¨/('┌' '')⍺ bords'└┐'
  }
  inner←{  ⍝ Inner subarray decorations.
    deco←{(type ⍵),1 shape ⍵}  ⍝ type and shape decorators.
    sepr deco¨matr dec open ⍵  ⍝ decorators: tt vv hh .
  }
  collect←{  ⍝ Collected subarrays.
    lft top tt vv hh←⍺  ⍝ array and subarray decorations.
    cells←vv right 1 open tt hh lower ⍵  ⍝ cells boxed right and below.
    boxes←(dec∨0∊⍴⍵)open cells  ⍝ opened to avoid ,/⍬ problem.
    lft,top⍪⍪⌿,/boxes
  }
  right←{  ⍝ Border right each subarray.
    types←2⊥¨(⍳⍴⍵)=⊂⍴⍵
    chars←'┼┤┴┘'[1+types]
    rgt←{⍵,(-≢⍵)↑(≢⍵)1 1/'│',⍺}  ⍝ form right border.
    ((matr 1 open ⍺),¨chars)rgt¨⍵  ⍝ cells bordered right.
  }
  lower←{  ⍝ Border below each subarray.
    split←{((¯2+2⊃⍴⍵)/'─')glue ⍺}
    bot←{⍵⍪(-2⊃⍴⍵)↑⍺ split ⍵}
    (matr,¨/⍺)bot¨matr ⍵
  }
  type←{  ⍝ Type decoration char.
    dec<|≡⍵:'─'  ⍝ nested: '─'
    isor ⍵:'∇'  ⍝ ⎕or:    '∇'
    sst←{  ⍝ simple scalar type.
      0=dec×⍴⍴⍵:'─'  ⍝ undecorated or scalar ⍕⍵: char,
      (1+↑⍵∊'¯',•d)⊃'#~'
    }∘⍕  ⍝ ⍕ distinguishes type of scalar.
    0=≡⍵:sst ⍵  ⍝ simple scalar: type.
    {(1+1=⍴⍵)⊃'+'⍵}∪,sst¨dec open ⍵
  }
  shape←{
    dec≤0=⍴⍴⍵:⍺/¨'│─'  ⍝ no decoration or scalar.
    cols←(1+×¯1↑⍴⍵)⊃'⊖→'
    rsig←(1+××/¯1↓⍴⍵)⊃'⌽↓'
    rows←(3⌊⍴⍴⍵)⊃'│'rsig'⍒'
    rows cols  ⍝ shape decorators.
  }
  matr←{⊃,↓⍵}
  sepr←{+/¨1⊂⊃⍵}
  open←{(⍺⌈⍴⍵)↑⍵}
  isor←{1 ⍬≡(≡⍵)(⍴⍵)}  ⍝ is ⎕or of object?
  glue←{0=⍴⍵ : ⍵ ⋄ ⍺{⍺,⍶,⍵}/⍵}
  isor ⍵:format⊂⍵
  1=≡,⍵:format ⍵
  box ⍵  ⍝ recursive boxing of array.
}

⍝ From http://dfns.dyalog.com/c_display.htm

display ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t} ⋄  ⍝ Boxed display of array.
  box←{  ⍝ box with type and axes
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'  ⍝ vert. and horiz. lines
    top←'─⊖→'[1+¯1↑⍺],hrz
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'  ⍝ right side with corners
    lax←'│⌽↓'[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt  ⍝ fully boxed array
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}  ⍝ type and axes vector
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}  ⍝ array axis types
  open←{(1⌈⍴⍵)⍴⍵}  ⍝ exposure of null axes
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}  ⍝ removal of extra blank cols
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•d)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  line←{(1+''≡0⍴⍵)⊃' -'}
  {
    0=≡⍵:' '⍪(open format ⍵)⍪line ⍵
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open format open ⍵
    ('∊'deco ⍵)box trim format ∇¨open ⍵
  }⍵
}

⍝ From http://dfns.dyalog.com/c_displays.htm

displays ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}  ⍝ Boxed display of array.
  box←{  ⍝ Box with type and axes.
    shp w←open\⍵
    vrt hrz←(¯1+⍴w)⍴¨'│─'  ⍝ Vert. and horiz. lines.
    top←('─⊖→')[1+¯1↑⍺],hrz
    ok←(⍴shp)<⍴hrz
    top←(⍴top)↑(2↑top),(ok/shp),(2+ok×⍴shp)↓top
    bot←(↑⍺),hrz
    rgt←'┐│',vrt,'┘'  ⍝ Right side with corners.
    lax←('│⌽↓')[1+¯1↓1↓⍺],¨⊂vrt
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪w⍪bot),rgt  ⍝ Fully boxed array.
  }
  deco←{⍺←type open ⍵ ⋄ ⍺,axes ⍵}  ⍝ Type and axes vector.
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}  ⍝ Array axis types.
  open←{(1⌈⍴⍵)⍴⍵}  ⍝ Expose null axes.
  trim←{(1⊃⍵)((~1 1⍷∧⌿(2⊃⍵)=' ')/(2⊃⍵))}
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•d)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  qfmt←{(⍕0+⍴⍺)(format open ⍵)}
  {  ⍝ Recursively box arrays:
    0=≡⍵:' '⍪(format ⍵)⍪(1+' '≡↑0⍴⍵)⊃' -'
    1 ⍬≡(≡⍵)(⍴⍵):'∇' 0 0 box(,'─')(format ⍵)
    1=≡⍵:(deco ⍵)box open ⍵ qfmt ⍵  ⍝ Simple array.
    ('∊'deco ⍵)box trim ⍵ qfmt ∇¨open ⍵  ⍝ Nested array.
  }⍵
}

⍝ From http://dfns.dyalog.com/c_displayr.htm

displayr ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}  ⍝ Boxed display of array
  box←{  ⍝ box with type and axes
    vrt hrz←(¯1+⍴⍵)⍴¨'│─'  ⍝ vert. and horiz. lines
    top←(1+⍴hrz)↑(↑(1+¯1↑⍺)⌷'─⊖',⊂⍕¯1↑2⊃⍺),hrz
    bot←(⍴top)↑(↑2↓⍺),hrz
    rgt←'┐│',vrt,'┘'  ⍝ right side with corners
    lax←(↑¨(1+¯1↓3↓⍺)⌷¨(-1⌈¯1+⍴2⊃⍺)↑(⊂'│⌽'),¨⊂∘⍕¨¯1↓0,2⊃⍺),¨⊂vrt
    lax←(⊂1+⍴vrt)↑¨(lax~¨⊂' '),¨'│'  ⍝ pad and trim
    lft←⍉'┌',(⊃lax),'└'
    lft,(top⍪⍵⍪bot),rgt  ⍝ fully boxed array
  }
  deco←{⍺←type open ⍵ ⋄ (⍴⍴⍵),(⊂0+⍴⍵),⍺,axes ⍵}
  axes←{(-2⌈⍴⍴⍵)↑1+×⍴⍵}  ⍝ array axis types
  open←{(1⌈⍴⍵)⍴⍵}  ⍝ exposed null axes
  trim←{(~1 1⍷∧⌿⍵=' ')/⍵}  ⍝ removal of extra blank cols
  char←{⍬≡⍴⍵:'─' ⋄ (1+↑⍵∊'¯',•d)⊃'#~'}∘⍕
  type←{{(1+1=⍴⍵)⊃'+'⍵}∪,char¨⍵}
  {  ⍝ recursively boxed arrays:
    0=≡⍵:' '⍪(open format ⍵)⍪(1+' '=↑0⍴⍵)⊃' -'
    1 ⍬≡(≡⍵)(⍴⍵):''(0 0)'∇' 0 0 box format ⍵
    1=≡⍵:(deco ⍵)box open' ',format open ⍵
    ((⊂⍕0+≡⍵)deco ⍵)box trim' ',format ∇¨open ⍵
  }⍵
}

⍝ From http://dfns.dyalog.com/c_dist.htm

dist ← {  ⍝ Levenshtein distance.
  a←(n+1)⍴(⍴⍺)+n←⍴⍵  ⍝ first row of matrix
  f←⍵{⌊\⍵⌊(↑⍵),(¯1↓⍵)-1+⍺=⍶}
  z←a f/⌽⍺
  ↑⌽z
}

fuzzy ← {({⍵⍳⌊/⍵}(lcase ⍺)∘dist∘lcase¨⍵)⊃⍵}

⍝ From http://dfns.dyalog.com/n_dsp.htm

dsp ← { format←{t←⊃,↓⍕⍵ ⋄ (¯2↑1 1,⍴t)⍴t}  ⍝ Reduced version of disp.
  (1=≡,⍵)∨0∊⍴⍵:format ⍵
  ⍺←1 ⋄ top←'─'∘⍪⍣⍺  ⍝ top '─' bar if ⍺
  1≥⍴⍴⍵:{  ⍝ vector or scalar:
    bars←{⍪(⌊/≢¨⍺ ⍵)/'│'}/2↕⍵,0
    join←{⊃,/(⌈/≢¨⍵)↑¨⍵}
    0 ¯1↓join top¨join¨↓⍉⊃⍵ bars
  }1 ∇¨⍵  ⍝ vector: formatted items
  subs←⍺ ∇¨⍵  ⍝ higher rank: formatted items
  rs cs←+/¨1⊂⊃⍴¨subs
  dims←(mrs←⌈/rs) ,⌝ mcs←⌈/⍪⍉cs
  join←{⍺{⍺,⍶,⍵}/⍵}
  rows←(mrs/¨'│')join¨↓dims↑¨subs  ⍝ complete rows with '│'-separated items
  hzs←'┼'join mcs/¨'─'  ⍝ inter-row horizontal '─┼─' separators
  cells←{⍺⍪hzs⍪⍵}/rows  ⍝ joined rows: array of 2D planes
  gaps←(⌽⍳¯2+⍴⍴⍵)/¨' '  ⍝ increasing cell gaps for higher ranks
  cjoin←{⍪/(⊂⍺),⍶,⊂⍵}
  top{⍺ cjoin⌿⍵}/gaps,⊂cells
}

⍝ From http://dfns.dyalog.com/s_dsp.htm

Tape ← { '∘',(⍺↑⍵),⊂{⍺ ⍵}/⍺↓⍵,'∘' }  ⍝ ⍺-window tape
Rgt ← { (⊂2↑⍵),(2↓¯1↓⍵),↑⌽⍵ }  ⍝ tape-head moves right 1 item

⍝ From http://dfns.dyalog.com/c_enlist.htm

enlist ← {  ⍝ List ⍺-leaves of nested array.
  ⍺←0  ⍝ default: list 0-leaves.
  ⍺≥¯1+|≡⍵:,⍵  ⍝ all shallow leaves: finished.
  1↓,/(⊂⊂↑↑⍵),⍺ ∇¨,⍵
}

⍝ From http://dfns.dyalog.com/c_from.htm

from ← {  ⍝ Select (1↓⍴⍵)-cells from array ⍵.
  ~(≢⍺)≡≢⍴⍵:'error'  ⍝ check index length.
  indx←⍺
  axes←1++\0,¯1↓{↑⍴⍴⍵}¨⍺
  {
    indx axis←⍺  ⍝ index and axis for selection.
    indx≡,⊂⍬:⍵  ⍝ skip: select all items.
    vec←⊂[(⍳⍴⍴⍵)~axis]⍵  ⍝ vector along given axis.
    sel←⊃indx⊃¨⊂vec
    pos←(axis-1)+⍳⍴⍴indx
    (pos,(⍳⍴⍴sel)~pos)⍉sel  ⍝ simple selection.
  }/⌽(⊂⍵),↓⍉⊃indx axes
}

⍝ From http://dfns.dyalog.com/n_foldl.htm

foldl ← { ⍺ ⍶⍨/⌽⍵ }  ⍝ Fold (reduce) from the left.

⍝ From http://dfns.dyalog.com/c_in.htm

in ← {  ⍝ Locations of item ⍺ in array ⍵.
  D←|≡item←⍺  ⍝ (depth of) sought item
  ⍬{  ⍝ ⍺ is pick-path
    item≡⍵:,⊂⍺  ⍝ match: path
    D≥|≡⍵:⍬  ⍝ give up
    paths←⍺∘,∘⊂¨⍳⍴⍵  ⍝ extended paths
    ,/,paths ∇¨⍵
  }⍵  ⍝ ⍵ is searched-in array
}

⍝ From http://dfns.dyalog.com/c_list.htm

list ← { {⍺ ⍵}/⍵,'∘' }  ⍝ List from vector ⍵, with '∘' as null.

⍝ From http://dfns.dyalog.com/c_ltrav.htm

ltrav ← {  ⍝ List traversal.
  '∘'≡head tail←⍵:⍺  ⍝ head and tail of list, else accumulator
  (⍺ ⍶ head)∇ tail
}

⍝ From http://dfns.dyalog.com/s_list.htm

listLength ← 0∘({⍺+1} ltrav)

vectFromList ← ⍬∘({⍺,⊂⍵} ltrav)

revl ← '∘'∘({⍺ ⍵}⍨ ltrav)

listRmDups ← {  ⍝ remove adjacent duplicates.
  ⍺←'∘'  ⍝ null accumulator.
  (a(b tail))←⍵  ⍝ first two items.
  b≡'∘': revl a ⍺  ⍝ b null: list expired.
  a≡b:⍺ ∇ b tail  ⍝ two items match: drop first one.
  a ⍺ ∇ b tail  ⍝ accumulate first, continue.
}

⍝ From http://dfns.dyalog.com/c_match.htm

match ← {  ⍝ Wildcard match.
  p x←{⍵'*'}⍣(1=≡,⍺),⍺  ⍝ pattern and wildcard.
  v←1↓¨{(x∘≡¨⍵)⊂⍵}(⊂x),p
  h←⊃v⍷¨⊂⍵
  r←0,¯1↓,⊃⍴¨v
  sl←{  ⍝ array shifted left.
    a←¯1↓⍴⍵  ⍝ leading axes.
    x←⌈/⍺  ⍝ maximum shift.
    p←⍵,(a,x)⍴0  ⍝ 0-padded on right.
    s←⍉a⍴⍺  ⍝ sub-array of vector rotations.
    (-(0×a),x)↓s⌽p  ⍝ shifted array.
  }  ⍝ :: r ∇ a → a
  m←⌽∨\⌽r sl h  ⍝ shifted mask 1 .. 1 0 .. 0
  ↑{
    (lh lm)(und rm)←⍺ ⍵  ⍝ left and right hits and masks.
    (lh∧rm)lm  ⍝ right-masked left hits.
  }/↓⍉⊃⊂⍤¯1¨h m
}

⍝ From http://dfns.dyalog.com/s_match.htm

showmatch ← {,[⍳⍴⍴⍵]⍵,[¯0.5+⍴⍴⍵](' ¯'[1+⍺ match ⍵])}

⍝ From http://dfns.dyalog.com/n_nlines.htm

nlines ← {  ⍝ Number of display lines for simple array.
  {
    (×/⍵)+{  ⍝ # of lines of real data +
      +/+\0⌈⍵-1,¯1↓⍵  ⍝ # of blank lines separating different planes
    }×\¯1↓⍵  ⍝ of the array
  }¯1↓⍴⍵  ⍝ last axis affects only width of display.
}

⍝ From http://dfns.dyalog.com/s_perv.htm

perv ← { ⍺←⊢  ⍝ Scalar pervasion
  1=≡⍺ ⍵ ⍵:⍺ ⍶ ⍵
           ⍺ ∇¨⍵  ⍝ (⍺ or) ⍵ deeper: recursive traversal.
}

⍝ From http://dfns.dyalog.com/c_pmat.htm

pmat ← {  ⍝ Permutation matrix of ⍳⍵.
  {  ⍝ perms of ⍳⍵:
    1≥⍴⍵:⊃,↓⍵ ⋄ ⊃⍪/⍵,∘∇¨⍵∘~¨⍵
  }⍳⍵  ⍝ permutations of identity perm.
}

⍝ From http://dfns.dyalog.com/c_pred.htm

pred ← { ⊃⍶/¨(⍺/⍳⍴⍺)⊆⍵ }  ⍝ Partitioned reduction.

⍝ From http://dfns.dyalog.com/c_rows.htm

rows ← {  ⍝ Operand function applied to argument rows.
  1<|≡⍵:∇¨⍵  ⍝ nested: item-wise application
  ⍶⍤1⊢⍵
}

⍝ From http://dfns.dyalog.com/c_sam.htm

sam ← {  ⍝ Select and modify.
  ⍺←⊢  ⍝ id function for missing ⍺.
  array←⍵  ⍝ 'name' array argument.
  (⍺ ⍶ array)←⍹ ⍺ ⍶ array
  array  ⍝ return updated value.
}

⍝ From http://dfns.dyalog.com/c_saw.htm

saw ← {  ⍝ Function operand applied Simple-Array-Wise.
  ⍺←⊢  ⍝ default left arg.
  2≥|≡⍺ ⍵ ⍵:⍺ ⍶ ⍵
  1≥|≡⍵    :⍺ ∇¨⊂⍵  ⍝ ⍵ simple: traverse ⍺.
  2≥|≡⍺ 1  :⍺∘∇¨⍵  ⍝ ⍺ simple: traverse ⍵.
  ⍺ ∇¨⍵  ⍝ Both nested: traverse both.
}

⍝ From http://dfns.dyalog.com/c_mscan.htm

mscan ← {  ⍝ Minus scan.
  ⍺←≢⍴⍵  ⍝ ⍺ is axis (default last).
  +\[⍺]⍵×[⍺](⍺⊃⍴⍵)⍴1,-1
}

⍝ From http://dfns.dyalog.com/c_dscan.htm

dscan ← {  ⍝ Divide scan
  ⍺←≢⍴⍵  ⍝ ⍺ is axis (default last).
  ×\[⍺]⍵*[⍺](⍺⊃⍴⍵)⍴1,-1
}

⍝ From http://dfns.dyalog.com/c_ascan.htm

ascan ← {  ⍝ Associative scan.
  2>0⊥⍴⍵:⍵  ⍝ few items: done.
  ⌽⊃⍶{(⊂(↑⍵)⍶ ⍺),⍵}/⌽(⊂∘↑¨↓⍵),⊃1↓¨↓⍵
}

⍝ From http://dfns.dyalog.com/c_ascana.htm

ascana ← {                                   ⍝ Higher rank associative scan.
  ⍺←≢⍴⍵                                  ⍝ default last axis.
  ⊃[⍺-0.1](⍶ ascan)¨↓[⍺]⍵
}

⍝ From http://dfns.dyalog.com/c_select.htm

select ← { ⍺⊃¨,¨/⊂¨¨⍵ }  ⍝ ⍺-selection of items of vector ⍵.

⍝ From http://dfns.dyalog.com/c_shannon.htm

shannon ← { -+/(2∘⍟×⊣)¨({≢⍵}⌸÷≢)⍵ }  ⍝ Shannon entropy of message ⍵.

⍝ From http://dfns.dyalog.com/c_subvec.htm

subvec ← { 0∊⍴⍺:1  ⍝ Is ⍺ a subvector of ⍵?
  0∊⍴⍵:0  ⍝ null ⍵: failure.
  (1↓⍺)∇(⍵⍳1↑⍺)↓⍵  ⍝ otherwise, check remaining items.
}

⍝ From http://dfns.dyalog.com/c_subs.htm

subs ← {  ⍝ Vector substitution.
  fs ts←≢¨fm to←⍺  ⍝ old and new vectors and sizes
  1≡≡⍺:to@(fm∘=)⍵  ⍝ special case: simple scalar subs
  0=⍴⍴⍵:↑(⍵≡fm)⌽⍵(⊂to)
  lead←fs↑1  ⍝ leading mask
  (fm⍷⍵){  ⍝ hits mask
    ~1∊⍺:⍵  ⍝ early out if no match
    ts↓,/{to,fs↓⍵}¨(lead,⍺)⊂fm,⍵
  }⍤1⊢⍵  ⍝ apply to vectors
}

⍝ From http://dfns.dyalog.com/c_lcase.htm

lcase ← {  ⍝ Lower-casification,
  lc←'abcdefghijklmnopqrstuvwxyzåäöàæéñøü'  ⍝ (lower case alphabet)
  uc←'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ'  ⍝ (upper case alphabet)
  (⍴⍵)⍴(lc,,⍵)[(uc,,⍵)⍳⍵]  ⍝ ... of simple array.
}

⍝ From http://dfns.dyalog.com/c_ucase.htm

ucase ← {  ⍝ Upper-casification,
  lc←'abcdefghijklmnopqrstuvwxyzåäöàæéñøü'  ⍝ (lower case alphabet)
  uc←'ABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖÀÆÉÑØÜ'  ⍝ (upper case alphabet)
  (⍴⍵)⍴(uc,,⍵)[(lc,,⍵)⍳⍵]  ⍝ ... of simple array.
}
