⍝ Dyalog dfns — adapted for bAsedPL
⍝ Source: https://dfns.dyalog.com/n_contents.htm
⍝ Exported from Dyalog 20.0.53963.0; individual sources below.
⍝ Modified to count from 0, for based arrays and for bAsedPL notation.

•load "lib/array.apl"
⍝ From https://dfns.dyalog.com/c_segs.htm
segs←{(~⍵∊⍺)⊆⍵}  ⍝ Separator-delimited segments.

⍝ From https://dfns.dyalog.com/c_m91.htm
m91←{⍵>100:⍵-10 ⋄ ∇ ∇ ⍵+11}  ⍝ McCarthy's M91 function.

⍝ From https://dfns.dyalog.com/c_life.htm
life←{[1 ⍵]∨.∧3 4=¨⊂+/,¯1 0 1⊖⌝¯1 0 1⌽⌝⊂⍵}

⍝ From https://dfns.dyalog.com/c_rmcm.htm
rmcm←{  ⍝ Replace comments with blanks.
  cm←∨\(⍵='⍝')>≠\⍵='''
  •ucs(cm×32)+(~cm)×•ucs⍵
}

⍝ From https://dfns.dyalog.com/c_ripple.htm
ripple←{⍵ ⍋⍒(⍴⍵)⍴0 1}  ⍝ Perfect ripple shuffle.

⍝ From https://dfns.dyalog.com/c_birthday.htm
birthday←{  ⍝ Probability of a shared birthday among ⍵ people, with ⍺ possible dates.
  11::1-*+/(⍟⍺-⍳⍵)-⍟⍺
  11::1-×/(⍺-⍳⍵)÷⍺
  1-((!⍺)÷(!⍺-⍵))÷(⍺*⍵)
}

⍝ From https://dfns.dyalog.com/c_lsys.htm
lsys←{  ⍝ Lindenmayer L-system expansion.
  2≥|≡⍺:(⊂,⍺)∇⍵
  [fms tos]←↓⍉⊃,⍺
  ∊[(fms,⍵)⍳⍵;]⌷tos,⍵
}

⍝ From https://dfns.dyalog.com/c_rr.htm
rr←{  ⍝ Round-robin tournament.
  v←⍳n←0⌈⍵-1
  1+(⍵÷2)↑⍠¯1 m{[⍺ ⋄ ⍵]}⍤1⌽m←0,v⌽1+[n n]⍴v
}

⍝ From https://dfns.dyalog.com/c_easter.htm
easter←{  ⍝ Easter Sunday, yyyymmdd.
  G←1+19|⍵
  C←1+⌊⍵÷100
  X←¯12+⌊C×3÷4
  Z←¯5+⌊(5+8×C)÷25
  S←(⌊(5×⍵)÷4)-X+10
  E←30|(11×G)+20+Z-X
  F←E+(E=24)∨(E=25)∧G>11
  N←(30×F>23)+44-F
  N←N+7-7|S+N
  M←3+N>31
  D←N-31×N>31
  10000 100 1+.×[⍵ M D]
}

⍝ From https://dfns.dyalog.com/c_dice.htm
dice←{
  ⍵≡6 6:"Box Cars"
  ⍵≡1 1:"Snake Eyes"
  =/⍵:"Pair"
  7=+/⍵:"Seven"
  "Unlucky"
}

⍝ From https://dfns.dyalog.com/c_packN.htm
packN←{  ⍝ Null packing: shape, non-null mask, non-null items.
  cmp←{mask←,⍵≠1↑0⍴⍵ ⋄ [⍴⍵ mask mask/,⍵]}
  exp←{[shape mask items]←⍵ ⋄ shape⍴mask\items}
  ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packR.htm
packR←{  ⍝ Run-length encoding: shape, counts, items.
  cmp←{
    shape←⍴⍵ ⋄ vect←,⍵
    runs←1,≠/2↕vect
    repls←-⍨/2↕⍸runs,1
    [shape repls runs/vect]
  }
  exp←{[shape repls solos]←⍵ ⋄ shape⍴repls/solos}
  ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packU.htm
packU←{  ⍝ Unique packing: shape, unique items, zero-based indices.
  cmp←{u←∪,⍵ ⋄ [⍴⍵ u u⍳,⍵]}
  exp←{[shape u i]←⍵ ⋄ shape⍴u[i]}
  ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_date.htm
date←{  ⍝ Timestamp from day number (Meeus).
  ⍺←¯53799
  qr←{⊂⍤¯1 (0,⍺)⊤⍵}
  [Z F]←1 qr ⍵+2415020
  a←⌊(Z-1867216.25)÷36524.25
  A←Z+(Z≥⍺+2415021)×1+a-⌊a÷4
  B←A+1524
  C←⌊(B-122.1)÷365.25
  D←⌊C×365.25
  E←⌊(B-D)÷30.6001
  [dd df]←1 qr (B-D)+F-⌊30.6001×E
  mm←E-1+12×E≥14
  yyyy←C-4715+mm>2
  part←60 60 1000 qr ⌊0.5+df×86400000
  ⊃⍠0 [yyyy mm dd],part
}

⍝ From https://dfns.dyalog.com/c_days.htm
days←{  ⍝ Days since 1899-12-31 (Meeus).
  ⍺←17520902
  1<≢⍴⍵:⍺ ∇⍤0 1 ⍵
  [yy mm dd h m s ms]←7↑⍵
  D←dd+(0 60 60 1000⊥[h m s ms])÷86400000
  [Y M]←[yy mm]+¯1 12×mm≤2
  A←⌊Y÷100
  B←(⍺<0 100 100⊥[yy mm dd])×(2-A)+⌊A÷4
  ¯2416544+D+B++/⌊365.25 30.6×[Y M]+4716 1
}

⍝ From https://dfns.dyalog.com/c_draw.htm
draw←{  ⍝ Box-drawing characters over a marker.
  ⍺←'*'
  rh←(×/¯1↓⍴⍵),¯1↑1,⍴⍵
  ch←"│─┌─┐─┬││└├┘┤┴┼"
  z←,' '⍪(' ',(rh⍴⍵),' ')⍪' '
  bv←z∊⍺ ⋄ ix←⍸bv
  in←{2⊥bv[((-⍵),¯1 1,⍵)+⌝ix]}2+1⊃rh
  z[(×in)/ix]←ch[¯1+in~0]
  (⍴⍵)⍴1 1↓¯1 ¯1↓(rh+2)⍴z
}

⍝ From https://dfns.dyalog.com/c_dots.htm
dots←{  ⍝ Show indentation with white dots.
  ⍺←'·'
  kwds←":E" ":C" ":U"⍷¨⊂⍵
  ends←⊃∨/kwds,⊂⍵='}'
  spcs←∧\' '=⍵
  xdents←ends∧1,0 ¯1↓spcs
  flood←(⍪1 2){hits←⍺⍷⍵ ⋄ ~1∊hits:⍵ ⋄ ⍺ ∇ hits+⍵}spcs+2×xdents
  ⍺@{2=flood-xdents}⍵
}

⍝ From https://dfns.dyalog.com/c_logic.htm
logic←{+⌿(0 1 2 3=⌝⍵+2×⍺)×2 2 2 2⊤⍺⍶⍵}

⍝ From https://dfns.dyalog.com/c_Depth.htm
Depth←{  ⍝ Apply ⍶ at depths ⍹.
  ⍺←⊣
  depths←⌽3⍴⌽⍹
  [m l r]←depths-⍨(|≡¨[⍵ ⍺ ⍵])×depths≥0
  [max encl]←"¨⊂"⍴¨⍨l(⌈,⟜|-)r
  [el er]←(1 0=l≤r)/¨⊂encl
  ⍎'(',el,"⍺)⍶",max,er,'⍵'
}

⍝ From https://dfns.dyalog.com/c_case.htm
case←{  ⍝ Select statement.
  ~∨/b←<\⍺:⍵
  ¯1↑b:⍹ ⍵
  1 0≡b:⍶ ⍵
  (¯1↓b)⍶ ⍵
}
⍝ From https://dfns.dyalog.com/c_morse.htm
 morse←{                     ⍝ Conversion to/from Morse code.

     [P M]←{⍵~¨' '}\↓⍉⊃{       ⍝ plain-text and Morse codes.
         [['A' " .-   "] ['B' " -... "] ['C' " -.-. "] ['D' " -..  "]],⍵}{
         [['E' " .    "] ['F' " ..-. "] ['G' " --.  "] ['H' " .... "]],⍵}{
         [['I' " ..   "] ['J' " .--- "] ['K' " -.-  "] ['L' " .-.. "]],⍵}{
         [['M' " --   "] ['N' " -.   "] ['O' " ---  "] ['P' " .--. "]],⍵}{
         [['Q' " --.- "] ['R' " .-.  "] ['S' " ...  "] ['T' " -    "]],⍵}{
         [['U' " ..-  "] ['V' " ...- "] ['W' " .--  "] ['X' " -..- "]],⍵}{
         [['Y' " -.-- "] ['Z' " --.. "]],⍵}{

         [['0' " ----- "] ['1' " .---- "] ['2' " ..--- "] ['3' " ...-- "]],⍵}{
         [['4' " ....- "] ['5' " ..... "] ['6' " -.... "] ['7' " --... "]],⍵}{
         [['8' " ---.. "] ['9' " ----. "]],⍵}{

         [['.' " .-.-.- "] [',' " --..-- "] [':' " ---... "]],⍵}{
         [['?' " ..--.. "] [''' " .----."] ['-' " -....- "]],⍵}{
         [['/' " -..-.  "] ['(' " -.--.  "] [')' " -.--.- "]],⍵}{
         [['"' " .-..-. "] ['@' " .--.-. "] ['=' " -...-  "]],⍵}{

         ⍵}⊂' ' " / "        ⍝ blank / inter-word separator.

     1=|≡,⍵:M[P⍳⍵∩P]          ⍝ plain text to Morse.
     2=|≡,⍵:P[M⍳⍵∩M]          ⍝ Morse to plain text.
 }

