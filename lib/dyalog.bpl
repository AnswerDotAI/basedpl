⍝ Dyalog dfns — adapted for BPL
⍝ Source: https://dfns.dyalog.com/n_contents.htm
⍝ Exported from Dyalog 20.0.53963.0; individual sources below.
⍝ Modified to count from 0, for based arrays and for BPL notation.

_array←•load "./array.bpl"
⍝ From https://dfns.dyalog.com/c_segs.htm
segs←{~ω∊α ⊆ ω}  ⍝ Separator-delimited segments.

⍝ From https://dfns.dyalog.com/c_m91.htm
m_91←{ω>100ₓ?ω-10ₓ;∇ ∇ ω+11ₓ}  ⍝ McCarthy's M91 function.

⍝ From https://dfns.dyalog.com/c_life.htm
life←{[$t ω]∨.∧3ₓ 4ₓ=¨⊂+/,¯1 0 1⊖⊗¯1 0 1⌽⊗⊂ω}

⍝ From https://dfns.dyalog.com/c_rmcm.htm
rmcm←{  ⍝ Replace comments with blanks.
  cm←∨\ ω='⍝' > ≠\ω='''
  ' '@ ⍸cm ω
}

⍝ From https://dfns.dyalog.com/c_ripple.htm
ripple←{[(⍋⍒(ρω)ρ0 1)]⌷ω}  ⍝ Perfect ripple shuffle.

⍝ From https://dfns.dyalog.com/c_birthday.htm
birthday←{  ⍝ Probability of a shared birthday among ω people, with α possible dates.
  1 - ×/ α-ιω ÷ α
}

⍝ From https://dfns.dyalog.com/c_lsys.htm
lsys←{  ⍝ Lindenmayer L-system expansion.
  2ₓ≥≡α?⊂,α ∇ ω;
  [fms tos]←↓⍉⊃,α
  ∊[(fms,ω)ιω]⌷tos,ω
}

⍝ From https://dfns.dyalog.com/c_rr.htm
rr←{  ⍝ Round-robin tournament.
  v←ιn←0ₓ⌈≤ω
  ≥(ω÷2)↑₋₁m{[α◇ω]}¹⌽m←0ₓ,v⌽≥[n n]ρv
}

⍝ From https://dfns.dyalog.com/c_easter.htm
easter←{  ⍝ Easter Sunday, yyyymmdd.
  G←≥19ₓ|ω
  C←≥⌊ω÷100ₓ
  X←¯12ₓ+⌊C×3ₓ÷ₐ4ₓ
  Z←¯5ₓ + ⌊ 5ₓ+8ₓ×C ÷ₐ 25ₓ
  S←(⌊ 5ₓ×ω ÷ₐ 4ₓ)-X+10ₓ
  E←30ₓ | 11ₓ×G + 20ₓ+Z-X
  F←E + E=24ₓ ∨ E=25ₓ ∧ G>11ₓ
  N←30ₓ×F>23ₓ + 44ₓ-F
  N+←7ₓ-7ₓ|S+N
  M←3ₓ+N>31ₓ
  D←N-31ₓ×N>31ₓ
  [10000 100 1]ₓ+.×[ω M D]
}

⍝ From https://dfns.dyalog.com/c_dice.htm
dice←{
  ω≡[6 6]ₓ?"Box Cars";
  ω≡[1 1]ₓ?"Snake Eyes";
  =/ω?"Pair";
  7ₓ=+/ω?"Seven";
  "Unlucky"
}

⍝ From https://dfns.dyalog.com/c_packN.htm
packN←{  ⍝ Null packing: shape, non-null mask, non-null items.
  cmp←{mask←,ω≠1↑0ρω ◇ [ρω mask mask#,ω]}
  exp←{[shape mask items]←ω ◇ shapeρmask#⁻items}
  α←$t ◇ α?cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_packR.htm
packR←{  ⍝ Run-length encoding: shape, counts, items.
  cmp←{
    shape←ρω ◇ vect←,ω
    runs←$t,≠/↕vect
    repls←-⍨/↕⍸runs,$t
    [shape repls runs#vect]
  }
  exp←{[shape repls solos]←ω ◇ shapeρrepls#solos}
  α←$t ◇ α?cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_packU.htm
packU←{  ⍝ Unique packing: shape, unique items, zero-based indices.
  cmp←{u←∪,ω ◇ [ρω u uι,ω]}
  exp←{[shape u i]←ω ◇ shapeρ[i]⌷u}
  α←$t ◇ α?cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_date.htm
date←{  ⍝ Timestamp from day number (Meeus).
  α←¯53799ₓ
  qr←{⊂⍠ ι≢ρω (0ₓ,α)⊤ω}
  [Z F]←1ₓ qr ω+2415020ₓ
  a←⌊ Z-1867216.25 ÷ 36524.25
  A←Z + Z≥α+2415021ₓ × ≥a-⌊a÷4ₓ
  B←A+1524ₓ
  C←⌊ B-122.1 ÷ 365.25
  D←⌊C×365.25
  E←⌊ B-D ÷ 30.6001
  [dd df]←1ₓ qr B-D + F-⌊30.6001×E
  mm←E-1ₓ+12ₓ×E≥14ₓ
  yyyy←C-4715ₓ+mm>2ₓ
  part←[60 60 1000]ₓ qr ⌊0.5+df×86400000ₓ
  ⊃₀[yyyy mm dd],part
}

⍝ From https://dfns.dyalog.com/c_days.htm
days←{  ⍝ Days since 1899-12-31 (Meeus).
  α←17520902ₓ
  1ₓ<≢ρω?α ∇⁰ ¹ ω;
  [yy mm dd h m s ms]←7↑ω
  D←dd + [0 60 60 1000]ₓ⊥[h m s ms] ÷ 86400000ₓ
  [Y M]←[yy mm]+[¯1 12]ₓ×mm≤2ₓ
  A←⌊Y÷100ₓ
  B←α<[0 100 100]ₓ⊥[yy mm dd] × 2ₓ-A + ⌊A÷4ₓ
  ¯2416544ₓ+D+B++/⌊365.25 30.6×[Y M]+[4716 1]ₓ
}

⍝ From https://dfns.dyalog.com/c_draw.htm
draw←{  ⍝ Box-drawing characters over a marker.
  α←"*"
  rh←×/¯1↓ρω , ¯1↑1,ρω
  ch←"│─┌─┐─┬││└├┘┤┴┼"
  z←,' '⍪(' ' , rhρω , ' ')⍪' '
  bv←z∊α ◇ ix←⍸bv
  in←{⊥[ix+⊗[-ω ¯1ₓ 1ₓ ω]]⌷bv}2ₓ+rh₁
  z.[(×in # ix)]←[≤in~[0ₓ]]⌷ch
  ρω ρ 1 1↓¯1 ¯1↓(rh+2)ρz
}

⍝ From https://dfns.dyalog.com/c_dots.htm
dots←{  ⍝ Show indentation with white dots.
  α←'·'
  kwds←":E" ":C" ":U"⍷¨⊂ω
  ends←⊃∨/kwds,⊂ω='}'
  spcs←∧\' '=ω
  xdents←ends∧$t,0 ¯1↓spcs
  flood←⍪[1 2]ₓ {hits←α⍷ω ◇ ~$t∊,hits?ω;α ∇ hits+ω} spcs+⊽xdents
  α@ ⍸2ₓ=flood-xdents ω
}

⍝ From https://dfns.dyalog.com/c_logic.htm
logic←{+/(ω+⊽α =⊗ [0 1 2 3]ₓ)×[2 2 2 2]ₓ⊤α⍶ω}

⍝ From https://dfns.dyalog.com/c_Depth.htm
Depth←{  ⍝ Apply ⍶ at depths ⍹.
  α←⊣
  depths←⌽3ρ⌽⍹
  [m l r]←depths-⍨ ≡¨[ω α ω] × depths≥0ₓ
  [max encl]←"¨⊂"ρ¨⍨l(⌈,↢|-)r
  [el er]←$t $f=l≤r #¨ ⊂encl
  ⍎'(',el,"α)⍶",max,er,'ω'
}

⍝ From https://dfns.dyalog.com/c_case.htm
case←{  ⍝ Select statement.
  ~∨/b←<\α?ω;
  ¯1↑b?⍹ ω;
  $t $f≡b?⍶ ω;
  ¯1↓b ⍶ ω
}
⍝ From https://dfns.dyalog.com/c_morse.htm
morse←{  ⍝ Conversion to/from Morse code. A blank is the inter-word separator /.
  [P M]←↓⍉['A' ".-"◇'B' "-..."◇'C' "-.-."◇'D' "-.."◇'E' "."◇'F' "..-."◇'G' "--."◇'H' "...."
    'I' ".."◇'J' ".---"◇'K' "-.-"◇'L' ".-.."◇'M' "--"◇'N' "-."◇'O' "---"◇'P' ".--."
    'Q' "--.-"◇'R' ".-."◇'S' "..."◇'T' "-"◇'U' "..-"◇'V' "...-"◇'W' ".--"◇'X' "-..-"
    'Y' "-.--"◇'Z' "--.."
    '0' "-----"◇'1' ".----"◇'2' "..---"◇'3' "...--"◇'4' "....-"
    '5' "....."◇'6' "-...."◇'7' "--..."◇'8' "---.."◇'9' "----."
    '.' ".-.-.-"◇',' "--..--"◇':' "---..."◇'?' "..--.."◇''' ".----."◇'-' "-....-"
    '/' "-..-."◇'(' "-.--."◇')' "-.--.-"◇'"' ".-..-."◇'@' ".--.-."◇'=' "-...-"
    ' ' "/"]
  1ₓ=≡,ω?[Pιω∩P]⌷M;  ⍝ plain text to Morse.
  2ₓ=≡,ω?[Mιω∩M]⌷P  ⍝ Morse to plain text.
}