⍝ From https://dfns.dyalog.com/c_base64.htm
base64←{  ⍝ Base64 encoding/decoding of octet vectors.
  chars←"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  bits←{,⍉(⍺⍴2)⊤⍵}
  part←{((⍴⍵)⍴⍺↑1)⊂⍵}
  ' '=↑0⍴,⍵:2⊸⊥⍤(8⊸↑)¨ 8 part {(-8|⍴⍵)↓⍵} 6 bits {(⍵≠64)/⍵} chars⍳⍵
  four←{
    8=⍴⍵:"=="∇⍵,0 0 0 0
    16=⍴⍵:'='∇⍵,0 0
    chars.[2⊸⊥¨ 6 part ⍵;],⍺
  }
  cats←{"",/⍵}
  cats ""⊸four¨ 24 part 8 bits ⍵
}

⍝ From https://dfns.dyalog.com/c_packB.htm
packB←{  ⍝ Unique indices packed as run lengths and bits.
  cmp←{
    u←∪,⍵
    i←u⍳,⍵ ⋄ w←⌈2⍟⍴u
    b←1,≠/2↕i
    j←0∨,(w⍴2)⊤b/i
    [(⍴⍴⍵),(⍴⍵),u b j]
  }
  exp←{
    [q b j]←⍵ ⋄ s←↑q
    r←s⍴1↓q ⋄ u←(s+1)↓q
    w←⌈2⍟⍴u
    e←-⍨/2↕⍸b,1
    i←2⊥w{(⍺,⌊(⍴⍵)÷⍺)⍴⍵}j
    r⍴u[e/i]
  }
  ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packX.htm
packX←{  ⍝ Text packed through pairs of unique items.
  cmp←{
    u←∪,⍵
    p←↓{((⌈0.5×⍴⍵),2)⍴⍵},⍵
    j←,u,⌝u
    b←j∊p ⋄ g←b/j
    i←g⍳p
    w←⌈2⍟⍴g
    q←0∨b,,(w⍴2)⊤i
    [u (⍴⍴⍵),⍴⍵ q]
  }
  unc←{
    [u d a]←⍵
    s←↑d ⋄ r←s⍴1↓d
    j←,u,⌝u
    p←((⍴j)↑a)/j
    q←(⍴j)↓a
    w←⌈2⍟⍴p
    i←2⊥w{(⍺,⌊(⍴⍵)÷⍺)⍴⍵}q
    r⍴∊p[i]
  }
  ⍺←1 ⋄ ⍺=1:cmp ⍵ ⋄ unc ⍵
}

⍝ From https://dfns.dyalog.com/c_packQ.htm
packQ←{  ⍝ Frequency-ranked uniques with variable-width keys.
  key←,/{↓(⍳⍵)>⌝⍳⍵}¨1+⍳23
  cmp←{
    dt←,⍵
    u←{⍵{⍺[⍒⍵]}{+/⍵=dt}¨⍵}∪dt
    k←1,¨key ⋄ d←∊k[u⍳dt]
    [u (⍴⍴⍵),⍴⍵ d]
  }
  unc←{
    [u q d]←⍵
    s←(↑q)⍴1↓q
    p←d≤¯1↓0,d
    s⍴u[key⍳p⊆d]
  }
  ⍺←1 ⋄ ⍺=1:cmp ⍵ ⋄ unc ⍵
}

⍝ From https://dfns.dyalog.com/c_packS.htm
packS←{  ⍝ Shannon-Fano packing.
  cmp←{
    u←∪,⍵ ⋄ b←u{+/⍺=⍵}¨⊂,⍵ ⋄ i←⍒b ⋄ uv←u[i] ⋄ b←b[i]
    bv←{(⍵≠2)⊆⍵}∊⍬{
      1=≢⍵:2,⍺ ⋄ a←(⊂⍺),¨0 1
      2=⍴⍵:a ∇¨ ⍵
      n←{1⌈+/0=((+/⍵)<2×+\⍵)}⍵ ⋄ a ∇¨ [n↑⍵ n↓⍵]
    }b
    z←(⍴,⍵)⍴0⌷bv ⋄ q←,⍵
    {0}(1↓uv){((⍺=q)/z)←⊂⍵}¨1↓bv:
    [⍴⍵ uv ∊⍴¨bv 0∨(∊bv),∊z]
  }
  exp←{
    [r uv lv d]←⍵
    bv←(lv/1+⍳⍴lv)⊆(+/lv)↑d
    r⍴""{
      0∊⍴⍵:⍺
      i←0{q←⍺⊃bv ⋄ q≡(⍴q)↑⍵:⍺ ⋄ (⍺+1)∇⍵}⍵
      (⍺,i⊃uv)∇(⍴i⊃bv)↓⍵
    }(+/lv)↓d
  }
  ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_cmat.htm
cmat←{  ⍝ ⍺-combinations of ⍳⍵: include the first item, then omit it.
  ⍺=0:1 0⍴0
  ⍺>⍵:[0 ⍺]⍴0
  ⍺=⍵:[1 ⍺]⍴⍳⍵
  (0,1+(⍺-1)∇⍵-1)⍪1+⍺∇⍵-1
}

⍝ From https://dfns.dyalog.com/c_mayan.htm
mayan←{  ⍝ Mayan numbers; left 0 uses base 20, left 1 uses the calendar base.
  ⍺←1
  [snail dot bar]←"_@/" '⍟' '⌹'
  n←1+⌈18⍟1⌈⍵
  base←20-2⌽n↑2×⍺
  ⍪' '⍪⟜{
    ⍵=0:1 5⍴' ',snail,' '
    [comp part]←0 5⊤⍵
    top←dot{
      ⍵=0:0 5⍴""
      ⍵=1:0 0 1 0 0\⍺
      ⍵=2:0 1 0 1 0\⍺
      ⍵=3:0 1 1 1 0\⍺
      ⍵=4:1 1 1 1 0\⍺
    }part
    top⍪[comp 5]⍴bar
  }¨{(-1⌈+/∨\⍵≠0)↑⍵}base⊤⍵
}

⍝ From https://dfns.dyalog.com/c_mac.htm
mac←{  ⍝ Macro expansion over linked token lists.
  mexp←{
    [dd [a [b uu]]]←⍵
    a≡'\':⍺ ∇ [[dd b] uu]
    b≡'=':⍺ ∇ defn ⍵
    a∊↑⍺:⍺ ∇ dref ⍵
    a≡'(':⍺ ∇ ⍺ ∇ [dd [b uu]]
    a≡')':[dd [b uu]]
    ⍺ ∇ [[dd a] [b uu]]
  }
  defn←{
    [dd [name [eq uu]]]←⍵
    [val uuu]←⍺ body ['(' uu]
    (⍺,⍨⟜⊂¨[name val])⍶[dd uuu]
  }
  body←{
    [dd [a [b uu]]]←⍵
    a∊↑⍺:⍺ ∇ dref [dd [a [b uu]]]
    a∊" )",(•ucs 13):[dd [b uu]]
    a≡'\':⍺ ∇ [[dd b] uu]
    a≡'(':⍺ ∇ ⍺ mexp [dd [b uu]]
    ⍺ ∇ [[dd a] [b uu]]
  }
  dref←{
    [names vals]←⍺
    [dd [name uu]]←⍵
    val←(names⍳name)⊃vals
    [ddd n]←dd number 1
    ⍺ ⍶ [ddd (val (n copy) uu)]
  }
  number←{
    [dd d]←⍺
    ~d∊•d:[⍺ 0]
    [ddd n]←dd ∇ ⍵×10
    [ddd n+⍵×•d⍳d]
  }
  copy←{
    ⍺≡'(':⍵
    [vv v]←⍺
    v≡'/':vv ∇ list [⍶ 1]/⍵
    vv ∇ [v ⍵]
  }
  list←{[⍺ ⍵]}/
  1↓¯2↓∊["" ⍬]mexp['(' (list ⍵,')')]
}

⍝ From https://dfns.dyalog.com/c_baby.htm
baby←{  ⍝ Manchester Small Scale Experimental Machine.
  fetch←{,(yshift ⍵)⌿⍺}
  store←{⍶@(1⍳⍨yshift⍹)⊢⍵}
  decode←{((⍳≢⍵)∊0 5 13 16)⊂⍵}
  yshift←(0=⍳≢⍵)⊸{~1∊⍵:⍺ ⋄ (¯1⌽⍺) ∇ dec ⍵}
  addsub←{c←⍺ ⍶ ⍵ ⋄ ~1∊c:⍺ ⍹ ⍵ ⋄ (⍺≠⍵) ∇ 0,¯1↓c}
  add←∧addsub∨
  sub←<addsub>
  inc←{⍵≠¯1↓1,∧\⍵}
  dec←{⍵≠¯1↓1,∧\~⍵}
  neg←{⍵≠¯1↓0,∨\⍵}
  ⍺←0∧2↑↓⍵
  0{
    [A CI_ M]←⍵
    CI←⍺ add inc CI_
    [S _ PI _]←decode M fetch CI
    0 0 0≡PI:0 ∇ [A (M fetch S) M]
    1 0 0≡PI:(M fetch S) ∇ [A CI M]
    0 1 0≡PI:0 ∇ [(neg M fetch S) CI M]
    1 1 0≡PI:0 ∇ [A CI (A store S)M]
    0 0 1≡PI:0 ∇ [(A sub M fetch S) CI M]
    1 0 1≡PI:0 ∇ [(A sub M fetch S) CI M]
    0 1 1≡PI:({0}\⌽A) ∇ [A CI M]
    1 1 1≡PI:M
  }⍺,⊂⍵
}

⍝ From https://dfns.dyalog.com/c_quzzle.htm
quzzle←{  ⍝ Shortest sliding-tile paths to the three other corners.
  extend←{
    (≢Fin)=p←1⍳⍨(Fin=0)∧NPath=⌊/(Fin=0)/NPath:0
    (≢Target)=+/NPath.(p)≥(Fin>0)/NPath:0
    n←↑¯1↑p⊃Path
    ∇ delpath p , ,/ p next¨ ,/ n moves¨ Tiles
  }
  moves←{
    ⍵,¨((,1 ¯1×⌝1,1+1⊃Shape)((,'_'⍪'∣',Shape⍴⍺⌷State){∧/((⍵=⍺⌽⍶)/⍶)∊⍵,' '})¨⍵)/"←↑→↓"
  }
  next←{
    ' '=t←↑⍵:¯1
    y←(↑¯1↑⍺⊃Path)⌷State
    x←' '@(=⟜t)y
    (((("←↑→↓"⍳1⊃⍵)⌷,1 ¯1×⌝1,1⊃Shape)⌽,t=y)/x)←t
    (≢State)=n←1⍳⍨State∧.=x:¯1⊣(addnode x)addpath[⍺ ⍵]
    ⍺∊i←⍸n∊¨Path:¯1
    0=≢i:¯1⊣(n addpath [⍺ ⍵])
    NPath.(⍺)≥¯1+⌊/Path.[i;]⍳¨n:¯1
    (n addpath [⍺ ⍵])modpath¨i
  }
  addnode←{
    (⍬⊃State)←State⍪⍵
    i←⍵.[[Target;]⌷(i-1),(≢⍵)-(i←1⊃Shape),1;]⍳↑Tiles
    ENode,←(1+i)×i<≢Target
    NMoves,←⌊/⍬
    ¯1+≢ENode
  }
  addpath←{
    [p m]←⍵ ⋄ e←⍺⊃ENode ⋄ n←≢p⊃Path
    (e>0)∧(e∊Fin)∧n≥(Fin⍳e)⊃NPath,0:¯1
    Path,←⊂(p⊃Path),⍺
    Moves,←⊂(p⊃Moves)⍪m
    Fin,←⍺⊃ENode
    NPath,←n
    NMoves.(⍺)⌊←n
    ¯1+≢Path
  }
  modpath←{
    ⍺=¯1:¯1
    (i←1+x⍳↑¯1↑⍺⊃Path)=≢x←⍵⊃Path:⍺
    (⍵⊃Path)←(⍺⊃Path),i↓x
    (⍵⊃Moves)←(⍺⊃Moves)⍪i↓⍵⊃Moves
    (⍵⊃NPath)←¯1+≢⍵⊃Path
    ¯1
  }
  delpath←{
    ~0∊i←~(⍳≢Path)∊⍵:0
    (⍬⊃Path)←i/Path
    (⍬⊃Moves)←i/Moves
    (⍬⊃NPath)←i/NPath
    (⍬⊃Fin)←i/Fin
    0
  }
  ⍺←⍳3 ⋄ Target←,⍺
  Tiles←(∪,⍵)~' ' ⋄ Shape←⍴⍵
  Path←,⊂,0 ⋄ Moves←,⊂1 2⍴' ' ⋄ NPath←,0 ⋄ Fin←,0
  State←(1,≢,⍵)⍴⍵ ⋄ NMoves←,0 ⋄ ENode←,0
  x←extend 0
  ∧/Fin=0:"There are no solutions"
  i←∨/{i\n=⌊/n←(i←Fin=⍵)/NPath}¨1+⍳≢Target
  x←"Top-right" "Bottom-left" "Bottom-right"[Target.[¯1+i/Fin;]]
  x←[x ⋄ (⊂"in "),¨({1↓0⍕⍵}¨i/NPath),¨⊂" moves"]
  x⍪i/Moves
}

⍝ From https://dfns.dyalog.com/c_refmt.htm
refmt←{  ⍝ Indent code and align comments.
  ⍺←4 ⋄ [dent csep]←⍺
  unqt←{~≠\⍵='''}
  [code coms]←↓⍉⊃{
    umsk←unqt ⍵ ⋄ cmsk←∨\umsk∧⍵='⍝'
    [code coms]←(0 1=⊂cmsk)/¨⊂⍵
    rlb←{(∨\' '≠⍵)/⍵}
    clean←⌽ rlb ⌽ rlb code
    [clean coms]
  }¨↓⊃⍵
  dents←{
    toks←(unqt ⍵){⍺\⍺/⍵}⍵
    dvec←-⌿0 1⌽+\"{}"=⌝toks
    lmask←toks='·'
    (dent×lmask/dvec)/¨' '
  },/'·',¨code
  scoms←(⊂csep⍴' '),¨coms
  icode←↓⊃dents,¨code
  rtb←⌽⍤{(∨\⍵≠' ')/⍵}⍤⌽
  qnr←icode,⟜rtb¨scoms
  ⊃⍣(1=≡⍵)⊢qnr
}

⍝ From https://dfns.dyalog.com/c_box.htm
box←{  ⍝ Frame a text array, with optional internal row and column borders.
  ⍺←⍬ ⍬ 0 ⋄ ar←{⍵,(≢⍵)↓⍬ ⍬ 0}{2>≡⍵:,⊂,⍵ ⋄ ⍵}⍺
  ch←{⍵:"++++++++-|+" ⋄ "┌┐└┘┬┤├┴─│┼"}1=2⊃ar
  rh←(×/¯1↓⍴⍵),¯1↑1,⍴⍵ ⋄ z←rh⍴⍵
  0∊⍴∊2↑ar:{q←ch.(8)⍪(ch.(9),⍵,9⊃ch)⍪8⊃ch ⋄ q.[0 ¯1;0 ¯1]←2 2⍴ch ⋄ q}z
  [r c]←rh{∪⍺{(⍵∊⍳⍺+1)/⍵}⍵,(~¯1∊⍵)/0,⍺}¨2↑ar
  [rw cl]←rh{{⍵[⍋⍵]}⍵∪0,⍺}¨[r c]
  (~(0,1⊃rh)∊c){
    (↑⍺)↓⍠1(-1⊃⍺)↓⍠1⍵.[∞;⍋(1+⍳1⊃rh),cl]
  }(~(0,0⊃rh)∊r){
    (↑⍺)↓⍠0(-1⊃⍺)↓⍠0⍵.[⍋(1+⍳0⊃rh),rw;]
  }{
    [h w]←(≢rw),≢cl ⋄ q←[h w]⍴10⊃ch
    hz←(h,1⊃rh)⍴8⊃ch
    vr←(rh.(0),w)⍴9⊃ch
    ∨/0∊¨⍴¨[rw cl]:(⍵⍪hz),vr⍪q
    q.(0)←4⊃ch ⋄ q.(∞ ¯1)←5⊃ch ⋄ q.(∞ 0)←6⊃ch ⋄ q.(¯1)←7⊃ch
    q.[0 ¯1;0 ¯1]←2 2⍴ch ⋄ (⍵⍪hz),vr⍪q
  }z
}

⍝ From https://dfns.dyalog.com/c_sudoku.htm
sudoku←{  ⍝ All solutions, with square or ⍺-shaped groups.
  wid←≢⍵ ⋄ ⍺←wid*0.5 ⋄ grp←2⍴⍺ ⋄ set←1+⍳wid
  inx←↓(,wid⊥¨(⊂grp)×⍳⌽grp)+⌝,wid⊥¨⍳grp
  inx,←{(↓⍵),↓⍉⍵}(⍴⍵)⍴⍳wid*2
  bas←(≢,⍵)⍴⊂set
  ↑(,⍵)⊸{q←⍺[⍵] ⋄ ((q=0)/q)←⊂set~,/q ⋄ {0}bas.[⍵;]∩←,¨q}¨inx:
  ∆sqz←{
    0=≢⍵:⍺ ⋄ i←↑⍵ ⋄ q←⍺
    b←1<≢¨⍺[i] ⋄ ~∨/b:⍺ ∇ 1↓⍵
    q[b/i]←{c←⍵⊃q ⋄ z←c~,/q[i~⍵] ⋄ 1≠≢z:c ⋄ z}¨b/i
    r←{
      s←≢¨⍵ ⋄ 2>+/s=2:s ⋄ ∧/s∊1 2:s
      k←(s=2){↑(⍸⍺)~⍵⍳∪⍺/⍵}⍵ ⋄ k=0:s
      c←~⍵∊⊂k⊃⍵ ⋄ q[c/i]←(c/⍵)~¨⊂k⊃⍵ ⋄ ≢¨q[i]
    }q[i]
    0∊r:⍬ ⋄ (∧/r=1)≥1∊r:q ∇ 1↓⍵
    j←(r>1)/i ⋄ q[j]←(q[j])~¨⊂,/q[i~j]
    q ∇ 1↓⍵
  }
  ∆chk←{m←⍵ ∆sqz inx ⋄ 0=≢m:⍬ ⋄ m≢⍵:∇ m ⋄ ⍵}
  ∆nxt←{
    0=≢⍵:⍵
    r←≢¨⍵ ⋄ ∧/r=1:⊂⍵
    j←,/{⍵[⍒{+/1=r[⍵]}¨⍵]}wid↑inx
    i←(r.[j;]⍳⌊/r~1)⊃j
    ⍵⊸{m←⍺ ⋄ (i⊃m)←,⍵ ⋄ m}¨i⊃⍵
  }
  ⍬{
    0=≢⍵:⍺
    m←∆chk↑⍵ ⋄ 0=≢m:⍺ ∇ 1↓⍵
    ∨/1<≢¨m:⍺ ∇ 1↓⍵,(∆nxt m)
    ∨/(+/set)≠{+/,/m[⍵]}¨inx:⍺ ∇ 1↓⍵
    (⍺,⊂(2⍴wid)⍴,/m) ∇ 1↓⍵
  }{0=≢⍵:⍵ ⋄ ⊂⍵}(bas ∆sqz inx)
}

⍝ From https://dfns.dyalog.com/c_kt.htm
kt←{  ⍝ Knight's tours; ⍺ limits the number of solutions.
  ⍺←1 ⋄ sreq←⍺ ⋄ nsqs←×/⍵
  kdef←,0 1⌽⌝1 ¯1,⌝2 ¯2
  net←(⊂,⍳⍵)∩¨↓(⍳⍵)+⌝kdef
  ⍳⟜(⍳⍵)¨⍬{
    sreq=≢⍵:⍵
    path←⍶,⊂⍺
    nsqs=≢path:⍵,⊂path
    nxt←⍺⊃⍹
    0=≢nxt:⍵
    net←⍹~¨⊂⊂⍺
    ord←nxt[⍒≢¨net[nxt]]
    ⍵(path ⍢ net)/ord
  }net/(⌽,⍳⍵),⊂0⍴⊂⍬
}

⍝ From https://dfns.dyalog.com/c_queens.htm
queens←{  ⍝ N-queens solutions up to rotation and reflection.
  search←{
    (⊂⍬)∊⍵:0⍴⊂⍬
    0=≢⍵:rmdups ⍺
    [hd tl]←[↑⍵ 1↓⍵]
    next←⍺⊸,¨hd
    rems←hd free¨ ⊂tl
    ,/ next ∇¨ rems
  }
  cvex←(1+⍳⍵)×⊂¯1 0 1
  free←{⍵~¨⍺+(≢⍵)↑cvex}
  rmdups←{
    rots←⍒⍣(⍳4)
    refs←⍋⍣0 1
    best←{(↑⍋⊃⍵)⊃⍵}
    all8←,/↓¨refs¨↓rots⍵
    (⍵ ≡ best all8)⊃[⍬ ,⊂⍵]
  }
  fmt←{
    chars←"·⍟" (⊃⍵)=⌝⍳⍺
    expd←1↓,⊃⍺⍴⊂0 1
    ⊃¨↓↓expd\chars
  }
  squares←(⊂⍳⌈⍵÷2),1↓⍵⍴⊂⍳⍵
  ⍵ fmt ⍬ search squares
}

⍝ From https://dfns.dyalog.com/c_ary.htm
ary←{  ⍝ Radix representation, including recurring fractional digits.
  ⍵=0:"0"
  base←|⍺
  [sign abs]←[×⍵ |⍵]
  exp←⌈base⍟abs
  man←abs×base*-exp
  [p q]←⌊{[⍵ 1]÷1∨⍵}man
  digs←{
    ⍵=0:[(↑↓[0 q]⊤⍺) ⍬ 1]
    lim<≢⍺:[(rnd↑↓[0 q]⊤⍺) ⍬ 0]
    ⍵∊⍺:(⍺ rep ⍵),1
    (⍺,⍵) ∇ base×q|⍵
  }
  ofmt←{
    [sig exp fix rep rat]←⍵
    fmt←{(•d,•a)[⍵]}
    [lft pad]←0⌈1 ¯1×exp
    zro←{⍵,(≢⍵)↓0}
    neg←(sig<0)/'¯'
    nnn←fmt zro lft↑fix,lft⍴rep
    fff←fmt(pad/0),lft↓fix⌊base-1
    rrr←fmt(0⌈exp-≢fix)⌽rep
    fffrrr←fff opt rrr
    dot←(""≡fffrrr)↓'.'
    etc←(×≢rep)/"..."
    cut←rat↓'?'
    neg,nnn,dot,fffrrr,etc,cut
  }⍣(⍺>0)
  lim←⌈base⍟÷1e¯14⌈2*¯53
  rnd←{(¯2↓⍵),+⟜(5⊸≤)/¯2↑⍵}
  rep←{(0 1=⊂∨\⍵=⍺)/¨1↑↓[0 q]⊤⍺}
  opt←{(-+/∧\=⌿⊃⌽¨[⍺ ⍵])↓⍺,⍵,⍵}
  q=1:ofmt [sign exp+1 ,1 ⍬ 1]
  ofmt [sign exp],(⍬ digs p×base)
}

⍝ From https://dfns.dyalog.com/c_pack4.htm
pack4←{  ⍝ Quad-tree packing; ⍺=0 expands, ⍺>1 allows lossy packing.
  cmp←{
    uniq←∪,⍵
    rep←{⍵{⍵/⍺-¯1⌽~⍵}~1 1 0⍷⍵<0}⍣≡
    (⍴⍵) {[uniq ⍺ ⍵]} rep ⍺ {
      ~0∊⍵∊⊂↑⍵:↑⍵
      ⍺≥≢∪,⍵:maxx+/(⍳≢uniq)=⌝,⍵
      ax←maxx⍴⍵
      size←ax⊃⍴⍵
      mask←1,1↓(⍳size)=⌊size÷2
      ¯1,,/(⍺ ∇¨ mask⊂⍠ax ⍵)
    } uniq⍳⍵
  }
  exp←{
    [uniq shape stream]←⍵
    split←{{[⌊⍵ ⌈⍵]}⍵÷1+(maxk ⍵)}
    maxk←{(⍳≢⍵)=(maxx ⍵)}
    exp←{(1+|0⌊1+⍵⌊0)/¯1⌈⍵}
    uniq ↑shape{
      [head tail]←⍵
      0≤head:[⍺⍴head tail]
      [s0 s1]←split ⍺
      [sub0 rem0]←s0 ∇ tail
      [sub1 rem1]←s1 ∇ rem0
      subs←sub0,⍠(maxx ⍺) sub1
      [subs rem1]
    }{[⍺ ⍵]}/(exp stream)
  }
  maxx←{⍵⍳⌈/⍵}
  ⍺←1 ⋄ ⍺>0:⍺ cmp ⍵ ⋄ exp ⍵
}

⍝ From https://dfns.dyalog.com/c_von.htm
von←{  ⍝ Capitalise names, preserving prefixes and user-supplied exceptions.
  ⍺←""
  [pre not]←{0=≢⍵:"" "" ⋄ 3>≡⍵:[⍵ ""] ⋄ ⍵}⍺
  tx←¯1•c,⍵,' '
  q←"ab af al an and ap as av da de dei della der di"
  q,←" el ja la le och of the und van ver von zu y"
  q←{' ',¨((⍵≠' ')⊆⍵),¨' '}q
  not,←" al-" " c/o" " d'" " de'" " el-" " l'" " s'" " t'"
  ∆pre←{(-≢⍺){⍵∨⍺⌽⍵}⍺⍷⍵}
  ∆not←{∨⌿⊃(-1+⍳¯1+≢⍺)⌽¨⊂⍺⍷⍵}
  ∆fix←{
    0=≢⍺:0
    q←¯1•c{2>≡⍵:,⊂,⍵ ⋄ ⍵}⍺
    1↓∊∨/q⍶¨⊂' ',⍵
  }
  b←(¯1↓∊∨/q⍷¨⊂' ',tx)<1,>/⌽⍤1⊢2↕~tx∊" -:;.,!?'""/)("
  b∨←pre(∆pre ∆fix)tx
  b>←not(∆not ∆fix)tx
  ¯1↓(1⊸•c)@{b}tx
}

⍝ From https://dfns.dyalog.com/c_packH.htm
packH←{                      ⍝ Huffman packing.

    compress←{                          ⍝ compression.
        shape←⍴⍵ ⋄ vect←,⍵              ⍝ shape and items of array.
        uniq←∪vect                      ⍝ unique items.
        freq←+/uniq=⌝vect              ⍝ frequency of occurrence.

        tree←{                          ⍝ Huffman tree.
            1=⍴⍵:↑⌽↑⍵                   ⍝ list exhausted: done
            nxt←⍵[2↑⍋↑¨⍵]                ⍝ next two lowest frequencies,
            [freqs items]←↓⍉⊃nxt        ⍝ and corresponding items.
            ∇(⍵~nxt),⊂(+/freqs),⊂items  ⍝ collect 2 most infrequent items.
        }↓⍉⊃[freq uniq]                 ⍝ tree from frequency-item pairs.

        [items csegs]←↓⍉⊃⍬{             ⍝ Dictionary: items ←→ code-segments.
            ⍬≡⍴⍵:⊂[⍵ ⍺]                 ⍝ all done: item and binary code.
            ,/(⍺⊸,¨0 1)∇¨⍵              ⍝ extended codes for sub-trees.
        }tree                           ⍝ from frequency tree.

        bits←∊csegs[items⍳vect]          ⍝ bit string of tree indices.

        [leaves depths]←↓⍉⊃tree{        ⍝ tree leaves and depths.
            ⍬≡⍴⍺:⊂[⍺ ⍵]                 ⍝ leaf: leaf and depth.
            ,/ ⍺ ∇¨ 1+⍵                 ⍝ branch: traverse deeper sub-branches.
        }0                              ⍝ starting at depth 0 for the root.

        [shape leaves depths bits]      ⍝ compressed structure.
    }

    expand←{                            ⍝ expansion.
        [shape leaves depths bits]←⍵    ⍝ compressed structure.

        tree←1⊃↑{                       ⍝ reconstituted Huffman tree.
            (↑⍺)≠↑↑⍵:(⊂⍺),⍵             ⍝ distinct adjacent depths: continue.
            [(↑⍺)-1 [1⊃⍺ 1⊃↑⍵]] ∇ 1↓⍵    ⍝ identical   ..      ..  : coalesce.
        }/(↓⍉⊃[depths leaves]),¯1       ⍝ depths, leaves with trailing dummy.

        picks←⍬{                        ⍝ tree index vectors.
            ⍬≡⍴⍵:⊂⍺                     ⍝ leaf: done.
            ,/(⍺⊸,¨0 1)∇¨⍵              ⍝ visit each branch,
        }tree                           ⍝ ··· of the tree.

        maxd←|≡tree                     ⍝ max depth.

        dict←,/{                       ⍝ lookup dictionary.
            item←tree⊃/⌽⍵               ⍝ next tree leaf.
            indl←≢⍵                   ⍝ trailing index length.
            (2*maxd-indl)⍴⊂[indl item]  ⍝ extended pick vectors.
        }¨picks                         ⍝ ··· for each index vector.

        iwin←⍳maxd                      ⍝ input index window.
        ibuff←bits,maxd⍴0               ⍝ padded input buffer.

        shape⍴(,shape⍴leaves){          ⍝ restore original shape.
            [ix ox]←⍵                   ⍝ input and output indices.
            ox=≢⍺:⍺                     ⍝ end of output, done
            nxt←2⊥ibuff[ix+iwin]        ⍝ next dictionary index.
            [skip item]←nxt⊃dict        ⍝ number of bits and output item.
            (item@ox⊢⍺) ∇ ⍵+[skip 1]    ⍝ traverse input and output buffers.
        }0 0                              ⍝ initial input/output buffer indices.
    }

    ⍺←1 ⋄ ⍺:compress ⍵ ⋄ expand ⍵       ⍝ compress or expand.
}

⍝ From https://dfns.dyalog.com/c_lisp.htm
 lisp←{             ⍝ Evaluator for a subset of Scheme.

    ⍝ Parser functions take as arguments:
    ⍝   ⍺ ⍝ the input string
    ⍝   ⍵ ⍝ the current parse position in ⍺
    ⍝ and return a triplet of:
    ⍝   0 position AST      ⍝ on success
    ⍝   1 position message  ⍝ on failure

     parse←{
         ⍵≥≢⍺:[1 ⍵ "unexpected eof"]
         ' '≡⍺.(⍵):⍺ ∇ ⍵+1
         '('≡⍺.(⍵):⍺ parseList ⍵+1
         ⍺.(⍵)∊•d:⍺ parseNum ⍵
         '''≡⍺.(⍵):⍺ parseQuote ⍵
         ⍺ parseAtom ⍵
     }

     parseList←{ ⍝ parse the inside of a list and consume the ')'
         ⍵≥≢⍺:[1 ⍵ "unexpected eof"]
         ')'≡⍺.(⍵):[0 ⍵+1 ⍬]
         [e p h]←⍺ parse ⍵
         e≠0:[e p h]
         [e p t]←⍺ ∇ p
         e≠0:[e p t]
         [0 p (⊂h),t]
     }

     parseNum←{
         l←20 ⍝ 1 + the maximum length of a numeric literal
         n←+/∧\•d∊⍨l↑⍵↓⍺ ⍝ actual length of the literal
         n≡l:[1 ⍵ "numeric literal too long"]
         [0 ⍵+n ⍎n↑⍵↓⍺]
     }

     parseAtom←{
         la←20 ⍝ 1 + the maximum length of an atom
         na←•d,"() '" ⍝ non-atom characters
         l←+/∧\~na∊⍨la↑⍵↓⍺
         l≡la:[1 ⍵ "atom too long"]
         [0 ⍵+l l↑⍵↓⍺]
     }

     parseQuote←{
         [e p r]←⍺ parse ⍵+1
         e≠0:[e p r]
         [e p ["quote" r]]
     }

    ⍝ Evaluator

     isAtom←{(' '≡↑0⍴⍵)∧1=⍴⍴⍵}
     isNum←{(0≡↑0⍴⍵)∧0=⍴⍴⍵}

     eval←{ ⍝ ⍺: environment, ⍵: expression
         isNum ⍵:⍵
         isAtom ⍵:[⍺.(∞ 0)⍳⊂⍵ 1]⌷⍺
         0≡≢⍵:⍬
         "quote"≡↑⍵:1⌷⍵
         (1↑⍵)∊,¨'\' "lambda":["closure" ⍺],1↓⍵
         "cond"≡↑⍵:⍺ evcond 1↓⍵
         (⍺ ∇ ↑⍵) apply (⊂⍺)∇¨1↓⍵
     }

     apply←{ ⍝ ⍺: procedure, ⍵: arguments
         "closure"≡↑⍺:((⍉[⍺.(2) ⋄ ⍵])⍪1⌷⍺) eval 3⌷⍺
         ⍎(↑⍺),'⍵'
     }

     evcond←{ ⍝ ⍺: environment, ⍵: clauses
         0≡≢⍵:⍬
         "else"≡↑↑⍵:⍺ eval ↑1↓↑⍵
         0≡(⍺ eval ↑↑⍵):⍺ ∇ 1↓⍵
         ⍺ eval ↑1↓↑⍵
     }

    ⍝ Initial environment
     env0←⍉⍪["+" ,⊂"+/"]
     env0⍪←["-" ,⊂"-/"]
     env0⍪←["*" ,⊂"×/"]
     env0⍪←["=" ,⊂"=/"]
     env0⍪←["write" ,⊂"⎕←"]

     [e p x]←⍵ parse 0   ⍝ e: error code, p: position, x: AST
     e≠0:x
     ⍺←1 ⋄ ⍺=0:x         ⍝ ⍺=0: parse only
     env0 eval x
 }

⍝ From https://dfns.dyalog.com/c_Cut.htm
 Cut←{
 ⍝ Model of cut.  Works in either index-origin.
 ⍝ Iverson, K.E., Rationalized APL, 1983, Section K. http://www.jsoftware.com/papers/RationalizedAPL.htm
 ⍝ Hui, R.K.W., Some Uses of { and }, 1987, Section 3.2. http://www.jsoftware.com/papers/from.htm
 ⍝ Iverson, K.E., A Dictionary of APL, 1987, m⍤v. http://www.jsoftware.com/papers/APLDictionary.htm
 ⍝ Hui, R.K.W., and K.E. Iverson, J Introduction and Dictionary, 2011, Cut (;.). http://www.jsoftware.com/help/dictionary/d331.htm

     ⍺←(|⍹){                          ⍝ default left argument
         0=⍺:⍉1,⍪-⍴⍵
         1=⍺:0=(1↑⍵)⍳⍵
         2=⍺:0=(¯1↑⍵)⍳⍵
         3=⍺:(⍴⍴⍵)⍴⌊/⍴⍵
     }⍵

     ⍺ ⍶{⍶ ⍵}{

         Under←{⍹⍣¯1⊢(⍹ ⍺)⍶(⍹ ⍵)}    ⍝ dyadic case of "under" operator
         p2←{(¯1⌽k↓⍺)⊂⍠⍶((⍶⍴0),k←-(⌽⍺)⍳1)↓⍵}
         Blk1←{0=≢⍺:⍵ ⋄ 0=≢↑⍺:(1↓⍺)((1+⍶)⍢)⍵ ⋄ (1↓⍺)((1+⍶)⍢)⊃(⊂↑⍺)⊂⍠⍶¨⍵}
         Blk2←{0=≢⍺:⍵ ⋄ 0=≢↑⍺:(1↓⍺)((1+⍶)⍢)⍵ ⋄ (1↓⍺)((1+⍶)⍢)⊃(⊂↑⍺)(⍶ p2)¨⍵}
         ifv←{(⍉⍺,⍪)⍣(2>≢⍴⍵)⊢⍵}          ⍝ laminate ⍺ if ⍵ is vector or scalar
         ci←{(0>s){⌽⍣⍺⊢⍵}¨(⍳¨|s)+(((⍴i)↑⍴⍵)|i)-(0>i)×¯1+|s⊣[i s]←↓(1 ifv ⍺)}
         ti←{⊃{⊃[⍵ (×s)×(|s)⌊r-⍵]}¨m⊸×¨⍳⌈r÷m+(0=m)×r←(⍴m)⍴⍴⍵⊣[m s]←↓(1 ifv ⍺)}
         tj←{⊃{⊃[⍵ s]}¨m⊸×¨⍳⌈(1+r-|s)÷m+(0=m)×r←(⍴m)⍴⍴⍵⊣[m s]←↓(1 ifv ⍺)}

         (1=|⍹)∧(⍬≡⍺)∨1≠≡⍺:⊃⍶¨(⊂(0>⍹)××≢¨⍺)↓¨⍺(0 Blk1)⊂⍵
         (2=|⍹)∧(⍬≡⍺)∨1≠≡⍺:⊃⍶¨(⊂(0>⍹)×-×≢¨⍺)↓¨⍺(0 Blk2)⊂⍵

         0=⍹:⍺(⍶ ci⌷⊢)⍵
         1=⍹:⊃⍺(⍶¨⊂⍠0)⍵
         ¯1=⍹:⊃⍺(⍶⍤(1⊸↓)¨⊂⍠0)⍵
         2=⍹:⍺ ((⍶⍤⊖)⍢1)Under⊖ ⍵
         ¯2=⍹:⍺ (⍶⍤(¯1⊸↓))⍢2 ⍵
         3=⍹:(⍺ ti ⍵) (⍶⍢0)⍤2 ∞ ⍵
         ¯3=⍹:(⍺ tj ⍵) (⍶⍢0)⍤2 ∞ ⍵

         *"domain error"
     }⍹ ⍵
 }

⍝ Dyalog 20.0.53963.0 ⎕AV and name-start alphabet; codecs retain their original wire format.
dfnsAV←•ucs 0 8 10 13 32 12 6 7 27 9 9014 619 37 39 9082 9077 95 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 1 2 175 46 9068 48 49 50 51 52 53 54 55 56 57 3 8866 165 36 163 162 8710 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 4 5 253 183 127 9049 193 194 195 199 200 202 203 204 205 206 207 208 210 211 212 213 217 218 219 221 254 227 236 240 242 245 123 8364 125 8867 9015 168 192 196 197 198 9064 201 209 214 216 220 223 224 225 226 228 229 230 231 232 233 234 235 237 238 239 241 91 47 9023 92 9024 60 8804 61 8805 62 8800 8744 8743 45 43 247 215 63 8714 9076 126 8593 8595 9075 9675 42 8968 8970 8711 8728 40 8834 8835 8745 8746 8869 8868 124 59 44 9073 9074 9042 9035 9033 9021 8854 9055 9017 33 9045 9038 9067 9066 8801 8802 243 244 246 248 34 35 30 38 180 9496 9488 9484 9492 9532 9472 9500 9508 9524 9516 9474 64 249 250 251 94 252 96 8739 182 58 9079 191 161 8900 8592 8594 9053 41 93 31 160 167 9109 9054 9059
dfnsLetters←"_abcdefghijklmnopqrstuvwxyz∆ABCDEFGHIJKLMNOPQRSTUVWXYZ⍙ÁÂÃÇÈÊËÌÍÎÏÐÒÓÔÕÙÚÛÝþãìðòõÀÄÅÆÉÑÖØÜßàáâäåæçèéêëíîïñóôöøùúûü"

⍝ From https://dfns.dyalog.com/c_words.htm
words←{  ⍝ Split a string into words and intervening text.
  ⍺←[dfnsLetters •d]
  1=≡,⍺:[⍺ ""]∇⍵
  [alph supp]←⍺
  w←0{(⍵∊alph)∨⍺∧⍵∊supp}\⍵
  (1++\w≠¯1⌽w)⊆⍵
}

⍝ From https://dfns.dyalog.com/c_eis.htm
 eis←{                   ⍝ enclose-if-simple / link
     m←0 ⋄ ⍺←m←1 ⋄ ~m:(∇ ⍺),∇⍵   ⍝ dyadic: link
     {⊂1/⍵}⍣(1=≡,⍵)⊢⍵    ⍝ monadic: enclose if simple
 }

⍝ From https://dfns.dyalog.com/c_iotag.htm
 iotag←{                        ⍝ Generalized ⍳.
     AlphaInterval←{⍵=' ':""         ⍝ ⍳' ' is empty vector.
         a←dfnsAV⍳"a0AÁ"                ⍝ Indices of a0AÁ.
         k←a+.≤i←dfnsAV⍳⍵             ⍝ Find starting point.
         ⍺←(a.(k-1))⌷dfnsAV               ⍝ Default left argument.
         </i,j←dfnsAV⍳⍺:⌽ ⍵ ∇ ⍺         ⍝ If ⍵ before ⍺, reverse.
         j↓(i+1)↑dfnsAV             ⍝ Truncate dfnsAV.
     }
   ⍝Interval←{⍺+0,+\(|d)⍴×d←⍵-⍺}    ⍝ Interval Function.
     Interval←{s←×/1↓⍵,(⍺>↑⍵)/¯1     ⍝ Calculate step size
         ⍺+s×⍳⌊1-(⍺-↑⍵)÷s}         ⍝ Generate Interval
     IndexOf←{i←1↓⍳⍴⍴⍺               ⍝ Enclose left arg axis.
         j←(1-⍴⍴⍺)↑⍳⍴⍴⍵              ⍝ Enclose right arg axis.
         m←([i;]⌷⍴⍺)⌈[j;]⌷⍴⍵         ⍝ Pad both arguments.
         (⊂⍠i m↑⍠i ⍺)⍳⊂⍠j m↑⍠j ⍵           ⍝ Get index.
     }
     ischar←{' '≡↑0⍴,⍵}          ⍝ character fill.
     m←0 ⋄ ⍺←m←1                      ⍝ Monadic?
     m∧1=⍴⍴⍵:,⌝/∇¨⍵                ⍝ Vector right argument
     m∧ischar ⍵:AlphaInterval ⍵      ⍝ Alpha Monadic.
     m:(×⍵)×⍳|⍵                      ⍝ Integer Monadic.
     s←0=⍴⍴⍺                         ⍝ Scalar Left Argument?
     s∧ischar ⍵:⍺ AlphaInterval ⍵    ⍝ Alpha Interval.
     s:⍺ Interval ⍵                  ⍝ Numeric Interval.
     ⍺ IndexOf ⍵                     ⍝ Dyadic.
 }

⍝ From https://dfns.dyalog.com/c_tokens.htm
 tokens←{                           ⍝ Lex of APL src line.
     ⍺←0 ⋄ nv←⍺                          ⍝ numeric vectors as single token.
     alph←dfnsLetters         ⍝ alphabet for names.
     all←{+/∧\⍺∊⍵}                       ⍝ No. of leading ⍺∊⍵.
     acc←{(⍺,⊂↑/⍵)lex↓/⍵}                ⍝ accumulated tokens.
     lex←{
         0=⍴⍵:⍺ ⋄ hd←↑⍵                  ⍝ Next char else finished.
         hd=' ':⍺{                       ⍝ White Space.
             size←⍵ all ' '
             ⍺ acc [size ⍵]
         }⍵
         hd∊alph:⍺{                      ⍝ Name
             size←⍵ all alph,•d
             ⍺ acc [size ⍵]
         }⍵
         hd∊'⎕':⍺{                       ⍝ System Name
             size←1 + (1↓⍵) all alph
             ⍺ acc [size ⍵]
         }⍵
         hd=''':⍺{                       ⍝ Char literal
             size←+/∧\{⍵∨¯1⌽⍵}≠\hd=⍵
             ⍺ acc [size ⍵]
         }⍵
         hd∊•d,'¯':⍺{                    ⍝ Numeric literal
             max←⍵ all •d,".¯EJ",nv/' '  ⍝ numbers with trailing blanks.
             size←max-+/∧\' '=⌽max↑⍵     ⍝  ..  without trailing blanks.
             ⍺ acc [size ⍵]
         }⍵
         hd∊"⍺⍵∇:":⍺{                    ⍝ ⍺⍺ or ⍵⍵ or ∇∇ or ::
             size←⍵ all hd
             ⍺ acc [size ⍵]
         }⍵
         hd='⍝':⍺ acc [⍴⍵ ⍵]             ⍝ Comment
         ⍺ acc [1 ⍵]                     ⍝ Single char token.
     }
     (0⍴⊂"")lex,⍵
 }

⍝ From https://dfns.dyalog.com/c_ssword.htm
 ssword←{                   ⍝ Approx alternative to xutils' ss.
     [srce find repl]←,¨⍵        ⍝ source, find and replace vectors
     alph←dfnsLetters ⍝ primary alphabet: initial letters for names
     supp←•d                     ⍝ supplementary: 0-9
     ⍺←[alph supp]               ⍝ default left argument
     ∊(⊂repl)@{                  ⍝ replace find with repl
         ⍵∊⊂find                 ⍝ matches
     }(⍺ words srce)             ⍝ source into words.
 }

⍝ From https://dfns.dyalog.com/c_packD.htm
 packD←{                                 ⍝ Pack char array to boolean vector.

     key←,/{↓(⍳⍵)>⌝⍳⍵}¨1+⍳23             ⍝ generate the keys (up to 276), they
                                        ⍝ start with ones and end with zeros.
     cmp←{                               ⍝ compress:
         dt←,⍵ ⋄ k←1,¨key                ⍝ vectorise items, separators to keys
         u←{⍵{⍺[⍒⍵]}{+/⍵=dt}¨⍵}∪dt       ⍝ sort uniques according to frequencies.
         uq←,⍉(8⍴2)⊤(¯1+⍴u),dfnsAV⍳u        ⍝ shape then uniques fit to eight bits.
         rk←,(4⍴2)⊤⍴⍴⍵                   ⍝ rank fits in four bytes

         sh←⍬{                           ⍝ shape is a trickier one
             0∊⍴⍵:⍺                      ⍝ all done?
             n←1+⌊2⍟1⌈↑⍵                 ⍝ how many bits are needed for representing the integer?
             q←(,(5⍴2)⊤n),,(n⍴2)⊤↑⍵      ⍝ n should fit to five bits (2*2*5 -> 4GB dimensions supported ;) )
             (⍺,q) ∇ 1↓⍵                 ⍝ add binarised figure to the result
         }⍴⍵

         rk,sh,uq,∊k[u⍳dt]               ⍝ join the result with binarised data
     }

     unc←{                               ⍝ uncompress:
         rk←2⊥4⍴⍵                        ⍝ rank's easy

         [sh x]←[⍬ 4 rk]{                ⍝ shape needs more treatment
             [s n r]←⍺                   ⍝ deliver args
             r=0:[s n]                   ⍝ all done: return shape and offset
             q←2⊥5⍴⍵                     ⍝ how many bits?
             d←2⊥q⍴5↓⍵                   ⍝ -> dimension length
             [s,d n+q+5 r-1] ∇ (q+5)↓⍵   ⍝ next
         }4↓⍵

         n←1+2⊥8⍴x↓⍵                     ⍝ how many uniques?
         u←dfnsAV[,2⊥⍉[n 8]⍴(n×8)↑(x+8)↓⍵] ⍝  -> make the list
         dt←(x+8×n+1)↓⍵                  ⍝ rip the data part
         p←dt≤¯1↓0,dt                    ⍝ binary partitioner (first 1's)
         sh⍴u[key⍳p⊆dt]                  ⍝ reconstruct
     }

     ⍺←1 ⋄ ⍺=1:cmp ⍵ ⋄ unc ⍵             ⍝ which way to go?
 }

⍝ From https://dfns.dyalog.com/c_packT.htm
 packT←{                        ⍝ Simple text vector packager.

     cmp←{
         ec←{                                ⍝ find the suitable esc character
             c←0⊃dfnsAV ⋄ n←+/⍵=c ⋄ n=0:c
             d←1⊃dfnsAV ⋄ o←+/⍵=d ⋄ o=0:d
             e←2⊃dfnsAV ⋄ p←+/⍵=e ⋄ p=0:e
             m←⌊/[n o p] ⋄ ([n o p]⍳m)⌷[c d e]
         },⍵

         lv←1,≠/2↕,⍵ ⋄ e←ec=,⍵                ⍝ repeat lengths
         ((e<¯1↓1 0 0 1⍷lv,1)/1⌽lv)←1        ⍝ remove 2's and 3's
         ((e<¯1↓1 0 1⍷lv,1)/1⌽lv)←1          ⍝
         ln←{-⍨/2↕(⍵,1)/⍳1+⍴⍵}lv              ⍝ packables

         (lv/lv)←1+⌊ln÷256                   ⍝ adjust for 256 length

         ln←∊{⍵<257:⍵                        ⍝ now we have to adjust the long
             q←256|⍵ ⋄ n←⌊⍵÷256              ⍝ sequences so that the last
             {                               ⍝ sequence will be
                 3<¯1↑⍵:⍵                    ⍝ properly done, ie:
                 (¯2↓⍵),¯4 4+¯2↑⍵            ⍝   257 -> 252 5,
             }(n/256),⍵-n×256                ⍝   NOT    256 1.
         }¨ln

         q←lv/,⍵                             ⍝ reduce data
         b←(ln>3)∨{(↑⍵),</2↕⍵}q=ec            ⍝ find places where
         (b/q)←(b/q){ec,⍺,⍵}¨dfnsAV[¯1+b/ln]       ⍝  to put pack sequences
         ec,∊q                               ⍝ remove nesting
     }

     exp←{
         ec←↑⍵                               ⍝ "esc"
         ~ec∊1↓⍵:1↓⍵                         ⍝ packed?

         bv←{(↑⍵),</2↕⍵}1↓⍵=ec                ⍝ reduce vector
         ln←0@{                              ⍝ lengths (remove escs)
             ¯2⌽bv}(1+dfnsAV⍳(¯2⌽bv)/1↓⍵)@{¯1⌽bv  ⍝ remove ascs
         }~bv                                ⍝ lengths of characters
         ln/1↓⍵                              ⍝ do it!
     }

     ⍺←1 ⋄ ⍺:cmp ⍵ ⋄ exp ⍵                   ⍝ compress or expand.
 }
⍝ From https://dfns.dyalog.com/c_parse.htm
 parse←{  ⍝ Bunda-Gerth parsing.

     [opt defs]←(⍵≡"")({[⍺ ⍵]}⍣(~0≡↑0⍴⍺))⍺     ⍝ trace/format option and defns.

     defn←{                                      ⍝ binding table definition.
         words←' 'segs¨↑¨'⍝'segs¨⍵               ⍝ blank-delimited words.
         pop←{[↑⍵ 1↓⍵]}                          ⍝ head & tail of list.
         sects←(⊂0⍴⊂"") segs words               ⍝ sections.
         [csect defs]←pop sects                  ⍝ separation of sections.
         bindx←(↑¨csect)⍳⊂"()"                   ⍝ index of bracket defn.
         bsegs←bindx⊃csect,⊂,⊂"()"               ⍝ bracket pairs: () [] <> ...
         [bkts blabs]←↓⍉⊃{[⌽2↑⍵ 2↓⍵]}¨¯1⌽¨bsegs  ⍝ brackets and their cat labels.
         [cats reps]←↓⍉⊃pop¨csect~⊂bsegs         ⍝ categories & representatives.
         bonds←⌽1+⍳≢defs                         ⍝ binding strengths
         bmat←cats{0}⌝cats                      ⍝ initialised binding table
         defn←{1+cats⊸⍳¨'.'segs¨(":→"segs ⍵)}    ⍝ split a.b:c.d→z defn
         dist←{(,,⌝/2↑⍵),¨↑⌽⍵}                 ⍝ distribution of productions
         lmat←dist⍤defn¨¨¨defs                   ⍝ loading matrix
         bftz←,/bonds,¨¨(,/)⍣2¨lmat           ⍝ bond-from-to-rslt tuples
         [cats reps bkts blabs],⊂⍠(0 1)⊃{        ⍝ binding structure.
             [bond fm to rslt]←⍺                 ⍝ binding and resulting cats.
             (⊂[bond rslt])@(⊂¯1+[fm to])⍵       ⍝ populate cell
         }/bftz,⊂bmat                            ⍝ loaded binding matrix.
     }⍤{                                         ⍝ pre-process alias=... lines.
         lines←↓' ',⊃⍵                       ⍝ lines from char array.
         wds←↑¨' 'segs¨lines                     ⍝ first word from each line.
         [msk nsk]←1 0=⊂'='∊¨wds                 ⍝ mask of alias lines.
         dict←'='segs¨msk/wds                    ⍝ (fm to) substitution pairs.
         {subs/dict,⊂⍵}¨nsk/lines               ⍝ lines with expanded aliases.
     }⍣(2≠≢⍴↑⌽defs)                             ⍝ compile unless compiled.

     table←{                                     ⍝ formatted Bunda-Gerth table.
         [cats bmat zmat]←⍵                      ⍝ categories and binding matrix.
         fmt←{(×⍺)/(⍕⍺),' ',⍵⊃cats,'?'}          ⍝ bond and category.
         ttl←{[' ' ⍺],.⍪[⍺ ⍵]}                   ⍝ row and column headers.
         cats ttl bmat fmt¨ (1+≢cats)|zmat-1     ⍝ formatted bonds & categories.
     }

     [cats reps bkts blabs bmat zmat]←defn defs  ⍝ definition structure.

     ⍵≡"":{                                      ⍝ null expr: binding matrix.
         ⍵=0:[cats reps bkts blabs bmat zmat]    ⍝ raw binding struct.
         trim←{                                  ⍝ without empty rows and cols.
             [r c]←1,¨1 0{∨/⍠⍺ ⍵}¨⊂1 1↓~⍵∊⊂""    ⍝ occupied rows and cols.
             r⌿c/⍵                               ⍝ masked out empties.
         }                                       ⍝ :: ∇ cells → cells
         snip←{                                  ⍝ snip out top corner.
             t←1+(,⍵)⍳'┬'                        ⍝ width up to the uppermost, leftmost ┬
             cnr←⊃[1 t-2 1]⊸/¨"  ┌" "  │" "┌─┼"    ⍝ empty corner.
             cnr@((⍳3),⌝⍳t)⊢⍵                 ⍝ snipped formatted matrix.
         }                                       ⍝ cmat ← ∇ cmat
         bfmt←snip⍤disp⍤(⍕¨)⍤trim⍤table          ⍝ binding matrix formatting.
         bfmt [cats bmat zmat]                   ⍝ formatted table.
     }opt                                        ⍝ opt-ional formatting.

     reduce←{                                    ⍝ 2-by-2 parsing.
         [[∆_ L] Aa Bb Cc [R _∆]]←⍵              ⍝ 3-token window on stream.
         [Aa Cc]∧.≡eos:Bb                        ⍝ single node: done.
         [Aa Bb]∧.≡eos:⍵                         ⍝ error: partial parse.
         [[A a] [B b] [C c]]←[Aa Bb Cc]          ⍝ cats and toks.
         (⊂[b c])∊1↓bkts:∇ rgt [[∆_ L] Aa (ebk b) R _∆] ⍝ empty brackets [].
         (⊂[a c])∊bkts:∇ rgt [∆_ L (a bkt Bb) R _∆]    ⍝ bracketed single value Bb.
         (⊂a)∊rbs:∇ lft lft ⍵                    ⍝ right bracket: skip left.
         ≥/(xmat [[A B] [B C]]):∇ lft ⍵          ⍝ A:B ≥ B:C → skip left.
         BbCc←zmat.[B-1;C-1],⊂[b c]              ⍝ B bound with C.
         ∇ show [[∆_ L] Aa BbCc R _∆]            ⍝ binds with token to the right?
     }                                           ⍝ :: ∇ stream → stream

     show←⊣⟜{⎕←sfmt lft⍣{eos≡↑⍺}⍵}⍣opt⍨          ⍝ optional tracing, en passant.

     bkt←{                                       ⍝ bind of bracketed node [ ⍵ ].
         [cat expr]←⍵                            ⍝ category of bracketed expr.
         zcat←(lbs⍳⍺)⌷cat,1↓bcats                ⍝ resulting category.
         [zcat [⍺ expr]]                         ⍝ ⍺-bracketed node.
     }                                           ⍝ :: left_bkt ∇ node → node

     ebk←{[bcats.(lbs⍳⍵) ⍵]}                      ⍝ empty brackets. [] {} ...

     tfmt←{                                      ⍝ tree-formatted.
         0=≡⍵:1 1⍴⍵                              ⍝ atom: char matrix single.
         subs←⍉⊃{⍺,' ',⍵}/↓⍠0⍤∇¨⍵                ⍝ formatted sub-expressions.
         mask←~(↑↓subs)∊"┌─┐ "                   ⍝ sub-exprs connection points.
         mid←(⍳⍴mask)=⌊(+/⍸mask)÷2               ⍝ mid-point for '┴' char.
         inx←mask+2×+\mask                       ⍝ indices for box-draw chars.
         top←" ?─┌ ┐┴"[inx+4×mid]                 ⍝ "  ┌─┴─┐  "
         top⍪subs                                ⍝ formatted expression.
     }

     atop←cats{                                  ⍝ category atop tree.
         ⍺=0:⍵                                  ⍝ ignore bad cat.
         dent←+/∧\(↑↓⍵)∊"┌─┐ "                   ⍝ indent for category.
         top←(dent/' '),(⍺-1)⊃⍶⊣0               ⍝ indented category.
         ⊃(⊂top),↓⍵                              ⍝ categorised tree.
     }

     vect←{⍺←⍬                                   ⍝ vector from cons list
         [∆_ Aa Bb Cc _∆]←⍵                      ⍝ 3-item window.
         ~Aa≡eos:(⍺,⊂Aa) ∇ rgt ⍵                 ⍝ accumlate next item.
         ~⍺≡⍬:⍺                                  ⍝ trailing eos: done.
         ⍺ ∇ rgt ⍵                               ⍝ skip leading eos.
     }

     class←{                                     ⍝ classification of expr tokens.
         pairs←,/⍺,¨¨1+⍳⍴⍺                      ⍝ token-category pairs.
         [toks cats]←↓⍉⊃pairs                    ⍝ category of each token.
         (toks⍳⍵)⊃¨⊂cats,0                      ⍝ token categories.
     }

     pfmt←atop⟜tfmt⌿                             ⍝ format of (token cat) pair.
     sfmt←disp⍤pfmt⍤⍉⍤⊃⍤vect                     ⍝ format of parse stream.

     lft←{[[∆_ L] A B C _∆]←⍵ ⋄ [∆_ L A B [C _∆]]}     ⍝ skip left.
     rgt←{[∆_ A B C [R _∆]]←⍵ ⋄ [[∆_ A] B C R _∆]}     ⍝ skip right.

     [lbs rbs]←↓⍉⊃bkts                           ⍝ left and right brackets.
     bcats←1+cats⍳blabs                          ⍝ bracket categories.
     xmat←0,0⍪bmat                               ⍝ extended bmat.
     pairs←{↓⍉⊃[(reps class ⍵) ⍵]}⍵~' '           ⍝ cat-token pairs.
     eos←0                                      ⍝ end of stream marker.

     ∆_←{[⍺ ⍵]}⍨/⌽eos,¯2↓pairs                  ⍝ left list.
     [Aa Bb Cc]←¯3↑[eos eos],pairs,eos           ⍝ 3-token window

     tree←reduce show [∆_ Aa Bb Cc eos]          ⍝ reduced expression.
     eos≡↑tree:sfmt tree                         ⍝ bad parse: show stream.
     pfmt tree                                  ⍝ good parse: show tree.
 }

⍝ From https://dfns.dyalog.com/c_tc.htm
tc←{  ⍝ Trace function application.
  m←0 ⋄ ⍺←m←1
  m:↑⌽⎕←[⍶ ⍵ '⇒' (⍶ ⍵)]
  ↑⌽⎕←[⍺ ⍶ ⍵ '⇒' (⍺ ⍶ ⍵)]
}

⍝ From https://dfns.dyalog.com/c_cols.htm
 cols←{                         ⍝ Multi-column display.                   
                                                                               
     ⍺←1 102                         ⍝ default inter-column-gap and max-width. 
                                                                               
     split←{(+/∨\' '≠⌽⍵)↑¨↓⍵}        ⍝ without-trailing-blanks-split (idiom).  
                                                                               
     1<⍴⍴⍵:⍺ ∇ split ⍵               ⍝ char matrix: use the split vector.      
                                                                               
     maxcols←{                       ⍝ maximum number of cols.                 
         ⍵=0:0                       ⍝ one or more items too wide: stop.       
         [nr xs]←|[0 ⍵]⊤-ni          ⍝ number of rows and excess.              
         xs≥nr:∇ ⍵-1                 ⍝ one or more empty columns: ignore.      
         ms←[⍵ nr]⍴lens              ⍝ matrix of sizes.                        
         wid←(gap×⍵-1)++/⌈/ms        ⍝ total width.                            
         wid≤max:⍵                   ⍝ it fits: hurrah!                        
         ∇ ⍵-1                       ⍝ otherwise, try one less column.         
     }                                                                         
                                                                               
     reshape←{                       ⍝ ⍺-col reshape of ⍵.                     
         ⍺=0:⊃max↑¨↓(col ⍵)          ⍝ item too wide: truncated single col.    
         nr←⌈ni÷⍺                    ⍝ number of rows.                         
         matr←[⍺ nr]⍴(⍺×nr)↑⍵,⍺/' '  ⍝ matrix blank-padded to whole no of cols.
         pads←⊃nr/↓gap/' '           ⍝ inter-column padding.                   
         {⍺,pads,⍵}/col¨↓matr       ⍝ gap-spaced cols.                        
     }                                                                         
                                                                               
     col←⍕⍤{1=≡⍵:⍪⍵ ⋄ ⊃⍵}            ⍝ formatted numeric or char-vecs column.  
                                                                               
     [gap max]←2↑⍺,102               ⍝ inter-column-gap and max-width.         
     ni←≢⍵                           ⍝ number of items.                        
     lens←(2×≢⍵)↑,⊃⍴¨⍕¨⍵             ⍝ item-lengths.                           
     ulim←⌈max÷1⌈gap+⌊/ni↑lens       ⍝ cols: upper limit for search.           
                                                                               
     (maxcols ulim) reshape ⍵        ⍝ multi-column display.                   
 }                                                                             

cal←{ ⍝ Calendar for absolute year or (year month).
    ⍺←1
    cntr←{(⌈0.5×+/∧\' '=⍵)⌽⍵}
    1=≢,⍵:{
        12≥|⍵:•signal "DOMAIN ERROR"
        year←4 3⍴(0 cal¨ ⍵,¨1+⍳12)
        join←{⍉⊃(↓⍉⍺),"   ",↓⍉⍵}
        head←cntr ¯66↑⍕0+⍵
        head⍪,⍠(⍳2)⊃join/year
    }⍵
    dys←"Su" "Mo" "Tu" "We" "Th" "Fr" "Sa"
    months←"January" "February" "March" "April" "May" "June"
    months,←"July" "August" "September" "October" "November" "December"
    [yyyy mm]←⍵
    day←days [yyyy mm 1]
    [mms dds]←2↑1↓↓⍉(date day+⍳31)
    fmts←2 0⊸⍕¨(mm=mms)/dds
    pad←(7|day)↑0↑fmts
    dmat←⊃{⍺,' ',⍵}/dys⍪6 7⍴42↑pad,fmts
    head←((mm-1)⊃months),⍺/' ',⍕0+yyyy
    ⊃(⊂cntr ¯20↑head),↓{(∨/⍵≠' ')⌿⍵}dmat
}
packZ←{ ⍝ LZW: positive bit limit compresses; zero expands; negative returns dictionary.
    ⍺←12
    ⍺=0:{
        [shape bits alph]←⍵
        codes←2⊥bits
        expand←{
            [codes dict prev out]←⍵
            0=≢codes:out
            n←↑codes
            text←{⍵<≢dict:⍵⊃dict ⋄ prev,↑prev}n
            next←dict,(0<≢prev)/⊂prev,↑text
            ∇ [1↓codes next text out,text]
        }
        shape⍴(expand [codes ,¨alph 0↑alph 0↑alph])
    }⍵
    shape←⍴⍵ ⋄ src←,⍵ ⋄ alph←∪src
    limit←2*|⍺
    limit<≢alph:•signal "DOMAIN ERROR"
    compress←{
        [rest dict word codes]←⍵
        0=≢rest:[codes,(0<≢word)/dict⍳⊂word dict]
        next←word,↑rest
        (⊂next)∊dict:∇ [1↓rest dict next codes]
        extended←dict,(limit>≢dict)/⊂next
        ∇ [1↓rest extended 1↑rest codes,dict⍳⊂word]
    }
    [codes dict]←compress [src ,¨alph 0↑alph ⍬]
    ⍺<0:⊃dict
    width←⌈2⍟1+⌈/codes,1
    [shape (width⍴2)⊤codes alph]
}

unify←{ ⍝ Unify expressions; ⍺ lists variable symbols.
    vars←,⍺
    isvar←{∨/⍵⊸≡¨vars}
    disagree←{
        ⍺≡⍵:⍬
        isvar ⍺:[⍺ ⍵]
        isvar ⍵:[⍵ ⍺]
        0∊≡¨[⍺ ⍵]:•signal "DOMAIN ERROR"
        ~(⍴⍺)≡⍴⍵:•signal "LENGTH ERROR"
        i←(,⍺≡¨⍵)⍳0
        (i⊃,⍺)∇(i⊃,⍵)
    }
    solve←{
        [x y]←⍵
        pair←x disagree y
        0=≢pair:x
        [var val]←pair
        occurs←{⍵≡var:1 ⋄ 0=≡⍵:0 ⋄ ∨/,∇¨⍵}
        occurs val:•signal "DOMAIN ERROR"
        subst←{⍵≡var:val ⋄ 0=≡⍵:⍵ ⋄ ∇¨⍵}
        ∇ [(subst x) (subst y)]
    }
    {solve [⍺ ⍵]}/⍵
}

ratsum←{
    sum←{[lrus mans rrus]←⍵
        [cr rru]←⍺ rrusum rrus
        [cm man_]←cr rsum mans
        [lru man]←cm lrusum [lrus man_]
        [lru man rru]
    }
    rrusum←{
        [co rru]←⍺ rsum ⍵
        co=⍺:[co rru]
        co rsum ⍵
    }
    lrusum←{[lrus man]←⍵
        [co lru]←⍺ rsum lrus
        co=⍺:[lru man]
        [cc dd]←⍺ rsum 2 ¯1↑lrus
        cc ∇ [¯1⌽lrus dd,man]
    }
    atab←{
        min←⍵⍳'0'
        vals←(⍳≢⍵)-min
        ntab←vals+⌝vals
        base←2/⍴⍵
        vtab←-min-base⊤base⊥min+base⊤ntab
        ptab←↓(min+(¯1⌽⍳3)⍉vtab)⊃¨⊂⍵
        {(ptab,⍵)⍪⍵,⊂"0."}⍵,¨'.'
    }
    rsum←(atab ⍶~"{}"){
        [cov itot]←↓⍉⊃⍶[↓⍉digs⍳⍵]
        '0'∧.=cov,⍺:['0' itot]
        [co tot]←'0'∇⊃[1↓cov,⍺ itot]
        [↑↑⌽'0'∇⊃co,⊂1↑cov tot]
    }
    compile←{
        [lmrs wids reps]←↓⍉⊃trans¨[⍺ ⍵]
        mpads←-wids-⊂⌈/wids
        rpads←2/⊂∧¨/reps
        ⊃¨↓⍉⊃pad¨↓⍉⊃[lmrs mpads rpads]
    }
    trans←{
        ~∧/⍵∊digs,"<|.>":err"bad char"
        ~"<>"≡(ext ⍵):err"bad <>s"
        ~"<>"≡⍵∩"<>":err"bad <>s"
        ~"||"≡⍵∩"||":err"bad ||s"
        [lru man rru]←'|'sepr ⍵~"<>"
        1∊[lru man rru]∊⊂"":err"null field"
        '.'∊lru,rru:err"bad number"
        ml←man⍳'.'
        mr←(⍴man)-ml+'.'∊man
        [[lru man rru] ml,mr ,⊃⍴¨[lru rru]]
    }
    pad←{
        [[l m r] [ml mr] [lr rr]]←⍵
        dot←'.'~m,mr↓'.'
        man←(⌽ml⍴⌽l),m,dot,mr⍴r
        rru←rr⍴mr⌽r
        lru←⌽lr⍴⌽(-ml)⌽l
        [lru man rru]
    }
    norm←{crru xabs icon clru ⍵}
    clru←{[lru man rru]←⍵
        ∨/∧/optl=⌝lru:⍵
        [lw rw]←⌈\⍴¨[lru rru]
        mw←⍴man~'.'
        _lru←cdigs[digs⍳lru]
        _man←". "repl(man≠'.')\mw⍴_lru
        _rru←rw⍴mw⌽_lru
        _lmr←[_lru _man _rru]
        lmrr←[lru man],⊂rw⍴rru
        ∇ '0' sum ⊃¨↓⍉⊃[_lmr lmrr]
    }
    icon←{[lru man rru]←⍵
        [⌽minrep⌽lru man (minrep rru)]
    }
    minrep←{
        facs←{(0=⍵|⍴⍵)/⍵}1+⍳⍴⍵
        reps←facs⍴¨⊂⍵
        seqs←(⍴⍵)⍴¨reps
        (seqs⍳⊂⍵)⊃reps
    }
    xabs←{[lru man rru]←⍵
        1=⍴man:⍵
        '.'=↑⌽man:∇ [lru ¯1↓man rru]
        ('.'∊man)∧(↑⌽man)=↑⌽rru:∇ ¯1 amd ⍵
        (~'.'∊2↑man)∧(↑lru)≡↑man:∇ 1 amd ⍵
        ⍵
    }
    amd←{[lru man rru]←⍵
        lru←(⍺⌈0)⌽lru
        rru←(⍺⌊0)⌽rru
        [lru ⍺↓man rru]
    }
    crru←{[lru man rru]←⍵
        ~rru≡,comp'0':⍵
        sig←×-/digs⍳rru,'0'
        inc←(digs⍳'0')⊃sig⌽digs
        zero←'0'⊣¨¨⍵
        inc sum ⊃¨↓⍉⊃[⍵ zero]
    }
    sepr←{⍺{(≢⍺)↓¨(⍺⍷⍵)⊂⍵}⍺,⍵}
    join←{⊃⍺{⍺,⍶,⍵}/⍵}
    repl←{⊃⍵{⍺ join ⍵ sepr ⍶}/⍺}
    comp←{[(digs,⍵)⍳⍵;]⌷(⌽digs),⍵}
    fmt←{'|' ".|"repl('|'join ⍵)join"<>"}
    ext←{⌽2↑¯1⌽⍵}
    err←{•signal "DOMAIN ERROR"}
    digs←⊃↓/(1 ¯1×"{}"≡(ext ⍶)),⊂⍶
    optl←'0',('0'∊(ext digs))/comp'0'
    cdigs←{
        '0'∊(ext ⍵):comp ⍵
        (¯1-2×⍵⍳'0')⌽⌽⍵
    }digs
    xchars←" {}[]()<>\|/,:."
    1∊xchars∊digs:err"bad digit"
    ~'0'∊digs:err"missing zero"
    ~digs≡∪digs:err"duplicate digits"
    dyad←1 ⋄ ⍺←0⊣dyad←0 ⋄ ~dyad:fmt norm ↑ trans comp ⍵
    fmt norm '0' sum ⍺ compile ⍵
}