⍝ From https://dfns.dyalog.com/c_base64.htm
base_64←{  ⍝ Base64 encoding/decoding of octet vectors.
  chars←"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  bits←{, αρ2ₓ ⊤ ω}
  part←{(ρω ρ α↑$t)⊂ω}
  ' '=↑0ρ,ω?2ₓ↣⊥∘(8↣↑)¨ 8 part {-8|ρω ↓ ω} 6 bits {ω≠64ₓ # ω} charsιω;
  four←{
    8ₓ=ρω?"=="∇ω,[0 0 0 0]ₓ;
    16ₓ=ρω?'='∇ω,[0 0]ₓ;
    chars.[(2ₓ↣⊥¨ 6 part ω)],α
  }
  cats←{"",/ω}
  cats ""↣four¨ 24 part 8 bits ω
}

⍝ From https://dfns.dyalog.com/c_packB.htm
packB←{  ⍝ Unique indices packed as run lengths and bits.
  cmp←{
    u←∪,ω
    i←uι,ω ◇ w←⌈2⊛ρu
    b←$t,≠/↕i
    j←0≠,⍉(wρ2)⊤b#i
    [(ρρω),(ρω),u b j]
  }
  exp←{
    [q b j]←ω ◇ s←↑q
    r←sρ1↓q ◇ u←≥s ↓ q
    w←⌈2⊛ρu
    e←-⍨/↕⍸b,$t
    i←⊥⍉w{α,⌊α÷⍨ρω ρ ω}j
    rρ[e#i]⌷u
  }
  α←$t ◇ α?cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_packX.htm
packX←{  ⍝ Text packed through pairs of unique items.
  cmp←{
    u←∪,ω
    p←↓{2,⍨⌈0.5×ρω ρ ω},ω
    j←,u,⊗u
    b←j∊p ◇ g←b#j
    i←gιp
    w←⌈2⊛ρg
    q←0≠b,,⍉(wρ2)⊤i
    [u (ρρω),ρω q]
  }
  unc←{
    [u d a]←ω
    s←↑d ◇ r←sρ1↓d
    j←,u,⊗u
    p←(ρj ↑ a)#j
    q←ρj ↓ a
    w←⌈2⊛ρp
    i←⊥⍉w{α,⌊α÷⍨ρω ρ ω}q
    rρ∊[i]⌷p
  }
  α←$t ◇ α?cmp ω;unc ω
}

byfreq ← {[⍒(≢⊢)⌸ω]⌷∪ω}  ⍝ Uniques of ω, most frequent first. Shared by packQ and packD.

⍝ From https://dfns.dyalog.com/c_packQ.htm
packQ←{  ⍝ Frequency-ranked uniques with variable-width keys.
  key←,/{↓ ιω >⊗ ιω}¨≥ι23ₓ
  cmp←{
    dt←,ω
    u←byfreq dt
    k←$t,¨key ◇ d←∊[uιdt]⌷k
    [u (ρρω),ρω d]
  }
  unc←{
    [u q d]←ω
    s←↑q ρ 1↓q
    p←d≤¯1↓$f,d
    sρ[keyιp⊆d]⌷u
  }
  α←$t ◇ α?cmp ω;unc ω
}

⍝ From https://dfns.dyalog.com/c_packS.htm
packS←{  ⍝ Shannon-Fano packing.
  cmp←{
    u←∪,ω ◇ b←≢⊢ ⌸,ω ◇ i←⍒b ◇ uv←[i]⌷u ◇ b←[i]⌷b
    bv←{ω≠2 ⊆ ω}∊⍬{
      1ₓ=≢ω?2,α;a←⊂α ,¨ 0 1
      2ₓ=ρω?a ∇¨ ω;
      n←{1⌈+/$f=(+/ω)<⊽+\ω}ω ◇ a ∇¨ [n↑ω n↓ω]
    }b
    z←[uvι,ω]⌷bv
    [ρω uv ∊ρ¨bv 0≠(∊bv),∊z]
  }
  exp←{
    [r uv lv d]←ω
    bv←lv#≥ιρlv ⊆ +/lv ↑ d
    rρ""{
      0ₓ∊ρω?α;
      i←0ₓ{q←α⊃bv ◇ q ≡ ρq ↑ ω?α;≥α ∇ ω}ω
      α,i⊃uv ∇ ρi⊃bv ↓ ω
    } +/lv ↓ d
  }
  α←$t ◇ α?cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_cmat.htm
cmat←{  ⍝ α-combinations of ιω: include the first item, then omit it.
  α=0ₓ?1 0ρ0ₓ;
  α>ω?[0 α]ρ0ₓ;
  α=ω?[1 α]ριω;
  (0ₓ,≥ ≤α ∇ ≤ω)⍪≥α∇≤ω
}

⍝ From https://dfns.dyalog.com/c_mayan.htm
mayan←{  ⍝ Mayan numbers; left 0 uses base 20, left 1 uses the calendar base.
  α←1
  [snail dot bar]←"_@/" '⊛' '⌹'
  n←≥⌈18⊛1⌈ω
  base←20ₓ-2⌽n↑⊽α
  ⍪' '⍪↢{
    ω=0ₓ?1 5ρ' ',snail,' ';
    [comp part]←[0 5]ₓ⊤ω
    top←dot{
      ω=0ₓ?0 5ρ"";
      ω=1ₓ?0 0 1 0 0#⁻α;
      ω=2ₓ?0 1 0 1 0#⁻α;
      ω=3ₓ?0 1 1 1 0#⁻α;
      ω=4ₓ?1 1 1 1 0#⁻α
    }part
    top⍪[comp 5]ρbar
  }¨{-1⌈+/∨\ω≠0ₓ ↑ ω}base⊤ω
}

⍝ From https://dfns.dyalog.com/c_mac.htm
mac←{  ⍝ Macro expansion over linked token lists.
  mexp←{
    [dd [a [b uu]]]←ω
    a≡'\'?α ∇ [[dd b] uu];
    b≡'='?α ∇ defn ω;
    a∊↑α?α ∇ dref ω;
    a≡'('?α ∇ α ∇ [dd [b uu]];
    a≡')'?[dd [b uu]];
    α ∇ [[dd a] [b uu]]
  }
  defn←{
    [dd [name [eq uu]]]←ω
    [val uuu]←α body ['(' uu]
    α,⍨↢⊂¨[name val] ⍶ [dd uuu]
  }
  body←{
    [dd [a [b uu]]]←ω
    a∊↑α?α ∇ dref [dd [a [b uu]]];
    a∊" )",•ucs13?[dd [b uu]];
    a≡'\'?α ∇ [[dd b] uu];
    a≡'('?α ∇ α mexp [dd [b uu]];
    α ∇ [[dd a] [b uu]]
  }
  dref←{
    [names vals]←α
    [dd [name uu]]←ω
    val←namesιname ⊃ vals
    [ddd n]←dd number 1ₓ
    α ⍶ [ddd (val (n copy) uu)]
  }
  number←{
    [dd d]←α
    ~d∊$d?[α 0ₓ];
    [ddd n]←dd ∇ ω×10ₓ
    [ddd n+ω×$dιd]
  }
  copy←{
    α≡'('?ω;
    [vv v]←α
    v≡'/'?vv ∇ list [⍶ 1]#ω;
    vv ∇ [v ω]
  }
  list←{[α ω]}/
  1↓¯2↓∊["" ⍬]mexp['(' (list ω,')')]
}

⍝ From https://dfns.dyalog.com/c_baby.htm
baby←{  ⍝ Manchester Small Scale Experimental Machine.
  fetch←{, yshiftω # α}
  store←{⍶@ 1ι⍨yshift⍹ ω}
  decode←{(ι≢ω ∊ [0 5 13 16]ₓ)⊂ω}
  yshift←0ₓ=ι≢ω ↣{~$t∊ω?α;¯1⌽α ∇ dec ω}
  addsub←{c←α ⍶ ω ◇ ~$t∊c?α ⍹ ω;α≠ω ∇ $f,¯1↓c}
  add←∧addsub∨
  sub←<addsub>
  inc←{ω≠¯1↓$t,∧\ω}
  dec←{ω≠¯1↓$t,∧\~ω}
  neg←{ω≠¯1↓$f,∨\ω}
  α←$f∧2↑↓ω
  $f{
    [A CIo M]←ω
    CI←α add inc CIo
    [S _ PI _]←decode M fetch CI
    $f $f $f≡PI?$f ∇ [A (M fetch S) M];
    $t $f $f≡PI?(M fetch S) ∇ [A CI M];
    $f $t $f≡PI?$f ∇ [(neg M fetch S) CI M];
    $t $t $f≡PI?$f ∇ [A CI (A store S)M];
    $f $f $t≡PI?$f ∇ [(A sub M fetch S) CI M];
    $t $f $t≡PI?$f ∇ [(A sub M fetch S) CI M];
    $f $t $t≡PI?{$f}\⌽A ∇ [A CI M];
    $t $t $t≡PI?M
  }α,⊂ω
}

⍝ From https://dfns.dyalog.com/c_quzzle.htm
quzzle←{  ⍝ Shortest sliding-tile paths to the three other corners.
  extend←{
    (≢Fin)=p←$tι⍨(Fin=0ₓ)∧NPath=⌊/(Fin=0ₓ)#NPath?0;
    (≢Target)=+/(p⌷NPath)≥(Fin>0ₓ)#NPath?0;
    n←¯1⊃p⊃Path
    ∇ delpath p , ,/ p next¨ ,/ n moves¨ Tiles
  }
  moves←{
    m←(,1 ¯1×⊗1,≥Shape₁)(,'_'⍪'∣',Shapeρα⌷State){∧/(ω=α⌽⍶ # ⍶)∊ω,' '}¨ω
    ω,¨m#"←↑→↓"
  }
  next←{
    ' '=t←↑ω?¯1ₓ;
    y←¯1⊃α⊃Path ⌷ State
    x←' '@[=↢t] y
    r←"←↑→↓"ιω₁ ⌷ ,1 ¯1×⊗1,Shape₁
    (r⌽,t=y # x)←t
    (≢State)=n←$tι⍨State∧.=x?addnode⁝x addpath [α ω] ◇ ¯1ₓ;
    α∊i←⍸n∊¨Path?¯1ₓ;
    0ₓ=≢i?n addpath [α ω] ◇ ¯1ₓ;
    α⌷NPath ≥ ≤⌊/Path.[i]ι¨n?¯1ₓ;
    (n addpath [α ω])modpath¨i
  }
  addnode←{
    State⍪←ω
    i←ω.[[Target]⌷(≤i),(≢ω)-(i←Shape₁),1]ι↑Tiles
    ENode,←≥i × i<≢Target
    NMoves,←∞
    ≤≢ENode
  }
  addpath←{
    [p m]←ω ◇ e←α⊃ENode ◇ n←≢p⊃Path
    e>0ₓ ∧ e∊Fin ∧ n ≥ Finιe ⊃ NPath,0ₓ?¯1ₓ;
    Path,←⊂(p⊃Path),α
    Moves,←⊂(p⊃Moves)⍪m
    Fin,←α⊃ENode
    NPath,←n
    (α⌷NMoves)⌊←1×n
    ≤≢Path
  }
  modpath←{
    α=¯1ₓ?¯1ₓ;
    (i←≥xι¯1⊃α⊃Path)=≢x←ω⊃Path?α;
    (ω⊃Path)←α⊃Path , i↓x
    (ω⊃Moves)←α⊃Moves ⍪ i↓ω⊃Moves
    (ω⊃NPath)←≤≢ω⊃Path
    ¯1ₓ
  }
  delpath←{
    ~$f∊i←~(ι≢Path)∊ω?0;
    (⍬⊃Path)←i#Path
    (⍬⊃Moves)←i#Moves
    (⍬⊃NPath)←i#NPath
    (⍬⊃Fin)←i#Fin
    0
  }
  α←ι3ₓ ◇ Target←,α
  Tiles←∪,ω ~ " " ◇ Shape←ρω
  Path←,⊂,0ₓ ◇ Moves←,⊂1 2ρ' ' ◇ NPath←,0ₓ ◇ Fin←,0ₓ
  State←1,≢,ω ρ ω ◇ NMoves←,0 ◇ ENode←,0ₓ
  x←extend0
  ∧/Fin=0ₓ?"There are no solutions";
  i←∨/{i#⁻n=⌊/n←(i←Fin=ω)#NPath}¨≥ι≢Target
  x←[[≤i#Fin]⌷Target]⌷"Top-right" "Bottom-left" "Bottom-right"
  x←[x◇(⊂"in "),¨(0⍕¨i#NPath),¨⊂" moves"]
  x⍪i#Moves
}

⍝ From https://dfns.dyalog.com/c_refmt.htm
refmt←{  ⍝ Indent code and align comments.
  α←4 ◇ [dent csep]←α
  unqt←{~≠\ω='''}
  [code coms]←⍉{
    umsk←unqt ω ◇ cmsk←∨\umsk∧ω='⍝'
    [code coms]←$f $t=⊂cmsk #¨ ⊂ω
    rlb←{∨\' '≠ω # ω}
    clean←⌽ rlb ⌽ rlb code
    [clean coms]
  }¹ ⊃ω
  dents←{
    toks←unqtω {α#⁻α#ω} ω
    dvec←-⌿0 1⌽+\"{}"=⊗toks
    lmask←toks='·'
    dent×lmask#dvec #¨ ' '
  },/'·',¨code
  scoms←⊂csepρ' ' ,¨ coms
  icode←↓⊃dents,¨code
  rtb←⌽∘{∨\ω≠' ' # ω}∘⌽
  qnr←icode,↢rtb¨scoms
  ⊃⍣ 1ₓ=≡ω qnr
}

⍝ From https://dfns.dyalog.com/c_box.htm
box←{  ⍝ Frame a text array, with optional internal row and column borders.
  α←⍬ ⍬ 0 ◇ ar←{ω,(≢ω)↓⍬ ⍬ 0}{2ₓ>≡ω?,⊂,ω;ω}α
  ch←{ω?"++++++++-|+";"┌┐└┘┬┤├┴─│┼"}1=ar₂
  rh←×/¯1↓ρω , ¯1↑1,ρω ◇ z←rhρω
  0ₓ∊ρ∊2↑ar?q←ch₈⍪(ch₉,z,ch₉)⍪ch₈ ◇ [0 ¯1;0 ¯1]⌷q ←2 2ρch ◇ q;
  [r c]←rh{∪α{ω∊ι≥α # ω}ω,(~¯1ₓ∊ω)#0ₓ,α}¨,¨2↑ar
  [rw cl]←rh{<ω∪0ₓ,α}¨[r c]
  (~ 0ₓ,rh₁ ∊ c){
    (↑α)↓₁(-α₁)↓₁[: ⍋(≥ιrh₁),cl]⌷ω
  }(~ 0ₓ,rh₀ ∊ r){
    (↑α)↓₀(-α₁)↓₀[⍋(≥ιrh₀),rw]⌷ω
  }{
    [h w]←≢rw , ≢cl ◇ q←[h w]ρch₁₀
    hz←h,rh₁ ρ ch₈
    vr←rh₀,w ρ ch₉
    ∨/0ₓ∊¨ρ¨[rw cl]?ω⍪hz , vr⍪q;
    q₀←ch₄ ◇ q.[: ¯1]←ch₅ ◇ q.[: 0]←ch₆ ◇ q₋₁←ch₇
    [0 ¯1;0 ¯1]⌷q ←2 2ρch ◇ ω⍪hz , vr⍪q
  }z
}

⍝ From https://dfns.dyalog.com/c_sudoku.htm
sudoku←{  ⍝ All solutions, with square or α-shaped groups.
  wid←≢ω ◇ α←√wid ◇ grp←2ρα ◇ set←≥ιwid
  inx←↓(,wid⊥¨grp↣×¨ι⌽grp)+⊗,wid⊥¨ιgrp
  inx,←{(↓ω),↓ωᵀ}(ρω)ριwid²
  bas←≢,ω ρ ⊂set
  ,ω ↣{q←[ω]⌷α ◇ (q=0ₓ # q)←⊂set~,/q ◇ bas.[ω]∩←,¨q}¨inx
  ∆sqz←{
    0ₓ=≢ω?α;i←↑ω ◇ q←α
    b←1ₓ<≢¨[i]⌷α ◇ ~∨/b?α ∇ 1↓ω;
    q.[b#i]←{c←ω⊃q ◇ z←c~,/[i~[ω]]⌷q ◇ 1ₓ≠≢z?c;z}¨b#i
    r←{
      s←≢¨ω ◇ 2ₓ>+/s=2ₓ?s;∧/s∊[1 2]ₓ?s;
      k←s=2ₓ {↑ ⍸α ~ ωι∪α#ω} ω ◇ k=0ₓ?s;
      c←~ω∊[k⊃ω] ◇ q.[c#i]←c#ω ~¨ ⊂k⊃ω ◇ ≢¨[i]⌷q
    }[i]⌷q
    0ₓ∊r?⍬;∧/r=1ₓ ≥ 1ₓ∊r?q ∇ 1↓ω;
    j←r>1ₓ # i ◇ q.[j]←[j]⌷q ~¨ ⊂,/[i~j]⌷q
    q ∇ 1↓ω
  }
  ∆chk←{m←ω ∆sqz inx ◇ 0ₓ=≢m?⍬;m≢ω?∇ m;ω}
  ∆nxt←{
    0ₓ=≢ω?ω;
    r←≢¨ω ◇ ∧/r=1ₓ?⊂ω;
    j←,/{[⍒{+/1ₓ=[ω]⌷r}¨ω]⌷ω}wid↑inx
    i←r.[j]ι⌊/r~[1ₓ] ⊃ j
    ω↣{m←α ◇ (i⊃m)←,ω ◇ m}¨i⊃ω
  }
  ⍬{
    0ₓ=≢ω?α;
    m←∆chk↑ω ◇ 0ₓ=≢m?α ∇ 1↓ω;
    ∨/1ₓ<≢¨m?α ∇ 1↓ω,(∆nxt m);
    ∨/ +/set ≠ {+/,/[ω]⌷m}¨inx?α ∇ 1↓ω;
    (α,⊂ 2ρwid ρ ,/m) ∇ 1↓ω
  }{0ₓ=≢ω?ω;⊂ω}(bas ∆sqz inx)
}

⍝ From https://dfns.dyalog.com/c_kt.htm
kt←{  ⍝ Knight's tours; α limits the number of solutions.
  α←1ₓ ◇ sreq←α ◇ nsqs←×/ω
  kdef←,0 1⌽⊗[1 ¯1]ₓ,⊗[2 ¯2]ₓ
  net←⊂,ιω ∩¨ ↓ ιω +⊗ kdef
  ι↢(ιω)¨⍬{
    sreq=≢ω?ω;
    path←⍶,⊂α
    nsqs=≢path?ω,⊂path;
    nxt←α⊃⍹
    0ₓ=≢nxt?ω;
    net←⍹~¨⊂[α]
    ord←[⍒≢¨[nxt]⌷net]⌷nxt
    ω(path ⍢ net)/ord
  }net/(⌽,ιω),⊂0ρ⊂⍬
}

⍝ From https://dfns.dyalog.com/c_queens.htm
queens←{  ⍝ N-queens solutions up to rotation and reflection.
  search←{
    ⊂⍬ ∊ ω?0ρ⊂⍬;
    0ₓ=≢ω?rmdups α;
    [hd tl]←[↑ω 1↓ω]
    next←α↣,¨hd
    rems←hd free¨ ⊂tl
    ,/ next ∇¨ rems
  }
  cvex←≥ιω × ⊂[¯1 0 1]ₓ
  free←{ω~¨α+(≢ω)↑cvex}
  rmdups←{
    rots←⍒⍣ ι4ₓ
    refs←⍋⍣0 1
    best←{↑⍋⊃ω ⊃ ω}
    syms←,/refs¨rotsω
    ω≡best⁝syms ⊃ [⍬ ,⊂ω]
  }
  fmt←{
    chars←[(⊃ω)=⊗ια]⌷"·⊛"
    expd←1↓,⊃αρ⊂0 1
    ⊂² expd#⁻₋₁chars
  }
  squares←[ι⌈ω÷2],1↓ωρ⊂ιω
  ω fmt ⍬ search squares
}

⍝ From https://dfns.dyalog.com/c_ary.htm
ary←{  ⍝ Radix representation, including recurring fractional digits.
  ω=0?"0";
  base←|α
  [sign abs]←[×ω |ω]
  exp←⌈base⊛abs
  man←abs×base*-exp
  [p q]←⌊{[ω 1ₓ]÷1ₓ∨ω}man
  digs←{
    ω=0ₓ?[(↑↓⍉[0ₓ q]⊤α) ⍬ 1];
    lim<≢α?[(rnd↑↓⍉[0ₓ q]⊤α) ⍬ 0];
    ω∊α?αrepω , 1;
    α,ω ∇ base×q|ω
  }
  ofmt←{
    [sig exp fix rep rat]←ω
    fmt←{[ω]⌷$d,$a}
    [lft pad]←0⌈1 ¯1×exp
    zro←{ω,(≢ω)↓0ₓ}
    neg←sig<0 # '¯'
    nnn←fmt zro lft↑fix,lftρrep
    fff←fmt(pad#0ₓ),lft↓fix⌊≤base
    rrr←fmt(0⌈exp-≢fix)⌽rep
    fffrrr←fff opt rrr
    dot←""≡fffrrr ↓ '.'
    etc←×≢rep # "..."
    cut←rat↓'?'
    neg,nnn,dot,fffrrr,etc,cut
  }⍣ α>0ₓ
  lim←⌈base⊛÷1e¯14⌈2*¯53
  rnd←{¯2↓ω , +↢(5ₓ↣≤)/¯2↑ω}
  rep←{$f $t=⊂∨\ω=α #¨ 1↑↓⍉[0ₓ q]⊤α}
  opt←{-+/∧\=⌿⊃⌽¨[α ω] ↓ α,ω,ω}
  q=1ₓ?ofmt [sign ≥exp ,1ₓ ⍬ 1];
  ofmt [sign exp],(⍬ digs p×base)
}

⍝ From https://dfns.dyalog.com/c_pack4.htm
pack_4←{  ⍝ Quad-tree packing; α=0 expands, α>1 allows lossy packing.
  cmp←{
    uniq←∪,ω
    rep←{ω{ω#α-¯1⌽~ω}~$t $t $f⍷ω<0ₓ}⍣≡
    ρω {[uniq α ω]} rep α {
      ~$f∊,ω∊[↑,ω]?↑,ω;
      α≥≢∪,ω?maxx+/ ι≢uniq =⊗ ,ω;
      ax←maxxρω
      size←ax⊃ρω
      mask←$t,1↓(ιsize)=⌊size÷2
      ¯1ₓ,,/(α ∇¨ mask⊂⍠ax ω)
    } uniqιω
  }
  exp←{
    [uniq shape stream]←ω
    split←{{[⌊ω ⌈ω]}ω÷ₐ≥maxkω}
    maxk←{ι≢ω = maxx ω}
    exp←{≥|0ₓ⌊≥ω⌊0ₓ # ¯1ₓ⌈ω}
    i←↑shape{
      [head tail]←ω
      0ₓ≤head?[αρhead tail];
      [ls rs]←split α
      [lsub lrem]←ls ∇ tail
      [rsub rrem]←rs ∇ lrem
      subs←lsub,⍠(maxx α) rsub
      [subs rrem]
    }{[α ω]}/(exp stream)
    [i]⌷uniq
  }
  maxx←↑⍒
  α←1ₓ ◇ α>0ₓ?α cmp ω;exp ω
}

⍝ From https://dfns.dyalog.com/c_von.htm
von←{  ⍝ Capitalise names, preserving prefixes and user-supplied exceptions.
  α←""
  [pre not]←{0ₓ=≢ω?"" "";3ₓ>≡ω?[ω ""];ω}α
  tx←¯1•c,ω,' '
  q←"ab af al an and ap as av da de dei della der di"
  q,←" el ja la le och of the und van ver von zu y"
  q←{' ',¨(ω≠' ' ⊆ ω),¨' '}q
  not,←" al-" " c/o" " d'" " de'" " el-" " l'" " s'" " t'"
  ∆pre←{-≢α {ω∨α⌽ω} α⍷ω}
  ∆not←{∨⌿⊃(-≥ι≤≢α)⌽¨⊂α⍷ω}
  ∆fix←{
    0ₓ=≢α?$f;
    q←¯1•c{2ₓ>≡ω?,⊂,ω;ω}α
    1↓∊∨/q⍶¨⊂' ',ω
  }
  b←¯1↓∊∨/q⍷¨⊂' ',tx < $t,>/⌽¹↕~tx∊" -:;.,!?'""/)("
  b∨←pre(∆pre ∆fix)tx
  b>←not(∆not ∆fix)tx
  ¯1↓(1↣•c)@(⍸b) tx
}

⍝ From https://dfns.dyalog.com/c_packH.htm
packH←{                      ⍝ Huffman packing.

    compress←{                          ⍝ compression.
        shape←ρω ◇ vect←,ω              ⍝ shape and items of array.
        uniq←∪vect                      ⍝ unique items.
        freq←≢⊢ ⌸vect                  ⍝ frequency of occurrence.

        tree←{                          ⍝ Huffman tree.
            1ₓ=ρω?↑⌽↑ω;                  ⍝ list exhausted: done
            nxt←[2↑⍋↑¨ω]⌷ω                ⍝ next two lowest frequencies,
            [freqs items]←↓⍉⊃nxt        ⍝ and corresponding items.
            ∇(ω~nxt),⊂(+/freqs),⊂items  ⍝ collect 2 most infrequent items.
        }↓⍉⊃[freq uniq]                 ⍝ tree from frequency-item pairs.

        [items csegs]←↓⍉⊃⍬{             ⍝ Dictionary: items ←→ code-segments.
            ⍬≡ρω?⊂[ω α];                 ⍝ all done: item and binary code.
            ,/(α↣,¨0 1)∇¨ω              ⍝ extended codes for sub-trees.
        }tree                           ⍝ from frequency tree.

        bits←∊[itemsιvect]⌷csegs          ⍝ bit string of tree indices.

        [leaves depths]←↓⍉⊃tree{        ⍝ tree leaves and depths.
            ⍬≡ρα?⊂[α ω];                 ⍝ leaf: leaf and depth.
            ,/ α ∇¨ ≥ω                ⍝ branch: traverse deeper sub-branches.
        }0ₓ                             ⍝ starting at depth 0 for the root.

        [shape leaves depths bits]      ⍝ compressed structure.
    }

    expand←{                            ⍝ expansion.
        [shape leaves depths bits]←ω    ⍝ compressed structure.

        tree←1⊃↑{                       ⍝ reconstituted Huffman tree.
            ↑α ≠ ↑↑ω?[α],ω;              ⍝ distinct adjacent depths: continue.
            [≤↑α [α₁ 1⊃↑ω]] ∇ 1↓ω      ⍝ identical   ..      ..  : coalesce.
        }/(↓⍉⊃[depths leaves]),¯1ₓ      ⍝ depths, leaves with trailing dummy.

        picks←⍬{                        ⍝ tree index vectors.
            ⍬≡ρω?⊂α;                     ⍝ leaf: done.
            ,/(α↣,¨0 1)∇¨ω              ⍝ visit each branch,
        }tree                           ⍝ ··· of the tree.

        maxd←≡tree                     ⍝ max depth.

        dict←,/{                       ⍝ lookup dictionary.
            item←tree⊃/⌽ω               ⍝ next tree leaf.
            indl←≢ω                   ⍝ trailing index length.
            2*maxd-indl ρ ⊂[indl item]  ⍝ extended pick vectors.
        }¨picks                         ⍝ ··· for each index vector.

        iwin←ιmaxd                      ⍝ input index window.
        ibuff←bits,maxdρ0               ⍝ padded input buffer.

        shapeρ(,shapeρleaves){          ⍝ restore original shape.
            [ix ox]←ω                   ⍝ input and output indices.
            ox=≢α?α;                     ⍝ end of output, done
            nxt←⊥[ix+iwin]⌷ibuff        ⍝ next dictionary index.
            [skip item]←nxt⊃dict        ⍝ number of bits and output item.
            item@oxα ∇ ω+[skip 1ₓ]   ⍝ traverse input and output buffers.
        }[0 0]ₓ                           ⍝ initial input/output buffer indices.
    }

    α←$t ◇ α?compress ω;expand ω       ⍝ compress or expand.
}

⍝ From https://dfns.dyalog.com/c_lisp.htm
 lisp←{             ⍝ Evaluator for a subset of Scheme.

    ⍝ Parser functions take as arguments:
    ⍝   α ⍝ the input string
    ⍝   ω ⍝ the current parse position in α
    ⍝ and return a triplet of:
    ⍝   $f position AST     ⍝ on success
    ⍝   $t position message ⍝ on failure

     parse←{
         ω≥≢α?[$t ω "unexpected eof"];
         ' '≡ω⌷α?α ∇ ≥ω;
         '('≡ω⌷α?α parseList ≥ω;
         ω⌷α ∊ $d?α parseNum ω;
         '''≡ω⌷α?α parseQuote ω;
         α parseAtom ω
     }

     parseList←{ ⍝ parse the inside of a list and consume the ')'
         ω≥≢α?[$t ω "unexpected eof"];
         ')'≡ω⌷α?[$f ≥ω ⍬];
         [e p h]←α parse ω
         e?[e p h];
         [e p t]←α ∇ p
         e?[e p t];
         [$f p [h],t]
     }

     parseNum←{
         l←20ₓ ⍝ 1 + the maximum length of a numeric literal
         n←+/∧\$d∊⍨l↑ω↓α ⍝ actual length of the literal
         n≡l?[$t ω "numeric literal too long"];
         [$f ω+n ⍎n↑ω↓α]
     }

     parseAtom←{
         la←20ₓ ⍝ 1 + the maximum length of an atom
         na←$d,"() '" ⍝ non-atom characters
         l←+/∧\~na∊⍨la↑ω↓α
         l≡la?[$t ω "atom too long"];
         [$f ω+l l↑ω↓α]
     }

     parseQuote←{
         [e p r]←α parse ≥ω
         e?[e p r];
         [e p ["quote" r]]
     }

    ⍝ Evaluator

     isAtom←{' '≡↑0ρω ∧ 1ₓ=ρρω}
     isNum←{0≡↑0ρω ∧ 0ₓ=ρρω}

     eval←{ ⍝ α: environment, ω: expression
         isNum ω?ω;
         isAtom ω?[α.[: 0]ι⊂ω 1]⌷α;
         0ₓ≡≢ω?⍬;
         "quote"≡↑ω?ω₁;
         1↑ω ∊ ,¨'\' "lambda"?["closure" α],1↓ω;
         "cond"≡↑ω?α evcond 1↓ω;
         α∇↑ω apply (⊂α)∇¨1↓ω
     }

     apply←{ ⍝ α: procedure, ω: arguments
         "closure"≡↑α?[α₂◇ω]ᵀ⍪α₁ eval α₃;
         ⍎(↑α),'ω'
     }

     evcond←{ ⍝ α: environment, ω: clauses
         0ₓ≡≢ω?⍬;
         "else"≡↑↑ω?α eval ↑1↓↑ω;
         0 ≡ αeval↑↑ω?α ∇ 1↓ω;
         α eval ↑1↓↑ω
     }

    ⍝ Initial environment
     env←⍉⍪["+" ,⊂"+/"]
     env⍪←["-" ,⊂"-/"]
     env⍪←["*" ,⊂"×/"]
     env⍪←["=" ,⊂"=/"]
     env⍪←["write" ,⊂"⎕←"]

     [e p x]←ω parse 0   ⍝ e: failure flag, p: position, x: AST
     e?x;
     α←$t ◇ ~α?x;        ⍝ ~α: parse only
     env eval x
 }

⍝ From https://dfns.dyalog.com/c_Cut.htm
 Cut←{
 ⍝ Model of cut.  Works in either index-origin.
 ⍝ Iverson, K.E., Rationalized APL, 1983, Section K. http://www.jsoftware.com/papers/RationalizedAPL.htm
 ⍝ Hui, R.K.W., Some Uses of { and }, 1987, Section 3.2. http://www.jsoftware.com/papers/from.htm
 ⍝ Iverson, K.E., A Dictionary of APL, 1987, m⍤v. http://www.jsoftware.com/papers/APLDictionary.htm
 ⍝ Hui, R.K.W., and K.E. Iverson, J Introduction and Dictionary, 2011, Cut (;.). http://www.jsoftware.com/help/dictionary/d331.htm

     α←|⍹ {                          ⍝ default left argument
         0ₓ=α?⍉0,⍪-ρω;
         1ₓ=α?0ₓ = 1↑ω ι ω;
         2ₓ=α?0ₓ = ¯1↑ω ι ω;
         3ₓ=α?ρρω ρ ⌊/ρω
     } ω

     α ⍶{⍶ ω}{

         Under←{⍹⁻ ⍹α ⍶ ⍹ω}    ⍝ dyadic case of "under" operator
         p_2←{(¯1⌽k↓α)⊂⍠⍶(⍶ρ0 , k←-$tι⍨⌽α)↓ω}
         Blk_1←{0ₓ=≢α?ω;0ₓ=≢↑α?1↓α (≥⍶)⍢ ω;1↓α (≥⍶)⍢ ⊃(⊂↑α)⊂⍠⍶¨ω}
         Blk_2←{0ₓ=≢α?ω;0ₓ=≢↑α?1↓α (≥⍶)⍢ ω;1↓α (≥⍶)⍢ ⊃(⊂↑α)(⍶ p_2)¨ω}
         ifv←{(⍉α,⍪)⍣ 2ₓ>≢ρω ω}         ⍝ laminate α if ω is vector or unit
         ci←{[i s]←↓(1 ifv α) ◇ (0ₓ>s){⌽⍣α ω}¨(ι¨|s)+(i |⍨ ρi ↑ ρω)-(0ₓ>i)×≤|s}
         ti←{[m s]←↓(1 ifv α) ◇ ⊃{⊃[ω (×s)×(|s)⌊r-ω]}¨m↣×¨ι⌈r÷m+(0ₓ=m)×r←(ρm)ρρω}
         tj←{[m s]←↓(1 ifv α) ◇ ⊃{⊃[ω s]}¨m↣×¨ι⌈(≥r-|s)÷m+(0ₓ=m)×r←(ρm)ρρω}

         1ₓ=|⍹ ∧ ⍬≡α ∨ 1ₓ≠≡α?⊃⍶¨(⊂ 0ₓ>⍹ × ×≢¨α)↓¨α(0 Blk_1)⊂ω;
         2ₓ=|⍹ ∧ ⍬≡α ∨ 1ₓ≠≡α?⊃⍶¨(⊂ 0ₓ>⍹ × -×≢¨α)↓¨α(0 Blk_2)⊂ω;

         0ₓ=⍹?α(⍶ ci⌷⊢)ω;
         1ₓ=⍹?⊃α(⍶¨⊂₀)ω;
         ¯1ₓ=⍹?⊃ α ⍶∘(1↣↓)¨⊂₀ ω;
         2ₓ=⍹?α (⍶∘⊖⍢1)Under⊖ ω;
         ¯2ₓ=⍹?α ⍶∘(¯1↣↓)⍢2 ω;
         3ₓ=⍹?αtiω (⍶⍢0)⍤2 ∞ ω;
         ¯3ₓ=⍹?αtjω (⍶⍢0)⍤2 ∞ ω
     }⍹ ω
 }

⍝ Dyalog 20.0.53963.0 ⎕AV and name-start alphabet; codecs retain their original wire format.
dfnsAV←•ucs 0 8 10 13 32 12 6 7 27 9 9014 619 37 39 9082 9077 95 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 1 2 175 46 9068 48 49 50 51 52 53 54 55 56 57 3 8866 165 36 163 162 8710 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 4 5 253 183 127 9049 193 194 195 199 200 202 203 204 205 206 207 208 210 211 212 213 217 218 219 221 254 227 236 240 242 245 123 8364 125 8867 9015 168 192 196 197 198 9064 201 209 214 216 220 223 224 225 226 228 229 230 231 232 233 234 235 237 238 239 241 91 47 9023 92 9024 60 8804 61 8805 62 8800 8744 8743 45 43 247 215 63 8714 9076 126 8593 8595 9075 9675 42 8968 8970 8711 8728 40 8834 8835 8745 8746 8869 8868 124 59 44 9073 9074 9042 9035 9033 9021 8854 9055 9017 33 9045 9038 9067 9066 8801 8802 243 244 246 248 34 35 30 38 180 9496 9488 9484 9492 9532 9472 9500 9508 9524 9516 9474 64 249 250 251 94 252 96 8739 182 58 9079 191 161 8900 8592 8594 9053 41 93 31 160 167 9109 9054 9059
dfnsLetters←"_abcdefghijklmnopqrstuvwxyz∆ABCDEFGHIJKLMNOPQRSTUVWXYZ⍙ÁÂÃÇÈÊËÌÍÎÏÐÒÓÔÕÙÚÛÝþãìðòõÀÄÅÆÉÑÖØÜßàáâäåæçèéêëíîïñóôöøùúûü"

⍝ From https://dfns.dyalog.com/c_words.htm
words←{  ⍝ Split a string into words and intervening text.
  α←[dfnsLetters $d]
  1ₓ=≡,α?[α ""]∇ω;
  [alph supp]←α
  w←$f{ω∊alph ∨ α∧ω∊supp}\ω
  ≥+\w≠¯1⌽w ⊆ ω
}

⍝ From https://dfns.dyalog.com/c_eis.htm
eis←{{⊂1#ω}⍣ 1ₓ=≡,ω ω}⊘{eisα , eisω}  ⍝ Enclose if simple; with two arguments, link them.

⍝ From https://dfns.dyalog.com/c_iotag.htm
 iotag←{                        ⍝ Generalized ι.
     AlphaInterval←{ω=' '?"";         ⍝ ι' ' is empty vector.
         a←dfnsAVι"a0AÁ"                ⍝ Indices of a0AÁ.
         k←a+.≤i←dfnsAVιω             ⍝ Find starting point.
         α←(≤k ⌷ a)⌷dfnsAV               ⍝ Default left argument.
         </i,j←dfnsAVια?⌽ ω ∇ α;         ⍝ If ω before α, reverse.
         j↓ ≥i ↑ dfnsAV             ⍝ Truncate dfnsAV.
     }
   ⍝Interval←{α+0,+\(|d)ρ×d←ω-α}    ⍝ Interval Function.
     Interval←{s←×/1↓ω,(α>↑ω)#¯1ₓ    ⍝ Calculate step size
         α+s×ι⌊1ₓ-(α-↑ω)÷s}        ⍝ Generate Interval
     IndexOf←{i←1↓ιρρα               ⍝ Enclose left arg axis.
         j←1-ρρα ↑ ιρρω              ⍝ Enclose right arg axis.
         m←[i]⌷ρα ⌈ [j]⌷ρω         ⍝ Pad both arguments.
         (⊂⍠i m↑⍠i α)ι⊂⍠j m↑⍠j ω           ⍝ Get index.
     }
     ischar←{' '≡↑0ρ,ω}          ⍝ character fill.
     m←$f ◇ α←m←$t                    ⍝ Monadic?
     m∧1ₓ=ρρω?,⊗/∇¨ω;               ⍝ Vector right argument
     m∧ischar ω?AlphaInterval ω;      ⍝ Alpha Monadic.
     m?×ω × ι|ω;                      ⍝ Integer Monadic.
     s←0ₓ=ρρα                        ⍝ Unit Left Argument?
     s∧ischar ω?α AlphaInterval ω;    ⍝ Alpha Interval.
     s?α Interval ω;                  ⍝ Numeric Interval.
     α IndexOf ω                     ⍝ Dyadic.
 }

⍝ From https://dfns.dyalog.com/c_tokens.htm
 tokens←{                           ⍝ Lex of APL src line.
     α←0 ◇ nv←α                          ⍝ numeric vectors as single token.
     alph←dfnsLetters         ⍝ alphabet for names.
     all←{+/∧\α∊,ω}                       ⍝ No. of leading α∊ω.
     acc←{α,⊂↑/ω lex ↓/ω}                ⍝ accumulated tokens.
     size←{                                  ⍝ Length of the token at the start of ω.
         hd←↑ω
         hd=' '?ω all ' ';                   ⍝ White Space.
         hd∊alph?ω all alph,$d;              ⍝ Name
         hd='⎕'?1 + 1↓ω all alph;          ⍝ System Name
         hd='''?+/∧\{ω∨¯1⌽ω}≠\hd=ω;          ⍝ Char literal
         hd∊$d,'¯'?max←ω all $d,".¯EJ",nv#' ' ◇ max-+/∧\' '=⌽max↑ω;  ⍝ Numeric literal, without trailing blanks.
         hd∊"αω∇:"?ω all hd;                 ⍝ αα or ωω or ∇∇ or ::
         hd='⍝'?ρω;                          ⍝ Comment
         1                                   ⍝ Single char token.
     }
     lex←{0ₓ=ρω?α;α acc [(size ω) ω]}       ⍝ Next token, else finished.
     0ρ⊂"" lex ,ω
 }

⍝ From https://dfns.dyalog.com/c_ssword.htm
 ssword←{                   ⍝ Approx alternative to xutils' ss.
     [srce find repl]←,¨ω        ⍝ source, find and replace vectors
     alph←dfnsLetters ⍝ primary alphabet: initial letters for names
     supp←$d                     ⍝ supplementary: 0-9
     α←[alph supp]               ⍝ default left argument
     ∊(⊂repl)@[{                  ⍝ replace find with repl
         ω∊[find]                 ⍝ matches
     }] αwords⁝srce             ⍝ source into words.
 }

⍝ From https://dfns.dyalog.com/c_packD.htm
 packD←{                                 ⍝ Pack char array to boolean vector.

     key←,/{↓ ιω >⊗ ιω}¨≥ι23ₓ             ⍝ generate the keys (up to 276), they
                                        ⍝ start with ones and end with zeros.
     cmp←{                               ⍝ compress:
         dt←,ω ◇ k←$t,¨key               ⍝ vectorise items, separators to keys
         u←byfreq dt                     ⍝ sort uniques according to frequencies.
         uq←,(8ρ2)⊤(≤ρu),dfnsAVιu        ⍝ shape then uniques fit to eight bits.
         rk←,(4ρ2)⊤ρρω                   ⍝ rank fits in four bytes

         sh←⍬{                           ⍝ shape is a trickier one
             0ₓ∊ρω?α;                     ⍝ all done?
             n←≥⌊2⊛1⌈↑ω                 ⍝ how many bits are needed for representing the integer?
             q←(, 5ρ2 ⊤ n),,(nρ2)⊤↑ω      ⍝ n should fit to five bits (2*2*5 -> 4GB dimensions supported ;) )
             α,q ∇ 1↓ω                 ⍝ add binarised figure to the result
         }ρω

         0≠rk,sh,uq,∊[uιdt]⌷k             ⍝ join the result with binarised data, as Booleans
     }

     unc←{                               ⍝ uncompress:
         rk←⊥4ρω                        ⍝ rank's easy

         [sh x]←[⍬ 4 rk]{                ⍝ shape needs more treatment
             [s n r]←α                   ⍝ deliver args
             r=0ₓ?[s n];                  ⍝ all done: return shape and offset
             q←⊥5ρω                     ⍝ how many bits?
             d←⊥qρ5↓ω                   ⍝ -> dimension length
             [s,d n+q+5 ≤r] ∇ q+5 ↓ ω   ⍝ next
         }4↓ω

         n←≥⊥8ρx↓ω                     ⍝ how many uniques?
         u←[,⊥[n 8]ρ(n×8)↑(x+8)↓ω]⌷dfnsAV   ⍝  -> make the list
         dt←x+8×≥n ↓ ω                  ⍝ rip the data part
         p←dt≤¯1↓$f,dt                   ⍝ binary partitioner (first 1's)
         shρ[keyιp⊆dt]⌷u                  ⍝ reconstruct
     }

     α←$t ◇ α?cmp ω;unc ω              ⍝ which way to go?
 }

⍝ From https://dfns.dyalog.com/c_packT.htm
 packT←{                        ⍝ Simple text vector packager.

     cmp←{
         ec←{c←3↑dfnsAV ◇ ↑⍋+/c=⊗ω ⌷ c},ω           ⍝ escape: the least frequent of the first three dfnsAV chars

         lv←1ₓ,≠/↕,ω ◇ e←ec=,ω                ⍝ repeat lengths
         mask←e<¯1↓[1 0 0 1]ₓ⍷lv,1ₓ ◇ (mask#1⌽lv)←1ₓ   ⍝ remove 2's and 3's
         mask←e<¯1↓[1 0 1]ₓ⍷lv,1ₓ ◇ (mask#1⌽lv)←1ₓ
         ln←{-⍨/↕(ω,1)#ι≥ρω}lv              ⍝ packables

         (lv#lv)←≥⌊ln÷256ₓ                  ⍝ adjust for 256 length

         ln←∊{ω<257ₓ?ω;                       ⍝ now we have to adjust the long
             q←256ₓ|ω ◇ n←⌊ω÷256ₓ            ⍝ sequences so that the last
             {                               ⍝ sequence will be
                 3ₓ<¯1↑ω?ω;                   ⍝ properly done, ie:
                 ¯2↓ω , [¯4 4]ₓ+¯2↑ω         ⍝   257 -> 252 5,
             } n#256ₓ , ω-n×256ₓ              ⍝   NOT    256 1.
         }¨ln

         q←lv#,ω                             ⍝ reduce data
         b←ln>3ₓ ∨ {↑ω , </↕ω}q=ec           ⍝ find places where
         (b#q)←b#q {ec,α,ω}¨ [≤b#ln]⌷dfnsAV       ⍝  to put pack sequences
         ec,∊q                               ⍝ remove nesting
     }

     exp←{
         ec←↑ω                               ⍝ "esc"
         ~ec∊1↓ω?1↓ω;                         ⍝ packed?

         bv←{↑ω , </↕ω}1↓ω=ec                ⍝ reduce vector
         ln←0ₓ@(⍸¯2⌽bv)(≥dfnsAVι ¯2⌽bv # 1↓ω)@(⍸¯1⌽bv)~bv  ⍝ lengths: 0 for escapes and counts, the count for each packed character
         ln#1↓ω                              ⍝ do it!
     }

     α←$t ◇ α?cmp ω;exp ω                  ⍝ compress or expand.
 }
⍝ From https://dfns.dyalog.com/c_parse.htm
 parse←{  ⍝ Bunda-Gerth parsing.

     [opt defs]←ω≡"" {[α ω]}⍣ ~0≡↑0ρα α     ⍝ trace/format option and defns.

     defn←{                                      ⍝ binding table definition.
         words←⊂" " segs¨ ↑¨ ⊂"⍝" segs¨ ω         ⍝ blank-delimited words.
         pop←{[↑ω 1↓ω]}                          ⍝ head & tail of list.
         sects←[0ρ⊂""] segs words               ⍝ sections.
         [csect defs]←pop sects                  ⍝ separation of sections.
         bindx←↑¨csect ι ⊂"()"                   ⍝ index of bracket defn.
         bsegs←bindx⊃csect,⊂,⊂"()"               ⍝ bracket pairs: () [] <> ...
         [bkts blabs]←↓⍉⊃{[⌽2↑ω 2↓ω]}¨¯1⌽¨bsegs  ⍝ brackets and their cat labels.
         [cats reps]←↓⍉⊃pop¨csect~[bsegs]         ⍝ categories & representatives.
         bonds←⌽≥ι≢defs                         ⍝ binding strengths
         bmat←cats{0ₓ}⊗cats                     ⍝ initialised binding table
         defn←{≥cats↣ι¨ ⊂"." segs¨ ":→"segsω}   ⍝ split a.b:c.d→z defn
         dist←{,,⊗/2↑ω ,¨ ↑⌽ω}                 ⍝ distribution of productions
         lmat←dist∘defn¨¨¨defs                   ⍝ loading matrix
         bftz←,/bonds,¨¨((,/)⍣2)¨lmat           ⍝ bond-from-to-rslt tuples
         [cats reps bkts blabs],⊂₀ ₁⊃{        ⍝ binding structure.
             [bond fm to rslt]←α                 ⍝ binding and resulting cats.
             (⊂[bond rslt])@ ⊂≤[fm to] ω       ⍝ populate cell
         }/bftz,⊂bmat                            ⍝ loaded binding matrix.
     }∘{                                         ⍝ pre-process alias=... lines.
         lines←↓' ',⊃ω                       ⍝ lines from char array.
         wds←↑¨ ⊂" " segs¨ lines                  ⍝ first word from each line.
         [msk nsk]←$t $f=⊂'='∊¨wds               ⍝ mask of alias lines.
         dict←⊂"=" segs¨ msk#wds                 ⍝ (fm to) substitution pairs.
         {_array.subs/dict,⊂ω}¨nsk#lines        ⍝ lines with expanded aliases.
     }⍣ 2ₓ≠≢ρ↑⌽defs                            ⍝ compile unless compiled.

     table←{                                     ⍝ formatted Bunda-Gerth table.
         [cats bmat zmat]←ω                      ⍝ categories and binding matrix.
         fmt←{×α # ⍬⍕α , ' ',ω⊃cats,'?'}          ⍝ bond and category.
         ttl←{[' ' α],.⍪[α ω]}                   ⍝ row and column headers.
         cats ttl bmat fmt¨ ≥≢cats | ≤zmat     ⍝ formatted bonds & categories.
     }

     [cats reps bkts blabs bmat zmat]←defn defs  ⍝ definition structure.

     ω≡""?{                                      ⍝ null expr: binding matrix.
         ω=0?[cats reps bkts blabs bmat zmat];    ⍝ raw binding struct.
         trim←{                                  ⍝ without empty rows and cols.
             [r c]←$t,¨1 0{∨/⍠α ω}¨⊂1 1↓~ω∊[""]   ⍝ occupied rows and cols.
             r#c#₋₁ω                               ⍝ masked out empties.
         }                                       ⍝ :: ∇ cells → cells
         snip←{                                  ⍝ snip out top corner.
             t←≥ ,ω ι '┬'                        ⍝ width up to the uppermost, leftmost ┬
             cnr←⊃[1 t-2 1]↣#¨"  ┌" "  │" "┌─┼"    ⍝ empty corner.
             cnr@(ι3ₓ ,⊗ ιt) ω                 ⍝ snipped formatted matrix.
         }                                       ⍝ cmat ← ∇ cmat
         bfmt←snip∘_array.disp∘((⍬↣⍕)¨)∘trim∘table   ⍝ binding matrix formatting.
         bfmt [cats bmat zmat]                   ⍝ formatted table.
     }opt;                                        ⍝ opt-ional formatting.

     reduce←{                                    ⍝ 2-by-2 parsing.
         [[∆l L] Aa Bb Cc [R ∆r]]←ω              ⍝ 3-token window on stream.
         [Aa Cc]∧.≡eos?Bb;                        ⍝ single node: done.
         [Aa Bb]∧.≡eos?ω;                         ⍝ error: partial parse.
         [[A a] [B b] [C c]]←[Aa Bb Cc]          ⍝ cats and toks.
         ⊂[b c] ∊ 1↓bkts?∇ rgt [[∆l L] Aa (ebk b) R ∆r]; ⍝ empty brackets [].
         ⊂[a c] ∊ bkts?∇ rgt [∆l L (a bkt Bb) R ∆r];    ⍝ bracketed single value Bb.
         ⊂a ∊ rbs?∇ lft lft ω;                    ⍝ right bracket: skip left.
         ≥/xmat.[[A B] [B C]]?∇ lft ω;              ⍝ A:B ≥ B:C → skip left.
         BbCc←zmat.[≤B ≤C],⊂[b c]              ⍝ B bound with C.
         ∇ show [[∆l L] Aa BbCc R ∆r]            ⍝ binds with token to the right?
     }                                           ⍝ :: ∇ stream → stream

     show←⊣↢{⎕←sfmt lft⍣{eos≡↑α}ω}⍣opt⍨          ⍝ optional tracing, en passant.

     bkt←{                                       ⍝ bind of bracketed node [ ω ].
         [cat expr]←ω                            ⍝ category of bracketed expr.
         zcat←lbsια ⌷ cat,1↓bcats                ⍝ resulting category.
         [zcat [α expr]]                         ⍝ α-bracketed node.
     }                                           ⍝ :: left_bkt ∇ node → node

     ebk←{[(lbsιω)⌷bcats ω]}                      ⍝ empty brackets. [] {} ...

     tfmt←{                                      ⍝ tree-formatted.
         0ₓ=≡ω?1 1ρω;                             ⍝ atom: char matrix single.
         subs←⍉⊃{α,' ',ω}/(↓₀∇)¨ω                ⍝ formatted sub-expressions.
         mask←~(↑↓subs)∊"┌─┐ "                   ⍝ sub-exprs connection points.
         mid←ιρmask = ⌊ +/⍸mask ÷ 2               ⍝ mid-point for '┴' char.
         inx←mask+⊽+\mask                       ⍝ indices for box-draw chars.
         top←[inx+4ₓ×mid]⌷" ?─┌ ┐┴"                ⍝ "  ┌─┴─┐  "
         top⍪subs                                ⍝ formatted expression.
     }

     atop←cats{                                  ⍝ category atop tree.
         α=0ₓ?ω;                                 ⍝ ignore bad cat.
         dent←+/∧\(↑↓ω)∊"┌─┐ "                   ⍝ indent for category.
         top←dent#' ' , ≤α ⊃ ⍶⊣0               ⍝ indented category.
         ⊃[top],↓ω                               ⍝ categorised tree.
     }

     vect←{α←⍬                                   ⍝ vector from cons list
         [∆l Aa Bb Cc ∆r]←ω                      ⍝ 3-item window.
         ~Aa≡eos?α,⊂Aa ∇ rgt ω;                 ⍝ accumlate next item.
         ~α≡⍬?α;                                  ⍝ trailing eos: done.
         α ∇ rgt ω                               ⍝ skip leading eos.
     }

     class←{                                     ⍝ classification of expr tokens.
         pairs←,/α,¨¨≥ιρα                      ⍝ token-category pairs.
         [toks cats]←↓⍉⊃pairs                    ⍝ category of each token.
         [toksιω]⌷cats,0ₓ                       ⍝ token categories.
     }

     pfmt←atop↢tfmt⌿                             ⍝ format of (token cat) pair.
     sfmt←_array.disp∘pfmt∘⍉∘⊃∘vect              ⍝ format of parse stream.

     lft←{[[∆l L] A B C ∆r]←ω ◇ [∆l L A B [C ∆r]]}     ⍝ skip left.
     rgt←{[∆l A B C [R ∆r]]←ω ◇ [[∆l A] B C R ∆r]}     ⍝ skip right.

     [lbs rbs]←↓⍉⊃bkts                           ⍝ left and right brackets.
     bcats←≥catsιblabs                          ⍝ bracket categories.
     xmat←0ₓ,0ₓ⍪bmat                             ⍝ extended bmat.
     pairs←{↓⍉⊃[(reps class ω) ω]}ω~" "           ⍝ cat-token pairs.
     eos←0                                      ⍝ end of stream marker.

     ∆l←{[α ω]}⍨/⌽eos,¯2↓pairs                  ⍝ left list.
     [Aa Bb Cc]←¯3↑[eos eos],pairs,eos           ⍝ 3-token window

     tree←reduce show [∆l Aa Bb Cc eos]          ⍝ reduced expression.
     eos≡↑tree?sfmt tree;                         ⍝ bad parse: show stream.
     pfmt tree                                  ⍝ good parse: show tree.
 }

⍝ From https://dfns.dyalog.com/c_tc.htm
tc←{  ⍝ Trace function application.
  ↑⌽⎕←[α ⍶ ω '⇒' (α ⍶ ω)]
}

⍝ From https://dfns.dyalog.com/c_cols.htm
 cols←{                         ⍝ Multi-column display.

     α←[1 102]ₓ                      ⍝ default inter-column-gap and max-width.

     split←{+/∨\' '≠⌽ω ↑¨ ↓ω}        ⍝ without-trailing-blanks-split (idiom).

     1ₓ<ρρω?α ∇ split ω;              ⍝ char matrix: use the split vector.

     maxcols←{                       ⍝ maximum number of cols.
         ω=0ₓ?0ₓ;                     ⍝ one or more items too wide: stop.
         [nr xs]←|[0ₓ ω]⊤-ni         ⍝ number of rows and excess.
         xs≥nr?∇ ≤ω;                 ⍝ one or more empty columns: ignore.
         ms←[ω nr]ρlens              ⍝ matrix of sizes.
         wid←gap×≤ω + +/⌈/ms        ⍝ total width.
         wid≤max?ω;                   ⍝ it fits: hurrah!
         ∇ ≤ω                       ⍝ otherwise, try one less column.
     }

     reshape←{                       ⍝ α-col reshape of ω.
         α=0ₓ?max↑¹ col ω;          ⍝ item too wide: truncated single col.
         nr←⌈ni÷ₐα                    ⍝ number of rows.
         matr←[α nr]ρ(α×nr)↑ω,α#' '  ⍝ matrix blank-padded to whole no of cols.
         pads←⊃nr#↓gap#' '           ⍝ inter-column padding.
         {α,pads,ω}/col¨↓matr       ⍝ gap-spaced cols.
     }

     col←(⍬↣⍕)∘{1ₓ=≡ω?⍪ω;⊃ω}         ⍝ formatted numeric or char-vecs column.

     [gap max]←2↑α,102ₓ              ⍝ inter-column-gap and max-width.
     ni←≢ω                           ⍝ number of items.
     lens←⊽≢ω ↑ ,⊃ρ¨(⍬↣⍕)¨ω          ⍝ item-lengths.
     ulim←⌈max÷ₐ1ₓ⌈gap+⌊/ni↑lens      ⍝ cols: upper limit for search.

     maxcols⁝ulim reshape ω        ⍝ multi-column display.
 }

cal←{ ⍝ Calendar for absolute year or (year month).
    α←1
    cntr←{⌈0.5×+/∧\' '=ω ⌽ ω}
    1ₓ=≢,ω?{
        12ₓ≥|ω?•signal "DOMAIN";
        year←4 3ρ 0 cal¨ ω,¨≥ι12ₓ
        join←{⍉⊃(↓αᵀ),"   ",↓ωᵀ}
        head←cntr ¯66↑⍬⍕ω
        head⍪,⍠ ι2ₓ ⊃join/year
    }ω;
    dys←"Su" "Mo" "Tu" "We" "Th" "Fr" "Sa"
    months←"January" "February" "March" "April" "May" "June"
    months,←"July" "August" "September" "October" "November" "December"
    [yyyy mm]←ω
    day←days [yyyy mm 1ₓ]
    [mms dds]←2↑1↓↓(date day+ι31ₓ)ᵀ
    fmts←2 0↣⍕¨(mm=mms)#dds
    pad←7ₓ|day ↑ 0↑fmts
    dmat←⊃{α,' ',ω}/dys⍪6 7ρ42↑pad,fmts
    head←(≤mm ⊃ months),α#' ',⍬⍕yyyy
    ⊃(⊂cntr ¯20↑head),↓{∨/ω≠' ' # ω}dmat
}
packZ←{ ⍝ LZW: positive bit limit compresses; zero expands; negative returns dictionary.
    α←12ₓ
    α=0ₓ?{
        [shape bits alph]←ω
        codes←⊥bitsᵀ
        expand←{
            [codes dict prev out]←ω
            0ₓ=≢codes?out;
            n←↑codes
            text←{ω<≢dict?ω⊃dict;prev,↑prev}n
            next←dict , 0ₓ<≢prev # ⊂prev,↑text
            ∇ [1↓codes next text out,text]
        }
        shapeρexpand [codes ,¨alph 0↑alph 0↑alph]
    }ω;
    shape←ρω ◇ src←,ω ◇ alph←∪src
    limit←2ₓ*|α
    limit<≢alph?•signal "DOMAIN";
    compress←{
        [rest dict word codes]←ω
        0ₓ=≢rest?[codes,(0ₓ<≢word)#dictι⊂word dict];
        next←word,↑rest
        ⊂next ∊ dict?∇ [1↓rest dict next codes];
        extended←dict , limit>≢dict # ⊂next
        ∇ [1↓rest extended 1↑rest codes,dictι⊂word]
    }
    [codes dict]←compress [src ,¨alph 0↑alph ⍬]
    α<0ₓ?⊃dict;
    width←⌈2⊛≥⌈/codes,1ₓ
    [shape ⍉(widthρ2ₓ)⊤codes alph]
}

unify←{ ⍝ Unify expressions; α lists variable symbols.
    vars←,α
    isvar←{∨/ω↣≡¨vars}
    disagree←{
        α≡ω?⍬;
        isvar α?[α ω];
        isvar ω?[ω α];
        0ₓ∊≡¨[α ω]?•signal "DOMAIN";
        ~ ρα ≡ ρω?•signal "LENGTH";
        i←,α≡¨ω ι $f
        i⊃,α ∇ i⊃,ω
    }
    solve←{
        [x y]←ω
        pair←x disagree y
        0ₓ=≢pair?x;
        [var val]←pair
        occurs←{ω≡var?$t;0ₓ=≡ω?$f;∨/,∇¨ω}
        occurs val?•signal "DOMAIN";
        subst←{ω≡var?val;0ₓ=≡ω?ω;∇¨ω}
        ∇ [(subst x) (subst y)]
    }
    {solve [α ω]}/ω
}

ratsum←{
    sum←{[lrus mans rrus]←ω
        [cr rru]←α rrusum rrus
        [cm pman]←cr rsum mans
        [lru man]←cm lrusum [lrus pman]
        [lru man rru]
    }
    rrusum←{
        [co rru]←α rsum ω
        co=α?[co rru];
        co rsum ω
    }
    lrusum←{[lrus man]←ω
        [co lru]←α rsum lrus
        co=α?[lru man];
        [cc dd]←α rsum 2 ¯1↑lrus
        cc ∇ [¯1⌽lrus dd,man]
    }
    atab←{
        min←ωι'0'
        vals←ι≢ω - min
        ntab←vals+⊗vals
        base←2#ρω
        vtab←-min-base⊤base⊥min+base⊤ntab
        ptab←↓[min+vtab]⌷ω
        {ptab,ω ⍪ ω,⊂"0."}ω,¨'.'
    }
    rsum←(atab ⍶~"{}"){
        [cov itot]←↓⍉⊃[↓⍉digsιω]⌷⍶
        '0'∧.=cov,α?['0' itot];
        [co tot]←'0'∇⊃[1↓cov,α itot]
        [↑↑⌽'0'∇⊃co,⊂1↑cov tot]
    }
    compile←{
        [lmrs wids reps]←⍉⊃trans¨[α ω]
        mpads←-wids-⊂⌈/wids
        rpads←2#⊂∧¨/reps
        padded←pad¹ ⍉[lmrs◇mpads◇rpads]
        ⊃¨↓⍉padded
    }
    trans←{
        ~∧/ω∊digs,"<|.>"?err"bad char";
        ~"<>"≡extω?err"bad <>s";
        ~"<>"≡ω∩"<>"?err"bad <>s";
        ~"||"≡ω∩"||"?err"bad ||s";
        [lru man rru]←'|'sepr ω~"<>"
        $t∊[lru man rru]∊[""]?err"null field";
        '.'∊lru,rru?err"bad number";
        ml←manι'.'
        mr←ρman - ml+'.'∊man
        [[lru man rru] ml,mr ,⊃ρ¨[lru rru]]
    }
    pad←{
        [[l m r] [ml mr] [lr rr]]←ω
        dot←'.'~m,mr↓'.'
        man←⌽mlρ⌽l , m,dot,mrρr
        rru←rrρmr⌽r
        lru←⌽lrρ⌽(-ml)⌽l
        [lru man rru]
    }
    norm←{crru xabs icon clru ω}
    clru←{[lru man rru]←ω
        ∨/∧/optl=⊗lru?ω;
        [lw rw]←⌈\ρ¨[lru rru]
        mw←ρman~"."
        _lru←[digsιlru]⌷cdigs
        _man←". "repl(man≠'.')#⁻mwρ_lru
        _rru←rwρmw⌽_lru
        _lmr←[_lru _man _rru]
        lmrr←[lru man rwρrru]
        ∇ '0' sum _lmr {[α◇ω]}¨ lmrr
    }
    icon←{[lru man rru]←ω
        [⌽minrep⌽lru man (minrep rru)]
    }
    minrep←{
        facs←{0ₓ=ω|ρω # ω}≥ιρω
        reps←facsρ¨⊂ω
        seqs←ρω ρ¨ reps
        seqsι⊂ω ⊃ reps
    }
    xabs←{[lru man rru]←ω
        1ₓ=ρman?ω;
        '.'=↑⌽man?∇ [lru ¯1↓man rru];
        '.'∊man ∧ ↑⌽man = ↑⌽rru?∇ ¯1 amd ω;
        ~'.'∊2↑man ∧ ↑lru ≡ ↑man?∇ 1 amd ω;
        ω
    }
    amd←{[lru man rru]←ω
        lru←α⌈0 ⌽ lru
        rru←α⌊0 ⌽ rru
        [lru α↓man rru]
    }
    crru←{[lru man rru]←ω
        ~rru≡,comp'0'?ω;
        sig←×-/digsιrru,'0'
        inc←digsι'0' ⊃ sig⌽digs
        zero←'0'¨¨ω
        inc sum ω {[α◇ω]}¨ zero
    }
    sepr←{α{≢α ↓¨ α⍷ω ⊂ ω}α,ω}
    join←{⊃α{α,⍶,ω}/ω}
    repl←{⊃ω{α join ω sepr ⍶}/α}
    comp←{[(digs,ω)ιω]⌷(⌽digs),ω}
    fmt←{'|' ".|"repl('|'join ω)join"<>"}
    ext←{⌽2↑¯1⌽ω}
    err←{•signal "DOMAIN"}
    digs←⊃↓/(1 ¯1×"{}"≡ext ⍶),⊂⍶
    optl←'0' , '0'∊ext⁝digs # comp'0'
    cdigs←{
        '0'∊extω?comp ω;
        ¯1-⊽ωι'0' ⌽ ⌽ω
    }digs
    xchars←" {}[]()<>\|/,:."
    $t∊xchars∊digs?err"bad digit";
    ~'0'∊digs?err"missing zero";
    ~digs≡∪digs?err"duplicate digits";
    dyad←$t ◇ α←0⊣dyad←$f ◇ ~dyad?fmt norm ↑ trans comp ω;
    fmt norm '0' sum α compile ω
}
