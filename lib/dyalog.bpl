⍝ Dyalog dfns — adapted for BPL
⍝ Source: https://dfns.dyalog.com/n_contents.htm
⍝ Exported from Dyalog 20.0.53963.0; individual sources below.
⍝ Modified to count from 0, for based arrays and for BPL notation.

_array←•load "./array.bpl"
⍝ From https://dfns.dyalog.com/c_segs.htm
segs←{~⍵∊⍺ ⊆ ⍵}  ⍝ Separator-delimited segments.

⍝ From https://dfns.dyalog.com/c_m91.htm
m_91←{⍵>100ₓ?⍵-10ₓ;∇ ∇ ⍵+11ₓ}  ⍝ McCarthy's M91 function.

⍝ From https://dfns.dyalog.com/c_life.htm
life←{[$t ⍵]∨.∧3ₓ 4ₓ=¨⊂+/,¯1 0 1⊖⊗¯1 0 1⌽⊗⊂⍵}

⍝ From https://dfns.dyalog.com/c_rmcm.htm
rmcm←{  ⍝ Replace comments with blanks.
  cm←∨\ ⍵='⍝' > ≠\⍵='''
  ' '@ ⍸cm ⍵
}

⍝ From https://dfns.dyalog.com/c_ripple.htm
ripple←{[(⍋⍒(⍴⍵)⍴0 1)]⌷⍵}  ⍝ Perfect ripple shuffle.

⍝ From https://dfns.dyalog.com/c_birthday.htm
birthday←{  ⍝ Probability of a shared birthday among ⍵ people, with ⍺ possible dates.
  1 - ×/ ⍺-⍳⍵ ÷ ⍺
}

⍝ From https://dfns.dyalog.com/c_lsys.htm
lsys←{  ⍝ Lindenmayer L-system expansion.
  2ₓ≥≡⍺?⊂,⍺ ∇ ⍵;
  [fms tos]←↓⍉⊃,⍺
  ∊[(fms,⍵)⍳⍵]⌷tos,⍵
}

⍝ From https://dfns.dyalog.com/c_rr.htm
rr←{  ⍝ Round-robin tournament.
  v←⍳n←0ₓ⌈≤⍵
  ≥(⍵÷2)↑⍠¯1 m{[⍺⋄⍵]}⍤1⌽m←0ₓ,v⌽≥[n n]⍴v
}

⍝ From https://dfns.dyalog.com/c_easter.htm
easter←{  ⍝ Easter Sunday, yyyymmdd.
  G←≥19ₓ|⍵
  C←≥⌊⍵÷100ₓ
  X←¯12ₓ+⌊C×3ₓ÷4ₓ
  Z←¯5ₓ + ⌊ 5ₓ+8ₓ×C ÷ 25ₓ
  S←(⌊ 5ₓ×⍵ ÷ 4ₓ)-X+10ₓ
  E←30ₓ | 11ₓ×G + 20ₓ+Z-X
  F←E + E=24ₓ ∨ E=25ₓ ∧ G>11ₓ
  N←30ₓ×F>23ₓ + 44ₓ-F
  N+←7ₓ-7ₓ|S+N
  M←3ₓ+N>31ₓ
  D←N-31ₓ×N>31ₓ
  [10000 100 1]ₓ+.×[⍵ M D]
}

⍝ From https://dfns.dyalog.com/c_dice.htm
dice←{
  ⍵≡[6 6]ₓ?"Box Cars";
  ⍵≡[1 1]ₓ?"Snake Eyes";
  =/⍵?"Pair";
  7ₓ=+/⍵?"Seven";
  "Unlucky"
}

⍝ From https://dfns.dyalog.com/c_packN.htm
packN←{  ⍝ Null packing: shape, non-null mask, non-null items.
  cmp←{mask←,⍵≠1↑0⍴⍵ ⋄ [⍴⍵ mask mask#,⍵]}
  exp←{[shape mask items]←⍵ ⋄ shape⍴mask#⁻¹items}
  ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packR.htm
packR←{  ⍝ Run-length encoding: shape, counts, items.
  cmp←{
    shape←⍴⍵ ⋄ vect←,⍵
    runs←$t,≠/↕vect
    repls←-⍨/↕⍸runs,$t
    [shape repls runs#vect]
  }
  exp←{[shape repls solos]←⍵ ⋄ shape⍴repls#solos}
  ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packU.htm
packU←{  ⍝ Unique packing: shape, unique items, zero-based indices.
  cmp←{u←∪,⍵ ⋄ [⍴⍵ u u⍳,⍵]}
  exp←{[shape u i]←⍵ ⋄ shape⍴[i]⌷u}
  ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_date.htm
date←{  ⍝ Timestamp from day number (Meeus).
  ⍺←¯53799ₓ
  qr←{⊂⍠ ⍳≢⍴⍵ (0ₓ,⍺)⊤⍵}
  [Z F]←1ₓ qr ⍵+2415020ₓ
  a←⌊ Z-1867216.25 ÷ 36524.25
  A←Z + Z≥⍺+2415021ₓ × ≥a-⌊a÷4ₓ
  B←A+1524ₓ
  C←⌊ B-122.1 ÷ 365.25
  D←⌊C×365.25
  E←⌊ B-D ÷ 30.6001
  [dd df]←1ₓ qr B-D + F-⌊30.6001×E
  mm←E-1ₓ+12ₓ×E≥14ₓ
  yyyy←C-4715ₓ+mm>2ₓ
  part←[60 60 1000]ₓ qr ⌊0.5+df×86400000ₓ
  ⊃⍠0 [yyyy mm dd],part
}

⍝ From https://dfns.dyalog.com/c_days.htm
days←{  ⍝ Days since 1899-12-31 (Meeus).
  ⍺←17520902ₓ
  1ₓ<≢⍴⍵?⍺ ∇⍤0 1 ⍵;
  [yy mm dd h m s ms]←7↑⍵
  D←dd + [0 60 60 1000]ₓ⊥[h m s ms] ÷ 86400000ₓ
  [Y M]←[yy mm]+[¯1 12]ₓ×mm≤2ₓ
  A←⌊Y÷100ₓ
  B←⍺<[0 100 100]ₓ⊥[yy mm dd] × 2ₓ-A + ⌊A÷4ₓ
  ¯2416544ₓ+D+B++/⌊365.25 30.6×[Y M]+[4716 1]ₓ
}

⍝ From https://dfns.dyalog.com/c_draw.htm
draw←{  ⍝ Box-drawing characters over a marker.
  ⍺←"*"
  rh←×/¯1↓⍴⍵ , ¯1↑1,⍴⍵
  ch←"│─┌─┐─┬││└├┘┤┴┼"
  z←,' '⍪(' ' , rh⍴⍵ , ' ')⍪' '
  bv←z∊⍺ ⋄ ix←⍸bv
  in←{⊥[ix+⊗[-⍵ ¯1ₓ 1ₓ ⍵]]⌷bv}2ₓ+rh₁
  z.[(×in # ix)]←[≤in~[0ₓ]]⌷ch
  ⍴⍵ ⍴ 1 1↓¯1 ¯1↓(rh+2)⍴z
}

⍝ From https://dfns.dyalog.com/c_dots.htm
dots←{  ⍝ Show indentation with white dots.
  ⍺←'·'
  kwds←":E" ":C" ":U"⍷¨⊂⍵
  ends←⊃∨/kwds,⊂⍵='}'
  spcs←∧\' '=⍵
  xdents←ends∧$t,0 ¯1↓spcs
  flood←⍪[1 2]ₓ {hits←⍺⍷⍵ ⋄ ~$t∊,hits?⍵;⍺ ∇ hits+⍵} spcs+⊽xdents
  ⍺@ ⍸2ₓ=flood-xdents ⍵
}

⍝ From https://dfns.dyalog.com/c_logic.htm
logic←{+/(⍵+⊽⍺ =⊗ [0 1 2 3]ₓ)×[2 2 2 2]ₓ⊤⍺⍶⍵}

⍝ From https://dfns.dyalog.com/c_Depth.htm
Depth←{  ⍝ Apply ⍶ at depths ⍹.
  ⍺←⊣
  depths←⌽3⍴⌽⍹
  [m l r]←depths-⍨ ≡¨[⍵ ⍺ ⍵] × depths≥0ₓ
  [max encl]←"¨⊂"⍴¨⍨l(⌈,↢|-)r
  [el er]←$t $f=l≤r #¨ ⊂encl
  ⍎'(',el,"⍺)⍶",max,er,'⍵'
}

⍝ From https://dfns.dyalog.com/c_case.htm
case←{  ⍝ Select statement.
  ~∨/b←<\⍺?⍵;
  ¯1↑b?⍹ ⍵;
  $t $f≡b?⍶ ⍵;
  ¯1↓b ⍶ ⍵
}
⍝ From https://dfns.dyalog.com/c_morse.htm
morse←{  ⍝ Conversion to/from Morse code. A blank is the inter-word separator /.
  [P M]←↓⍉['A' ".-"⋄'B' "-..."⋄'C' "-.-."⋄'D' "-.."⋄'E' "."⋄'F' "..-."⋄'G' "--."⋄'H' "...."
    'I' ".."⋄'J' ".---"⋄'K' "-.-"⋄'L' ".-.."⋄'M' "--"⋄'N' "-."⋄'O' "---"⋄'P' ".--."
    'Q' "--.-"⋄'R' ".-."⋄'S' "..."⋄'T' "-"⋄'U' "..-"⋄'V' "...-"⋄'W' ".--"⋄'X' "-..-"
    'Y' "-.--"⋄'Z' "--.."
    '0' "-----"⋄'1' ".----"⋄'2' "..---"⋄'3' "...--"⋄'4' "....-"
    '5' "....."⋄'6' "-...."⋄'7' "--..."⋄'8' "---.."⋄'9' "----."
    '.' ".-.-.-"⋄',' "--..--"⋄':' "---..."⋄'?' "..--.."⋄''' ".----."⋄'-' "-....-"
    '/' "-..-."⋄'(' "-.--."⋄')' "-.--.-"⋄'"' ".-..-."⋄'@' ".--.-."⋄'=' "-...-"
    ' ' "/"]
  1ₓ=≡,⍵?[P⍳⍵∩P]⌷M;  ⍝ plain text to Morse.
  2ₓ=≡,⍵?[M⍳⍵∩M]⌷P  ⍝ Morse to plain text.
}

⍝ From https://dfns.dyalog.com/c_base64.htm
base_64←{  ⍝ Base64 encoding/decoding of octet vectors.
  chars←"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
  bits←{, ⍺⍴2ₓ ⊤ ⍵}
  part←{(⍴⍵ ⍴ ⍺↑$t)⊂⍵}
  ' '=↑0⍴,⍵?2ₓ↣⊥∘(8↣↑)¨ 8 part {-8|⍴⍵ ↓ ⍵} 6 bits {⍵≠64ₓ # ⍵} chars⍳⍵;
  four←{
    8ₓ=⍴⍵?"=="∇⍵,[0 0 0 0]ₓ;
    16ₓ=⍴⍵?'='∇⍵,[0 0]ₓ;
    chars.[(2ₓ↣⊥¨ 6 part ⍵)],⍺
  }
  cats←{"",/⍵}
  cats ""↣four¨ 24 part 8 bits ⍵
}

⍝ From https://dfns.dyalog.com/c_packB.htm
packB←{  ⍝ Unique indices packed as run lengths and bits.
  cmp←{
    u←∪,⍵
    i←u⍳,⍵ ⋄ w←⌈2⍟⍴u
    b←$t,≠/↕i
    j←0≠,⍉(w⍴2)⊤b#i
    [(⍴⍴⍵),(⍴⍵),u b j]
  }
  exp←{
    [q b j]←⍵ ⋄ s←↑q
    r←s⍴1↓q ⋄ u←≥s ↓ q
    w←⌈2⍟⍴u
    e←-⍨/↕⍸b,$t
    i←⊥⍉w{⍺,⌊⍺÷⍨⍴⍵ ⍴ ⍵}j
    r⍴[e#i]⌷u
  }
  ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_packX.htm
packX←{  ⍝ Text packed through pairs of unique items.
  cmp←{
    u←∪,⍵
    p←↓{2,⍨⌈0.5×⍴⍵ ⍴ ⍵},⍵
    j←,u,⊗u
    b←j∊p ⋄ g←b#j
    i←g⍳p
    w←⌈2⍟⍴g
    q←0≠b,,⍉(w⍴2)⊤i
    [u (⍴⍴⍵),⍴⍵ q]
  }
  unc←{
    [u d a]←⍵
    s←↑d ⋄ r←s⍴1↓d
    j←,u,⊗u
    p←(⍴j ↑ a)#j
    q←⍴j ↓ a
    w←⌈2⍟⍴p
    i←⊥⍉w{⍺,⌊⍺÷⍨⍴⍵ ⍴ ⍵}q
    r⍴∊[i]⌷p
  }
  ⍺←$t ⋄ ⍺?cmp ⍵;unc ⍵
}

byfreq ← {[⍒(≢⊢)⌸⍵]⌷∪⍵}  ⍝ Uniques of ⍵, most frequent first. Shared by packQ and packD.

⍝ From https://dfns.dyalog.com/c_packQ.htm
packQ←{  ⍝ Frequency-ranked uniques with variable-width keys.
  key←,/{↓ ⍳⍵ >⊗ ⍳⍵}¨≥⍳23ₓ
  cmp←{
    dt←,⍵
    u←byfreq dt
    k←$t,¨key ⋄ d←∊[u⍳dt]⌷k
    [u (⍴⍴⍵),⍴⍵ d]
  }
  unc←{
    [u q d]←⍵
    s←↑q ⍴ 1↓q
    p←d≤¯1↓$f,d
    s⍴[key⍳p⊆d]⌷u
  }
  ⍺←$t ⋄ ⍺?cmp ⍵;unc ⍵
}

⍝ From https://dfns.dyalog.com/c_packS.htm
packS←{  ⍝ Shannon-Fano packing.
  cmp←{
    u←∪,⍵ ⋄ b←≢⊢ ⌸,⍵ ⋄ i←⍒b ⋄ uv←[i]⌷u ⋄ b←[i]⌷b
    bv←{⍵≠2 ⊆ ⍵}∊⍬{
      1ₓ=≢⍵?2,⍺;a←⊂⍺ ,¨ 0 1
      2ₓ=⍴⍵?a ∇¨ ⍵;
      n←{1⌈+/$f=(+/⍵)<⊽+\⍵}⍵ ⋄ a ∇¨ [n↑⍵ n↓⍵]
    }b
    z←[uv⍳,⍵]⌷bv
    [⍴⍵ uv ∊⍴¨bv 0≠(∊bv),∊z]
  }
  exp←{
    [r uv lv d]←⍵
    bv←lv#≥⍳⍴lv ⊆ +/lv ↑ d
    r⍴""{
      0ₓ∊⍴⍵?⍺;
      i←0ₓ{q←⍺⊃bv ⋄ q ≡ ⍴q ↑ ⍵?⍺;≥⍺ ∇ ⍵}⍵
      ⍺,i⊃uv ∇ ⍴i⊃bv ↓ ⍵
    } +/lv ↓ d
  }
  ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_cmat.htm
cmat←{  ⍝ ⍺-combinations of ⍳⍵: include the first item, then omit it.
  ⍺=0ₓ?1 0⍴0ₓ;
  ⍺>⍵?[0 ⍺]⍴0ₓ;
  ⍺=⍵?[1 ⍺]⍴⍳⍵;
  (0ₓ,≥ ≤⍺ ∇ ≤⍵)⍪≥⍺∇≤⍵
}

⍝ From https://dfns.dyalog.com/c_mayan.htm
mayan←{  ⍝ Mayan numbers; left 0 uses base 20, left 1 uses the calendar base.
  ⍺←1
  [snail dot bar]←"_@/" '⍟' '⌹'
  n←≥⌈18⍟1⌈⍵
  base←20ₓ-2⌽n↑⊽⍺
  ⍪' '⍪↢{
    ⍵=0ₓ?1 5⍴' ',snail,' ';
    [comp part]←[0 5]ₓ⊤⍵
    top←dot{
      ⍵=0ₓ?0 5⍴"";
      ⍵=1ₓ?0 0 1 0 0#⁻¹⍺;
      ⍵=2ₓ?0 1 0 1 0#⁻¹⍺;
      ⍵=3ₓ?0 1 1 1 0#⁻¹⍺;
      ⍵=4ₓ?1 1 1 1 0#⁻¹⍺
    }part
    top⍪[comp 5]⍴bar
  }¨{-1⌈+/∨\⍵≠0ₓ ↑ ⍵}base⊤⍵
}

⍝ From https://dfns.dyalog.com/c_mac.htm
mac←{  ⍝ Macro expansion over linked token lists.
  mexp←{
    [dd [a [b uu]]]←⍵
    a≡'\'?⍺ ∇ [[dd b] uu];
    b≡'='?⍺ ∇ defn ⍵;
    a∊↑⍺?⍺ ∇ dref ⍵;
    a≡'('?⍺ ∇ ⍺ ∇ [dd [b uu]];
    a≡')'?[dd [b uu]];
    ⍺ ∇ [[dd a] [b uu]]
  }
  defn←{
    [dd [name [eq uu]]]←⍵
    [val uuu]←⍺ body ['(' uu]
    ⍺,⍨↢⊂¨[name val] ⍶ [dd uuu]
  }
  body←{
    [dd [a [b uu]]]←⍵
    a∊↑⍺?⍺ ∇ dref [dd [a [b uu]]];
    a∊" )",•ucs13?[dd [b uu]];
    a≡'\'?⍺ ∇ [[dd b] uu];
    a≡'('?⍺ ∇ ⍺ mexp [dd [b uu]];
    ⍺ ∇ [[dd a] [b uu]]
  }
  dref←{
    [names vals]←⍺
    [dd [name uu]]←⍵
    val←names⍳name ⊃ vals
    [ddd n]←dd number 1ₓ
    ⍺ ⍶ [ddd (val (n copy) uu)]
  }
  number←{
    [dd d]←⍺
    ~d∊$d?[⍺ 0ₓ];
    [ddd n]←dd ∇ ⍵×10ₓ
    [ddd n+⍵×$d⍳d]
  }
  copy←{
    ⍺≡'('?⍵;
    [vv v]←⍺
    v≡'/'?vv ∇ list [⍶ 1]#⍵;
    vv ∇ [v ⍵]
  }
  list←{[⍺ ⍵]}/
  1↓¯2↓∊["" ⍬]mexp['(' (list ⍵,')')]
}

⍝ From https://dfns.dyalog.com/c_baby.htm
baby←{  ⍝ Manchester Small Scale Experimental Machine.
  fetch←{, yshift⍵ # ⍺}
  store←{⍶@ 1⍳⍨yshift⍹ ⍵}
  decode←{(⍳≢⍵ ∊ [0 5 13 16]ₓ)⊂⍵}
  yshift←0ₓ=⍳≢⍵ ↣{~$t∊⍵?⍺;¯1⌽⍺ ∇ dec ⍵}
  addsub←{c←⍺ ⍶ ⍵ ⋄ ~$t∊c?⍺ ⍹ ⍵;⍺≠⍵ ∇ $f,¯1↓c}
  add←∧addsub∨
  sub←<addsub>
  inc←{⍵≠¯1↓$t,∧\⍵}
  dec←{⍵≠¯1↓$t,∧\~⍵}
  neg←{⍵≠¯1↓$f,∨\⍵}
  ⍺←$f∧2↑↓⍵
  $f{
    [A CIo M]←⍵
    CI←⍺ add inc CIo
    [S _ PI _]←decode M fetch CI
    $f $f $f≡PI?$f ∇ [A (M fetch S) M];
    $t $f $f≡PI?(M fetch S) ∇ [A CI M];
    $f $t $f≡PI?$f ∇ [(neg M fetch S) CI M];
    $t $t $f≡PI?$f ∇ [A CI (A store S)M];
    $f $f $t≡PI?$f ∇ [(A sub M fetch S) CI M];
    $t $f $t≡PI?$f ∇ [(A sub M fetch S) CI M];
    $f $t $t≡PI?{$f}\⌽A ∇ [A CI M];
    $t $t $t≡PI?M
  }⍺,⊂⍵
}

⍝ From https://dfns.dyalog.com/c_quzzle.htm
quzzle←{  ⍝ Shortest sliding-tile paths to the three other corners.
  extend←{
    (≢Fin)=p←$t⍳⍨(Fin=0ₓ)∧NPath=⌊/(Fin=0ₓ)#NPath?0;
    (≢Target)=+/(p⌷NPath)≥(Fin>0ₓ)#NPath?0;
    n←¯1⊃p⊃Path
    ∇ delpath p , ,/ p next¨ ,/ n moves¨ Tiles
  }
  moves←{
    m←(,1 ¯1×⊗1,≥Shape₁)(,'_'⍪'∣',Shape⍴⍺⌷State){∧/(⍵=⍺⌽⍶ # ⍶)∊⍵,' '}¨⍵
    ⍵,¨m#"←↑→↓"
  }
  next←{
    ' '=t←↑⍵?¯1ₓ;
    y←¯1⊃⍺⊃Path ⌷ State
    x←' '@[=↢t] y
    r←"←↑→↓"⍳⍵₁ ⌷ ,1 ¯1×⊗1,Shape₁
    (r⌽,t=y # x)←t
    (≢State)=n←$t⍳⍨State∧.=x?addnode⁝x addpath [⍺ ⍵] ⋄ ¯1ₓ;
    ⍺∊i←⍸n∊¨Path?¯1ₓ;
    0ₓ=≢i?n addpath [⍺ ⍵] ⋄ ¯1ₓ;
    ⍺⌷NPath ≥ ≤⌊/Path.[i]⍳¨n?¯1ₓ;
    (n addpath [⍺ ⍵])modpath¨i
  }
  addnode←{
    State⍪←⍵
    i←⍵.[[Target]⌷(≤i),(≢⍵)-(i←Shape₁),1]⍳↑Tiles
    ENode,←≥i × i<≢Target
    NMoves,←∞
    ≤≢ENode
  }
  addpath←{
    [p m]←⍵ ⋄ e←⍺⊃ENode ⋄ n←≢p⊃Path
    e>0ₓ ∧ e∊Fin ∧ n ≥ Fin⍳e ⊃ NPath,0ₓ?¯1ₓ;
    Path,←⊂(p⊃Path),⍺
    Moves,←⊂(p⊃Moves)⍪m
    Fin,←⍺⊃ENode
    NPath,←n
    (⍺⌷NMoves)⌊←1×n
    ≤≢Path
  }
  modpath←{
    ⍺=¯1ₓ?¯1ₓ;
    (i←≥x⍳¯1⊃⍺⊃Path)=≢x←⍵⊃Path?⍺;
    (⍵⊃Path)←⍺⊃Path , i↓x
    (⍵⊃Moves)←⍺⊃Moves ⍪ i↓⍵⊃Moves
    (⍵⊃NPath)←≤≢⍵⊃Path
    ¯1ₓ
  }
  delpath←{
    ~$f∊i←~(⍳≢Path)∊⍵?0;
    (⍬⊃Path)←i#Path
    (⍬⊃Moves)←i#Moves
    (⍬⊃NPath)←i#NPath
    (⍬⊃Fin)←i#Fin
    0
  }
  ⍺←⍳3ₓ ⋄ Target←,⍺
  Tiles←∪,⍵ ~ " " ⋄ Shape←⍴⍵
  Path←,⊂,0ₓ ⋄ Moves←,⊂1 2⍴' ' ⋄ NPath←,0ₓ ⋄ Fin←,0ₓ
  State←1,≢,⍵ ⍴ ⍵ ⋄ NMoves←,0 ⋄ ENode←,0ₓ
  x←extend0
  ∧/Fin=0ₓ?"There are no solutions";
  i←∨/{i#⁻¹n=⌊/n←(i←Fin=⍵)#NPath}¨≥⍳≢Target
  x←[[≤i#Fin]⌷Target]⌷"Top-right" "Bottom-left" "Bottom-right"
  x←[x⋄(⊂"in "),¨(0⍕¨i#NPath),¨⊂" moves"]
  x⍪i#Moves
}

⍝ From https://dfns.dyalog.com/c_refmt.htm
refmt←{  ⍝ Indent code and align comments.
  ⍺←4 ⋄ [dent csep]←⍺
  unqt←{~≠\⍵='''}
  [code coms]←⍉{
    umsk←unqt ⍵ ⋄ cmsk←∨\umsk∧⍵='⍝'
    [code coms]←$f $t=⊂cmsk #¨ ⊂⍵
    rlb←{∨\' '≠⍵ # ⍵}
    clean←⌽ rlb ⌽ rlb code
    [clean coms]
  }⍤1 ⊃⍵
  dents←{
    toks←unqt⍵ {⍺#⁻¹⍺#⍵} ⍵
    dvec←-⌿0 1⌽+\"{}"=⊗toks
    lmask←toks='·'
    dent×lmask#dvec #¨ ' '
  },/'·',¨code
  scoms←⊂csep⍴' ' ,¨ coms
  icode←↓⊃dents,¨code
  rtb←⌽∘{∨\⍵≠' ' # ⍵}∘⌽
  qnr←icode,↢rtb¨scoms
  ⊃⍣ 1ₓ=≡⍵ qnr
}

⍝ From https://dfns.dyalog.com/c_box.htm
box←{  ⍝ Frame a text array, with optional internal row and column borders.
  ⍺←⍬ ⍬ 0 ⋄ ar←{⍵,(≢⍵)↓⍬ ⍬ 0}{2ₓ>≡⍵?,⊂,⍵;⍵}⍺
  ch←{⍵?"++++++++-|+";"┌┐└┘┬┤├┴─│┼"}1=ar₂
  rh←×/¯1↓⍴⍵ , ¯1↑1,⍴⍵ ⋄ z←rh⍴⍵
  0ₓ∊⍴∊2↑ar?q←ch₈⍪(ch₉,z,ch₉)⍪ch₈ ⋄ [0 ¯1;0 ¯1]⌷q ←2 2⍴ch ⋄ q;
  [r c]←rh{∪⍺{⍵∊⍳≥⍺ # ⍵}⍵,(~¯1ₓ∊⍵)#0ₓ,⍺}¨,¨2↑ar
  [rw cl]←rh{<⍵∪0ₓ,⍺}¨[r c]
  (~ 0ₓ,rh₁ ∊ c){
    (↑⍺)↓⍠1(-⍺₁)↓⍠1[: ⍋(≥⍳rh₁),cl]⌷⍵
  }(~ 0ₓ,rh₀ ∊ r){
    (↑⍺)↓⍠0(-⍺₁)↓⍠0[⍋(≥⍳rh₀),rw]⌷⍵
  }{
    [h w]←≢rw , ≢cl ⋄ q←[h w]⍴ch₁₀
    hz←h,rh₁ ⍴ ch₈
    vr←rh₀,w ⍴ ch₉
    ∨/0ₓ∊¨⍴¨[rw cl]?⍵⍪hz , vr⍪q;
    q₀←ch₄ ⋄ q.[: ¯1]←ch₅ ⋄ q.[: 0]←ch₆ ⋄ q₋₁←ch₇
    [0 ¯1;0 ¯1]⌷q ←2 2⍴ch ⋄ ⍵⍪hz , vr⍪q
  }z
}

⍝ From https://dfns.dyalog.com/c_sudoku.htm
sudoku←{  ⍝ All solutions, with square or ⍺-shaped groups.
  wid←≢⍵ ⋄ ⍺←√wid ⋄ grp←2⍴⍺ ⋄ set←≥⍳wid
  inx←↓(,wid⊥¨grp↣×¨⍳⌽grp)+⊗,wid⊥¨⍳grp
  inx,←{(↓⍵),↓⍵ᵀ}(⍴⍵)⍴⍳wid²
  bas←≢,⍵ ⍴ ⊂set
  ,⍵ ↣{q←[⍵]⌷⍺ ⋄ (q=0ₓ # q)←⊂set~,/q ⋄ bas.[⍵]∩←,¨q}¨inx
  ∆sqz←{
    0ₓ=≢⍵?⍺;i←↑⍵ ⋄ q←⍺
    b←1ₓ<≢¨[i]⌷⍺ ⋄ ~∨/b?⍺ ∇ 1↓⍵;
    q.[b#i]←{c←⍵⊃q ⋄ z←c~,/[i~[⍵]]⌷q ⋄ 1ₓ≠≢z?c;z}¨b#i
    r←{
      s←≢¨⍵ ⋄ 2ₓ>+/s=2ₓ?s;∧/s∊[1 2]ₓ?s;
      k←s=2ₓ {↑ ⍸⍺ ~ ⍵⍳∪⍺#⍵} ⍵ ⋄ k=0ₓ?s;
      c←~⍵∊[k⊃⍵] ⋄ q.[c#i]←c#⍵ ~¨ ⊂k⊃⍵ ⋄ ≢¨[i]⌷q
    }[i]⌷q
    0ₓ∊r?⍬;∧/r=1ₓ ≥ 1ₓ∊r?q ∇ 1↓⍵;
    j←r>1ₓ # i ⋄ q.[j]←[j]⌷q ~¨ ⊂,/[i~j]⌷q
    q ∇ 1↓⍵
  }
  ∆chk←{m←⍵ ∆sqz inx ⋄ 0ₓ=≢m?⍬;m≢⍵?∇ m;⍵}
  ∆nxt←{
    0ₓ=≢⍵?⍵;
    r←≢¨⍵ ⋄ ∧/r=1ₓ?⊂⍵;
    j←,/{[⍒{+/1ₓ=[⍵]⌷r}¨⍵]⌷⍵}wid↑inx
    i←r.[j]⍳⌊/r~[1ₓ] ⊃ j
    ⍵↣{m←⍺ ⋄ (i⊃m)←,⍵ ⋄ m}¨i⊃⍵
  }
  ⍬{
    0ₓ=≢⍵?⍺;
    m←∆chk↑⍵ ⋄ 0ₓ=≢m?⍺ ∇ 1↓⍵;
    ∨/1ₓ<≢¨m?⍺ ∇ 1↓⍵,(∆nxt m);
    ∨/ +/set ≠ {+/,/[⍵]⌷m}¨inx?⍺ ∇ 1↓⍵;
    (⍺,⊂ 2⍴wid ⍴ ,/m) ∇ 1↓⍵
  }{0ₓ=≢⍵?⍵;⊂⍵}(bas ∆sqz inx)
}

⍝ From https://dfns.dyalog.com/c_kt.htm
kt←{  ⍝ Knight's tours; ⍺ limits the number of solutions.
  ⍺←1ₓ ⋄ sreq←⍺ ⋄ nsqs←×/⍵
  kdef←,0 1⌽⊗[1 ¯1]ₓ,⊗[2 ¯2]ₓ
  net←⊂,⍳⍵ ∩¨ ↓ ⍳⍵ +⊗ kdef
  ⍳↢(⍳⍵)¨⍬{
    sreq=≢⍵?⍵;
    path←⍶,⊂⍺
    nsqs=≢path?⍵,⊂path;
    nxt←⍺⊃⍹
    0ₓ=≢nxt?⍵;
    net←⍹~¨⊂[⍺]
    ord←[⍒≢¨[nxt]⌷net]⌷nxt
    ⍵(path ⍢ net)/ord
  }net/(⌽,⍳⍵),⊂0⍴⊂⍬
}

⍝ From https://dfns.dyalog.com/c_queens.htm
queens←{  ⍝ N-queens solutions up to rotation and reflection.
  search←{
    ⊂⍬ ∊ ⍵?0⍴⊂⍬;
    0ₓ=≢⍵?rmdups ⍺;
    [hd tl]←[↑⍵ 1↓⍵]
    next←⍺↣,¨hd
    rems←hd free¨ ⊂tl
    ,/ next ∇¨ rems
  }
  cvex←≥⍳⍵ × ⊂[¯1 0 1]ₓ
  free←{⍵~¨⍺+(≢⍵)↑cvex}
  rmdups←{
    rots←⍒⍣ ⍳4ₓ
    refs←⍋⍣0 1
    best←{↑⍋⊃⍵ ⊃ ⍵}
    syms←,/refs¨rots⍵
    ⍵≡best⁝syms ⊃ [⍬ ,⊂⍵]
  }
  fmt←{
    chars←[(⊃⍵)=⊗⍳⍺]⌷"·⍟"
    expd←1↓,⊃⍺⍴⊂0 1
    ⊂⍤2 expd#⁻¹⍠¯1chars
  }
  squares←[⍳⌈⍵÷2],1↓⍵⍴⊂⍳⍵
  ⍵ fmt ⍬ search squares
}

⍝ From https://dfns.dyalog.com/c_ary.htm
ary←{  ⍝ Radix representation, including recurring fractional digits.
  ⍵=0?"0";
  base←|⍺
  [sign abs]←[×⍵ |⍵]
  exp←⌈base⍟abs
  man←abs×base*-exp
  [p q]←⌊{[⍵ 1ₓ]÷1ₓ∨⍵}man
  digs←{
    ⍵=0ₓ?[(↑↓⍉[0ₓ q]⊤⍺) ⍬ 1];
    lim<≢⍺?[(rnd↑↓⍉[0ₓ q]⊤⍺) ⍬ 0];
    ⍵∊⍺?⍺rep⍵ , 1;
    ⍺,⍵ ∇ base×q|⍵
  }
  ofmt←{
    [sig exp fix rep rat]←⍵
    fmt←{[⍵]⌷$d,$a}
    [lft pad]←0⌈1 ¯1×exp
    zro←{⍵,(≢⍵)↓0ₓ}
    neg←sig<0 # '¯'
    nnn←fmt zro lft↑fix,lft⍴rep
    fff←fmt(pad#0ₓ),lft↓fix⌊≤base
    rrr←fmt(0⌈exp-≢fix)⌽rep
    fffrrr←fff opt rrr
    dot←""≡fffrrr ↓ '.'
    etc←×≢rep # "..."
    cut←rat↓'?'
    neg,nnn,dot,fffrrr,etc,cut
  }⍣ ⍺>0ₓ
  lim←⌈base⍟÷1e¯14⌈2*¯53
  rnd←{¯2↓⍵ , +↢(5ₓ↣≤)/¯2↑⍵}
  rep←{$f $t=⊂∨\⍵=⍺ #¨ 1↑↓⍉[0ₓ q]⊤⍺}
  opt←{-+/∧\=⌿⊃⌽¨[⍺ ⍵] ↓ ⍺,⍵,⍵}
  q=1ₓ?ofmt [sign ≥exp ,1ₓ ⍬ 1];
  ofmt [sign exp],(⍬ digs p×base)
}

⍝ From https://dfns.dyalog.com/c_pack4.htm
pack_4←{  ⍝ Quad-tree packing; ⍺=0 expands, ⍺>1 allows lossy packing.
  cmp←{
    uniq←∪,⍵
    rep←{⍵{⍵#⍺-¯1⌽~⍵}~$t $t $f⍷⍵<0ₓ}⍣≡
    ⍴⍵ {[uniq ⍺ ⍵]} rep ⍺ {
      ~$f∊,⍵∊[↑,⍵]?↑,⍵;
      ⍺≥≢∪,⍵?maxx+/ ⍳≢uniq =⊗ ,⍵;
      ax←maxx⍴⍵
      size←ax⊃⍴⍵
      mask←$t,1↓(⍳size)=⌊size÷2
      ¯1ₓ,,/(⍺ ∇¨ mask⊂⍠ax ⍵)
    } uniq⍳⍵
  }
  exp←{
    [uniq shape stream]←⍵
    split←{{[⌊⍵ ⌈⍵]}⍵÷≥maxk⍵}
    maxk←{⍳≢⍵ = maxx ⍵}
    exp←{≥|0ₓ⌊≥⍵⌊0ₓ # ¯1ₓ⌈⍵}
    i←↑shape{
      [head tail]←⍵
      0ₓ≤head?[⍺⍴head tail];
      [ls rs]←split ⍺
      [lsub lrem]←ls ∇ tail
      [rsub rrem]←rs ∇ lrem
      subs←lsub,⍠(maxx ⍺) rsub
      [subs rrem]
    }{[⍺ ⍵]}/(exp stream)
    [i]⌷uniq
  }
  maxx←↑⍒
  ⍺←1ₓ ⋄ ⍺>0ₓ?⍺ cmp ⍵;exp ⍵
}

⍝ From https://dfns.dyalog.com/c_von.htm
von←{  ⍝ Capitalise names, preserving prefixes and user-supplied exceptions.
  ⍺←""
  [pre not]←{0ₓ=≢⍵?"" "";3ₓ>≡⍵?[⍵ ""];⍵}⍺
  tx←¯1•c,⍵,' '
  q←"ab af al an and ap as av da de dei della der di"
  q,←" el ja la le och of the und van ver von zu y"
  q←{' ',¨(⍵≠' ' ⊆ ⍵),¨' '}q
  not,←" al-" " c/o" " d'" " de'" " el-" " l'" " s'" " t'"
  ∆pre←{-≢⍺ {⍵∨⍺⌽⍵} ⍺⍷⍵}
  ∆not←{∨⌿⊃(-≥⍳≤≢⍺)⌽¨⊂⍺⍷⍵}
  ∆fix←{
    0ₓ=≢⍺?$f;
    q←¯1•c{2ₓ>≡⍵?,⊂,⍵;⍵}⍺
    1↓∊∨/q⍶¨⊂' ',⍵
  }
  b←¯1↓∊∨/q⍷¨⊂' ',tx < $t,>/⌽⍤1↕~tx∊" -:;.,!?'""/)("
  b∨←pre(∆pre ∆fix)tx
  b>←not(∆not ∆fix)tx
  ¯1↓(1↣•c)@(⍸b) tx
}

⍝ From https://dfns.dyalog.com/c_packH.htm
packH←{                      ⍝ Huffman packing.

    compress←{                          ⍝ compression.
        shape←⍴⍵ ⋄ vect←,⍵              ⍝ shape and items of array.
        uniq←∪vect                      ⍝ unique items.
        freq←≢⊢ ⌸vect                  ⍝ frequency of occurrence.

        tree←{                          ⍝ Huffman tree.
            1ₓ=⍴⍵?↑⌽↑⍵;                  ⍝ list exhausted: done
            nxt←[2↑⍋↑¨⍵]⌷⍵                ⍝ next two lowest frequencies,
            [freqs items]←↓⍉⊃nxt        ⍝ and corresponding items.
            ∇(⍵~nxt),⊂(+/freqs),⊂items  ⍝ collect 2 most infrequent items.
        }↓⍉⊃[freq uniq]                 ⍝ tree from frequency-item pairs.

        [items csegs]←↓⍉⊃⍬{             ⍝ Dictionary: items ←→ code-segments.
            ⍬≡⍴⍵?⊂[⍵ ⍺];                 ⍝ all done: item and binary code.
            ,/(⍺↣,¨0 1)∇¨⍵              ⍝ extended codes for sub-trees.
        }tree                           ⍝ from frequency tree.

        bits←∊[items⍳vect]⌷csegs          ⍝ bit string of tree indices.

        [leaves depths]←↓⍉⊃tree{        ⍝ tree leaves and depths.
            ⍬≡⍴⍺?⊂[⍺ ⍵];                 ⍝ leaf: leaf and depth.
            ,/ ⍺ ∇¨ ≥⍵                ⍝ branch: traverse deeper sub-branches.
        }0ₓ                             ⍝ starting at depth 0 for the root.

        [shape leaves depths bits]      ⍝ compressed structure.
    }

    expand←{                            ⍝ expansion.
        [shape leaves depths bits]←⍵    ⍝ compressed structure.

        tree←1⊃↑{                       ⍝ reconstituted Huffman tree.
            ↑⍺ ≠ ↑↑⍵?[⍺],⍵;              ⍝ distinct adjacent depths: continue.
            [≤↑⍺ [⍺₁ 1⊃↑⍵]] ∇ 1↓⍵      ⍝ identical   ..      ..  : coalesce.
        }/(↓⍉⊃[depths leaves]),¯1ₓ      ⍝ depths, leaves with trailing dummy.

        picks←⍬{                        ⍝ tree index vectors.
            ⍬≡⍴⍵?⊂⍺;                     ⍝ leaf: done.
            ,/(⍺↣,¨0 1)∇¨⍵              ⍝ visit each branch,
        }tree                           ⍝ ··· of the tree.

        maxd←≡tree                     ⍝ max depth.

        dict←,/{                       ⍝ lookup dictionary.
            item←tree⊃/⌽⍵               ⍝ next tree leaf.
            indl←≢⍵                   ⍝ trailing index length.
            2*maxd-indl ⍴ ⊂[indl item]  ⍝ extended pick vectors.
        }¨picks                         ⍝ ··· for each index vector.

        iwin←⍳maxd                      ⍝ input index window.
        ibuff←bits,maxd⍴0               ⍝ padded input buffer.

        shape⍴(,shape⍴leaves){          ⍝ restore original shape.
            [ix ox]←⍵                   ⍝ input and output indices.
            ox=≢⍺?⍺;                     ⍝ end of output, done
            nxt←⊥[ix+iwin]⌷ibuff        ⍝ next dictionary index.
            [skip item]←nxt⊃dict        ⍝ number of bits and output item.
            item@ox⍺ ∇ ⍵+[skip 1ₓ]   ⍝ traverse input and output buffers.
        }[0 0]ₓ                           ⍝ initial input/output buffer indices.
    }

    ⍺←$t ⋄ ⍺?compress ⍵;expand ⍵       ⍝ compress or expand.
}

⍝ From https://dfns.dyalog.com/c_lisp.htm
 lisp←{             ⍝ Evaluator for a subset of Scheme.

    ⍝ Parser functions take as arguments:
    ⍝   ⍺ ⍝ the input string
    ⍝   ⍵ ⍝ the current parse position in ⍺
    ⍝ and return a triplet of:
    ⍝   $f position AST     ⍝ on success
    ⍝   $t position message ⍝ on failure

     parse←{
         ⍵≥≢⍺?[$t ⍵ "unexpected eof"];
         ' '≡⍵⌷⍺?⍺ ∇ ≥⍵;
         '('≡⍵⌷⍺?⍺ parseList ≥⍵;
         ⍵⌷⍺ ∊ $d?⍺ parseNum ⍵;
         '''≡⍵⌷⍺?⍺ parseQuote ⍵;
         ⍺ parseAtom ⍵
     }

     parseList←{ ⍝ parse the inside of a list and consume the ')'
         ⍵≥≢⍺?[$t ⍵ "unexpected eof"];
         ')'≡⍵⌷⍺?[$f ≥⍵ ⍬];
         [e p h]←⍺ parse ⍵
         e?[e p h];
         [e p t]←⍺ ∇ p
         e?[e p t];
         [$f p [h],t]
     }

     parseNum←{
         l←20ₓ ⍝ 1 + the maximum length of a numeric literal
         n←+/∧\$d∊⍨l↑⍵↓⍺ ⍝ actual length of the literal
         n≡l?[$t ⍵ "numeric literal too long"];
         [$f ⍵+n ⍎n↑⍵↓⍺]
     }

     parseAtom←{
         la←20ₓ ⍝ 1 + the maximum length of an atom
         na←$d,"() '" ⍝ non-atom characters
         l←+/∧\~na∊⍨la↑⍵↓⍺
         l≡la?[$t ⍵ "atom too long"];
         [$f ⍵+l l↑⍵↓⍺]
     }

     parseQuote←{
         [e p r]←⍺ parse ≥⍵
         e?[e p r];
         [e p ["quote" r]]
     }

    ⍝ Evaluator

     isAtom←{' '≡↑0⍴⍵ ∧ 1ₓ=⍴⍴⍵}
     isNum←{0≡↑0⍴⍵ ∧ 0ₓ=⍴⍴⍵}

     eval←{ ⍝ ⍺: environment, ⍵: expression
         isNum ⍵?⍵;
         isAtom ⍵?[⍺.[: 0]⍳⊂⍵ 1]⌷⍺;
         0ₓ≡≢⍵?⍬;
         "quote"≡↑⍵?⍵₁;
         1↑⍵ ∊ ,¨'\' "lambda"?["closure" ⍺],1↓⍵;
         "cond"≡↑⍵?⍺ evcond 1↓⍵;
         ⍺∇↑⍵ apply (⊂⍺)∇¨1↓⍵
     }

     apply←{ ⍝ ⍺: procedure, ⍵: arguments
         "closure"≡↑⍺?[⍺₂⋄⍵]ᵀ⍪⍺₁ eval ⍺₃;
         ⍎(↑⍺),'⍵'
     }

     evcond←{ ⍝ ⍺: environment, ⍵: clauses
         0ₓ≡≢⍵?⍬;
         "else"≡↑↑⍵?⍺ eval ↑1↓↑⍵;
         0 ≡ ⍺eval↑↑⍵?⍺ ∇ 1↓⍵;
         ⍺ eval ↑1↓↑⍵
     }

    ⍝ Initial environment
     env←⍉⍪["+" ,⊂"+/"]
     env⍪←["-" ,⊂"-/"]
     env⍪←["*" ,⊂"×/"]
     env⍪←["=" ,⊂"=/"]
     env⍪←["write" ,⊂"⎕←"]

     [e p x]←⍵ parse 0   ⍝ e: failure flag, p: position, x: AST
     e?x;
     ⍺←$t ⋄ ~⍺?x;        ⍝ ~⍺: parse only
     env eval x
 }

⍝ From https://dfns.dyalog.com/c_Cut.htm
 Cut←{
 ⍝ Model of cut.  Works in either index-origin.
 ⍝ Iverson, K.E., Rationalized APL, 1983, Section K. http://www.jsoftware.com/papers/RationalizedAPL.htm
 ⍝ Hui, R.K.W., Some Uses of { and }, 1987, Section 3.2. http://www.jsoftware.com/papers/from.htm
 ⍝ Iverson, K.E., A Dictionary of APL, 1987, m⍤v. http://www.jsoftware.com/papers/APLDictionary.htm
 ⍝ Hui, R.K.W., and K.E. Iverson, J Introduction and Dictionary, 2011, Cut (;.). http://www.jsoftware.com/help/dictionary/d331.htm

     ⍺←|⍹ {                          ⍝ default left argument
         0ₓ=⍺?⍉0,⍪-⍴⍵;
         1ₓ=⍺?0ₓ = 1↑⍵ ⍳ ⍵;
         2ₓ=⍺?0ₓ = ¯1↑⍵ ⍳ ⍵;
         3ₓ=⍺?⍴⍴⍵ ⍴ ⌊/⍴⍵
     } ⍵

     ⍺ ⍶{⍶ ⍵}{

         Under←{⍹⁻¹ ⍹⍺ ⍶ ⍹⍵}    ⍝ dyadic case of "under" operator
         p_2←{(¯1⌽k↓⍺)⊂⍠⍶(⍶⍴0 , k←-$t⍳⍨⌽⍺)↓⍵}
         Blk_1←{0ₓ=≢⍺?⍵;0ₓ=≢↑⍺?1↓⍺ (≥⍶)⍢ ⍵;1↓⍺ (≥⍶)⍢ ⊃(⊂↑⍺)⊂⍠⍶¨⍵}
         Blk_2←{0ₓ=≢⍺?⍵;0ₓ=≢↑⍺?1↓⍺ (≥⍶)⍢ ⍵;1↓⍺ (≥⍶)⍢ ⊃(⊂↑⍺)(⍶ p_2)¨⍵}
         ifv←{(⍉⍺,⍪)⍣ 2ₓ>≢⍴⍵ ⍵}         ⍝ laminate ⍺ if ⍵ is vector or unit
         ci←{[i s]←↓(1 ifv ⍺) ⋄ (0ₓ>s){⌽⍣⍺ ⍵}¨(⍳¨|s)+(i |⍨ ⍴i ↑ ⍴⍵)-(0ₓ>i)×≤|s}
         ti←{[m s]←↓(1 ifv ⍺) ⋄ ⊃{⊃[⍵ (×s)×(|s)⌊r-⍵]}¨m↣×¨⍳⌈r÷m+(0ₓ=m)×r←(⍴m)⍴⍴⍵}
         tj←{[m s]←↓(1 ifv ⍺) ⋄ ⊃{⊃[⍵ s]}¨m↣×¨⍳⌈(≥r-|s)÷m+(0ₓ=m)×r←(⍴m)⍴⍴⍵}

         1ₓ=|⍹ ∧ ⍬≡⍺ ∨ 1ₓ≠≡⍺?⊃⍶¨(⊂ 0ₓ>⍹ × ×≢¨⍺)↓¨⍺(0 Blk_1)⊂⍵;
         2ₓ=|⍹ ∧ ⍬≡⍺ ∨ 1ₓ≠≡⍺?⊃⍶¨(⊂ 0ₓ>⍹ × -×≢¨⍺)↓¨⍺(0 Blk_2)⊂⍵;

         0ₓ=⍹?⍺(⍶ ci⌷⊢)⍵;
         1ₓ=⍹?⊃⍺(⍶¨⊂⍠0)⍵;
         ¯1ₓ=⍹?⊃ ⍺ ⍶∘(1↣↓)¨⊂⍠0 ⍵;
         2ₓ=⍹?⍺ (⍶∘⊖⍢1)Under⊖ ⍵;
         ¯2ₓ=⍹?⍺ ⍶∘(¯1↣↓)⍢2 ⍵;
         3ₓ=⍹?⍺ti⍵ (⍶⍢0)⍤2 ∞ ⍵;
         ¯3ₓ=⍹?⍺tj⍵ (⍶⍢0)⍤2 ∞ ⍵
     }⍹ ⍵
 }

⍝ Dyalog 20.0.53963.0 ⎕AV and name-start alphabet; codecs retain their original wire format.
dfnsAV←•ucs 0 8 10 13 32 12 6 7 27 9 9014 619 37 39 9082 9077 95 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 1 2 175 46 9068 48 49 50 51 52 53 54 55 56 57 3 8866 165 36 163 162 8710 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 4 5 253 183 127 9049 193 194 195 199 200 202 203 204 205 206 207 208 210 211 212 213 217 218 219 221 254 227 236 240 242 245 123 8364 125 8867 9015 168 192 196 197 198 9064 201 209 214 216 220 223 224 225 226 228 229 230 231 232 233 234 235 237 238 239 241 91 47 9023 92 9024 60 8804 61 8805 62 8800 8744 8743 45 43 247 215 63 8714 9076 126 8593 8595 9075 9675 42 8968 8970 8711 8728 40 8834 8835 8745 8746 8869 8868 124 59 44 9073 9074 9042 9035 9033 9021 8854 9055 9017 33 9045 9038 9067 9066 8801 8802 243 244 246 248 34 35 30 38 180 9496 9488 9484 9492 9532 9472 9500 9508 9524 9516 9474 64 249 250 251 94 252 96 8739 182 58 9079 191 161 8900 8592 8594 9053 41 93 31 160 167 9109 9054 9059
dfnsLetters←"_abcdefghijklmnopqrstuvwxyz∆ABCDEFGHIJKLMNOPQRSTUVWXYZ⍙ÁÂÃÇÈÊËÌÍÎÏÐÒÓÔÕÙÚÛÝþãìðòõÀÄÅÆÉÑÖØÜßàáâäåæçèéêëíîïñóôöøùúûü"

⍝ From https://dfns.dyalog.com/c_words.htm
words←{  ⍝ Split a string into words and intervening text.
  ⍺←[dfnsLetters $d]
  1ₓ=≡,⍺?[⍺ ""]∇⍵;
  [alph supp]←⍺
  w←$f{⍵∊alph ∨ ⍺∧⍵∊supp}\⍵
  ≥+\w≠¯1⌽w ⊆ ⍵
}

⍝ From https://dfns.dyalog.com/c_eis.htm
eis←{{⊂1#⍵}⍣ 1ₓ=≡,⍵ ⍵}⊘{eis⍺ , eis⍵}  ⍝ Enclose if simple; with two arguments, link them.

⍝ From https://dfns.dyalog.com/c_iotag.htm
 iotag←{                        ⍝ Generalized ⍳.
     AlphaInterval←{⍵=' '?"";         ⍝ ⍳' ' is empty vector.
         a←dfnsAV⍳"a0AÁ"                ⍝ Indices of a0AÁ.
         k←a+.≤i←dfnsAV⍳⍵             ⍝ Find starting point.
         ⍺←(≤k ⌷ a)⌷dfnsAV               ⍝ Default left argument.
         </i,j←dfnsAV⍳⍺?⌽ ⍵ ∇ ⍺;         ⍝ If ⍵ before ⍺, reverse.
         j↓ ≥i ↑ dfnsAV             ⍝ Truncate dfnsAV.
     }
   ⍝Interval←{⍺+0,+\(|d)⍴×d←⍵-⍺}    ⍝ Interval Function.
     Interval←{s←×/1↓⍵,(⍺>↑⍵)#¯1ₓ    ⍝ Calculate step size
         ⍺+s×⍳⌊1ₓ-(⍺-↑⍵)÷s}        ⍝ Generate Interval
     IndexOf←{i←1↓⍳⍴⍴⍺               ⍝ Enclose left arg axis.
         j←1-⍴⍴⍺ ↑ ⍳⍴⍴⍵              ⍝ Enclose right arg axis.
         m←[i]⌷⍴⍺ ⌈ [j]⌷⍴⍵         ⍝ Pad both arguments.
         (⊂⍠i m↑⍠i ⍺)⍳⊂⍠j m↑⍠j ⍵           ⍝ Get index.
     }
     ischar←{' '≡↑0⍴,⍵}          ⍝ character fill.
     m←$f ⋄ ⍺←m←$t                    ⍝ Monadic?
     m∧1ₓ=⍴⍴⍵?,⊗/∇¨⍵;               ⍝ Vector right argument
     m∧ischar ⍵?AlphaInterval ⍵;      ⍝ Alpha Monadic.
     m?×⍵ × ⍳|⍵;                      ⍝ Integer Monadic.
     s←0ₓ=⍴⍴⍺                        ⍝ Unit Left Argument?
     s∧ischar ⍵?⍺ AlphaInterval ⍵;    ⍝ Alpha Interval.
     s?⍺ Interval ⍵;                  ⍝ Numeric Interval.
     ⍺ IndexOf ⍵                     ⍝ Dyadic.
 }

⍝ From https://dfns.dyalog.com/c_tokens.htm
 tokens←{                           ⍝ Lex of APL src line.
     ⍺←0 ⋄ nv←⍺                          ⍝ numeric vectors as single token.
     alph←dfnsLetters         ⍝ alphabet for names.
     all←{+/∧\⍺∊,⍵}                       ⍝ No. of leading ⍺∊⍵.
     acc←{⍺,⊂↑/⍵ lex ↓/⍵}                ⍝ accumulated tokens.
     size←{                                  ⍝ Length of the token at the start of ⍵.
         hd←↑⍵
         hd=' '?⍵ all ' ';                   ⍝ White Space.
         hd∊alph?⍵ all alph,$d;              ⍝ Name
         hd='⎕'?1 + 1↓⍵ all alph;          ⍝ System Name
         hd='''?+/∧\{⍵∨¯1⌽⍵}≠\hd=⍵;          ⍝ Char literal
         hd∊$d,'¯'?max←⍵ all $d,".¯EJ",nv#' ' ⋄ max-+/∧\' '=⌽max↑⍵;  ⍝ Numeric literal, without trailing blanks.
         hd∊"⍺⍵∇:"?⍵ all hd;                 ⍝ ⍺⍺ or ⍵⍵ or ∇∇ or ::
         hd='⍝'?⍴⍵;                          ⍝ Comment
         1                                   ⍝ Single char token.
     }
     lex←{0ₓ=⍴⍵?⍺;⍺ acc [(size ⍵) ⍵]}       ⍝ Next token, else finished.
     0⍴⊂"" lex ,⍵
 }

⍝ From https://dfns.dyalog.com/c_ssword.htm
 ssword←{                   ⍝ Approx alternative to xutils' ss.
     [srce find repl]←,¨⍵        ⍝ source, find and replace vectors
     alph←dfnsLetters ⍝ primary alphabet: initial letters for names
     supp←$d                     ⍝ supplementary: 0-9
     ⍺←[alph supp]               ⍝ default left argument
     ∊(⊂repl)@[{                  ⍝ replace find with repl
         ⍵∊[find]                 ⍝ matches
     }] ⍺words⁝srce             ⍝ source into words.
 }

⍝ From https://dfns.dyalog.com/c_packD.htm
 packD←{                                 ⍝ Pack char array to boolean vector.

     key←,/{↓ ⍳⍵ >⊗ ⍳⍵}¨≥⍳23ₓ             ⍝ generate the keys (up to 276), they
                                        ⍝ start with ones and end with zeros.
     cmp←{                               ⍝ compress:
         dt←,⍵ ⋄ k←$t,¨key               ⍝ vectorise items, separators to keys
         u←byfreq dt                     ⍝ sort uniques according to frequencies.
         uq←,(8⍴2)⊤(≤⍴u),dfnsAV⍳u        ⍝ shape then uniques fit to eight bits.
         rk←,(4⍴2)⊤⍴⍴⍵                   ⍝ rank fits in four bytes

         sh←⍬{                           ⍝ shape is a trickier one
             0ₓ∊⍴⍵?⍺;                     ⍝ all done?
             n←≥⌊2⍟1⌈↑⍵                 ⍝ how many bits are needed for representing the integer?
             q←(, 5⍴2 ⊤ n),,(n⍴2)⊤↑⍵      ⍝ n should fit to five bits (2*2*5 -> 4GB dimensions supported ;) )
             ⍺,q ∇ 1↓⍵                 ⍝ add binarised figure to the result
         }⍴⍵

         0≠rk,sh,uq,∊[u⍳dt]⌷k             ⍝ join the result with binarised data, as Booleans
     }

     unc←{                               ⍝ uncompress:
         rk←⊥4⍴⍵                        ⍝ rank's easy

         [sh x]←[⍬ 4 rk]{                ⍝ shape needs more treatment
             [s n r]←⍺                   ⍝ deliver args
             r=0ₓ?[s n];                  ⍝ all done: return shape and offset
             q←⊥5⍴⍵                     ⍝ how many bits?
             d←⊥q⍴5↓⍵                   ⍝ -> dimension length
             [s,d n+q+5 ≤r] ∇ q+5 ↓ ⍵   ⍝ next
         }4↓⍵

         n←≥⊥8⍴x↓⍵                     ⍝ how many uniques?
         u←[,⊥[n 8]⍴(n×8)↑(x+8)↓⍵]⌷dfnsAV   ⍝  -> make the list
         dt←x+8×≥n ↓ ⍵                  ⍝ rip the data part
         p←dt≤¯1↓$f,dt                   ⍝ binary partitioner (first 1's)
         sh⍴[key⍳p⊆dt]⌷u                  ⍝ reconstruct
     }

     ⍺←$t ⋄ ⍺?cmp ⍵;unc ⍵              ⍝ which way to go?
 }

⍝ From https://dfns.dyalog.com/c_packT.htm
 packT←{                        ⍝ Simple text vector packager.

     cmp←{
         ec←{c←3↑dfnsAV ⋄ ↑⍋+/c=⊗⍵ ⌷ c},⍵           ⍝ escape: the least frequent of the first three dfnsAV chars

         lv←1ₓ,≠/↕,⍵ ⋄ e←ec=,⍵                ⍝ repeat lengths
         mask←e<¯1↓[1 0 0 1]ₓ⍷lv,1ₓ ⋄ (mask#1⌽lv)←1ₓ   ⍝ remove 2's and 3's
         mask←e<¯1↓[1 0 1]ₓ⍷lv,1ₓ ⋄ (mask#1⌽lv)←1ₓ
         ln←{-⍨/↕(⍵,1)#⍳≥⍴⍵}lv              ⍝ packables

         (lv#lv)←≥⌊ln÷256ₓ                  ⍝ adjust for 256 length

         ln←∊{⍵<257ₓ?⍵;                       ⍝ now we have to adjust the long
             q←256ₓ|⍵ ⋄ n←⌊⍵÷256ₓ            ⍝ sequences so that the last
             {                               ⍝ sequence will be
                 3ₓ<¯1↑⍵?⍵;                   ⍝ properly done, ie:
                 ¯2↓⍵ , [¯4 4]ₓ+¯2↑⍵         ⍝   257 -> 252 5,
             } n#256ₓ , ⍵-n×256ₓ              ⍝   NOT    256 1.
         }¨ln

         q←lv#,⍵                             ⍝ reduce data
         b←ln>3ₓ ∨ {↑⍵ , </↕⍵}q=ec           ⍝ find places where
         (b#q)←b#q {ec,⍺,⍵}¨ [≤b#ln]⌷dfnsAV       ⍝  to put pack sequences
         ec,∊q                               ⍝ remove nesting
     }

     exp←{
         ec←↑⍵                               ⍝ "esc"
         ~ec∊1↓⍵?1↓⍵;                         ⍝ packed?

         bv←{↑⍵ , </↕⍵}1↓⍵=ec                ⍝ reduce vector
         ln←0ₓ@(⍸¯2⌽bv)(≥dfnsAV⍳ ¯2⌽bv # 1↓⍵)@(⍸¯1⌽bv)~bv  ⍝ lengths: 0 for escapes and counts, the count for each packed character
         ln#1↓⍵                              ⍝ do it!
     }

     ⍺←$t ⋄ ⍺?cmp ⍵;exp ⍵                  ⍝ compress or expand.
 }
⍝ From https://dfns.dyalog.com/c_parse.htm
 parse←{  ⍝ Bunda-Gerth parsing.

     [opt defs]←⍵≡"" {[⍺ ⍵]}⍣ ~0≡↑0⍴⍺ ⍺     ⍝ trace/format option and defns.

     defn←{                                      ⍝ binding table definition.
         words←⊂" " segs¨ ↑¨ ⊂"⍝" segs¨ ⍵         ⍝ blank-delimited words.
         pop←{[↑⍵ 1↓⍵]}                          ⍝ head & tail of list.
         sects←[0⍴⊂""] segs words               ⍝ sections.
         [csect defs]←pop sects                  ⍝ separation of sections.
         bindx←↑¨csect ⍳ ⊂"()"                   ⍝ index of bracket defn.
         bsegs←bindx⊃csect,⊂,⊂"()"               ⍝ bracket pairs: () [] <> ...
         [bkts blabs]←↓⍉⊃{[⌽2↑⍵ 2↓⍵]}¨¯1⌽¨bsegs  ⍝ brackets and their cat labels.
         [cats reps]←↓⍉⊃pop¨csect~[bsegs]         ⍝ categories & representatives.
         bonds←⌽≥⍳≢defs                         ⍝ binding strengths
         bmat←cats{0ₓ}⊗cats                     ⍝ initialised binding table
         defn←{≥cats↣⍳¨ ⊂"." segs¨ ":→"segs⍵}   ⍝ split a.b:c.d→z defn
         dist←{,,⊗/2↑⍵ ,¨ ↑⌽⍵}                 ⍝ distribution of productions
         lmat←dist∘defn¨¨¨defs                   ⍝ loading matrix
         bftz←,/bonds,¨¨(,/)²¨lmat           ⍝ bond-from-to-rslt tuples
         [cats reps bkts blabs],⊂⍠[0 1]⊃{        ⍝ binding structure.
             [bond fm to rslt]←⍺                 ⍝ binding and resulting cats.
             (⊂[bond rslt])@ ⊂≤[fm to] ⍵       ⍝ populate cell
         }/bftz,⊂bmat                            ⍝ loaded binding matrix.
     }∘{                                         ⍝ pre-process alias=... lines.
         lines←↓' ',⊃⍵                       ⍝ lines from char array.
         wds←↑¨ ⊂" " segs¨ lines                  ⍝ first word from each line.
         [msk nsk]←$t $f=⊂'='∊¨wds               ⍝ mask of alias lines.
         dict←⊂"=" segs¨ msk#wds                 ⍝ (fm to) substitution pairs.
         {_array.subs/dict,⊂⍵}¨nsk#lines        ⍝ lines with expanded aliases.
     }⍣ 2ₓ≠≢⍴↑⌽defs                            ⍝ compile unless compiled.

     table←{                                     ⍝ formatted Bunda-Gerth table.
         [cats bmat zmat]←⍵                      ⍝ categories and binding matrix.
         fmt←{×⍺ # ⍬⍕⍺ , ' ',⍵⊃cats,'?'}          ⍝ bond and category.
         ttl←{[' ' ⍺],.⍪[⍺ ⍵]}                   ⍝ row and column headers.
         cats ttl bmat fmt¨ ≥≢cats | ≤zmat     ⍝ formatted bonds & categories.
     }

     [cats reps bkts blabs bmat zmat]←defn defs  ⍝ definition structure.

     ⍵≡""?{                                      ⍝ null expr: binding matrix.
         ⍵=0?[cats reps bkts blabs bmat zmat];    ⍝ raw binding struct.
         trim←{                                  ⍝ without empty rows and cols.
             [r c]←$t,¨1 0{∨/⍠⍺ ⍵}¨⊂1 1↓~⍵∊[""]   ⍝ occupied rows and cols.
             r#c#⍠¯1 ⍵                               ⍝ masked out empties.
         }                                       ⍝ :: ∇ cells → cells
         snip←{                                  ⍝ snip out top corner.
             t←≥ ,⍵ ⍳ '┬'                        ⍝ width up to the uppermost, leftmost ┬
             cnr←⊃[1 t-2 1]↣#¨"  ┌" "  │" "┌─┼"    ⍝ empty corner.
             cnr@(⍳3ₓ ,⊗ ⍳t) ⍵                 ⍝ snipped formatted matrix.
         }                                       ⍝ cmat ← ∇ cmat
         bfmt←snip∘_array.disp∘((⍬↣⍕)¨)∘trim∘table   ⍝ binding matrix formatting.
         bfmt [cats bmat zmat]                   ⍝ formatted table.
     }opt;                                        ⍝ opt-ional formatting.

     reduce←{                                    ⍝ 2-by-2 parsing.
         [[∆l L] Aa Bb Cc [R ∆r]]←⍵              ⍝ 3-token window on stream.
         [Aa Cc]∧.≡eos?Bb;                        ⍝ single node: done.
         [Aa Bb]∧.≡eos?⍵;                         ⍝ error: partial parse.
         [[A a] [B b] [C c]]←[Aa Bb Cc]          ⍝ cats and toks.
         ⊂[b c] ∊ 1↓bkts?∇ rgt [[∆l L] Aa (ebk b) R ∆r]; ⍝ empty brackets [].
         ⊂[a c] ∊ bkts?∇ rgt [∆l L (a bkt Bb) R ∆r];    ⍝ bracketed single value Bb.
         ⊂a ∊ rbs?∇ lft lft ⍵;                    ⍝ right bracket: skip left.
         ≥/xmat.[[A B] [B C]]?∇ lft ⍵;              ⍝ A:B ≥ B:C → skip left.
         BbCc←zmat.[≤B ≤C],⊂[b c]              ⍝ B bound with C.
         ∇ show [[∆l L] Aa BbCc R ∆r]            ⍝ binds with token to the right?
     }                                           ⍝ :: ∇ stream → stream

     show←⊣↢{⎕←sfmt lft⍣{eos≡↑⍺}⍵}⍣opt⍨          ⍝ optional tracing, en passant.

     bkt←{                                       ⍝ bind of bracketed node [ ⍵ ].
         [cat expr]←⍵                            ⍝ category of bracketed expr.
         zcat←lbs⍳⍺ ⌷ cat,1↓bcats                ⍝ resulting category.
         [zcat [⍺ expr]]                         ⍝ ⍺-bracketed node.
     }                                           ⍝ :: left_bkt ∇ node → node

     ebk←{[(lbs⍳⍵)⌷bcats ⍵]}                      ⍝ empty brackets. [] {} ...

     tfmt←{                                      ⍝ tree-formatted.
         0ₓ=≡⍵?1 1⍴⍵;                             ⍝ atom: char matrix single.
         subs←⍉⊃{⍺,' ',⍵}/(↓⍠0∇)¨⍵                ⍝ formatted sub-expressions.
         mask←~(↑↓subs)∊"┌─┐ "                   ⍝ sub-exprs connection points.
         mid←⍳⍴mask = ⌊ +/⍸mask ÷ 2               ⍝ mid-point for '┴' char.
         inx←mask+⊽+\mask                       ⍝ indices for box-draw chars.
         top←[inx+4ₓ×mid]⌷" ?─┌ ┐┴"                ⍝ "  ┌─┴─┐  "
         top⍪subs                                ⍝ formatted expression.
     }

     atop←cats{                                  ⍝ category atop tree.
         ⍺=0ₓ?⍵;                                 ⍝ ignore bad cat.
         dent←+/∧\(↑↓⍵)∊"┌─┐ "                   ⍝ indent for category.
         top←dent#' ' , ≤⍺ ⊃ ⍶⊣0               ⍝ indented category.
         ⊃[top],↓⍵                               ⍝ categorised tree.
     }

     vect←{⍺←⍬                                   ⍝ vector from cons list
         [∆l Aa Bb Cc ∆r]←⍵                      ⍝ 3-item window.
         ~Aa≡eos?⍺,⊂Aa ∇ rgt ⍵;                 ⍝ accumlate next item.
         ~⍺≡⍬?⍺;                                  ⍝ trailing eos: done.
         ⍺ ∇ rgt ⍵                               ⍝ skip leading eos.
     }

     class←{                                     ⍝ classification of expr tokens.
         pairs←,/⍺,¨¨≥⍳⍴⍺                      ⍝ token-category pairs.
         [toks cats]←↓⍉⊃pairs                    ⍝ category of each token.
         [toks⍳⍵]⌷cats,0ₓ                       ⍝ token categories.
     }

     pfmt←atop↢tfmt⌿                             ⍝ format of (token cat) pair.
     sfmt←_array.disp∘pfmt∘⍉∘⊃∘vect              ⍝ format of parse stream.

     lft←{[[∆l L] A B C ∆r]←⍵ ⋄ [∆l L A B [C ∆r]]}     ⍝ skip left.
     rgt←{[∆l A B C [R ∆r]]←⍵ ⋄ [[∆l A] B C R ∆r]}     ⍝ skip right.

     [lbs rbs]←↓⍉⊃bkts                           ⍝ left and right brackets.
     bcats←≥cats⍳blabs                          ⍝ bracket categories.
     xmat←0ₓ,0ₓ⍪bmat                             ⍝ extended bmat.
     pairs←{↓⍉⊃[(reps class ⍵) ⍵]}⍵~" "           ⍝ cat-token pairs.
     eos←0                                      ⍝ end of stream marker.

     ∆l←{[⍺ ⍵]}⍨/⌽eos,¯2↓pairs                  ⍝ left list.
     [Aa Bb Cc]←¯3↑[eos eos],pairs,eos           ⍝ 3-token window

     tree←reduce show [∆l Aa Bb Cc eos]          ⍝ reduced expression.
     eos≡↑tree?sfmt tree;                         ⍝ bad parse: show stream.
     pfmt tree                                  ⍝ good parse: show tree.
 }

⍝ From https://dfns.dyalog.com/c_tc.htm
tc←{  ⍝ Trace function application.
  ↑⌽⎕←[⍺ ⍶ ⍵ '⇒' (⍺ ⍶ ⍵)]
}

⍝ From https://dfns.dyalog.com/c_cols.htm
 cols←{                         ⍝ Multi-column display.

     ⍺←[1 102]ₓ                      ⍝ default inter-column-gap and max-width.

     split←{+/∨\' '≠⌽⍵ ↑¨ ↓⍵}        ⍝ without-trailing-blanks-split (idiom).

     1ₓ<⍴⍴⍵?⍺ ∇ split ⍵;              ⍝ char matrix: use the split vector.

     maxcols←{                       ⍝ maximum number of cols.
         ⍵=0ₓ?0ₓ;                     ⍝ one or more items too wide: stop.
         [nr xs]←|[0ₓ ⍵]⊤-ni         ⍝ number of rows and excess.
         xs≥nr?∇ ≤⍵;                 ⍝ one or more empty columns: ignore.
         ms←[⍵ nr]⍴lens              ⍝ matrix of sizes.
         wid←gap×≤⍵ + +/⌈/ms        ⍝ total width.
         wid≤max?⍵;                   ⍝ it fits: hurrah!
         ∇ ≤⍵                       ⍝ otherwise, try one less column.
     }

     reshape←{                       ⍝ ⍺-col reshape of ⍵.
         ⍺=0ₓ?max↑⍤1 col ⍵;          ⍝ item too wide: truncated single col.
         nr←⌈ni÷⍺                    ⍝ number of rows.
         matr←[⍺ nr]⍴(⍺×nr)↑⍵,⍺#' '  ⍝ matrix blank-padded to whole no of cols.
         pads←⊃nr#↓gap#' '           ⍝ inter-column padding.
         {⍺,pads,⍵}/col¨↓matr       ⍝ gap-spaced cols.
     }

     col←(⍬↣⍕)∘{1ₓ=≡⍵?⍪⍵;⊃⍵}         ⍝ formatted numeric or char-vecs column.

     [gap max]←2↑⍺,102ₓ              ⍝ inter-column-gap and max-width.
     ni←≢⍵                           ⍝ number of items.
     lens←⊽≢⍵ ↑ ,⊃⍴¨(⍬↣⍕)¨⍵          ⍝ item-lengths.
     ulim←⌈max÷1ₓ⌈gap+⌊/ni↑lens      ⍝ cols: upper limit for search.

     maxcols⁝ulim reshape ⍵        ⍝ multi-column display.
 }

cal←{ ⍝ Calendar for absolute year or (year month).
    ⍺←1
    cntr←{⌈0.5×+/∧\' '=⍵ ⌽ ⍵}
    1ₓ=≢,⍵?{
        12ₓ≥|⍵?•signal "DOMAIN";
        year←4 3⍴ 0 cal¨ ⍵,¨≥⍳12ₓ
        join←{⍉⊃(↓⍺ᵀ),"   ",↓⍵ᵀ}
        head←cntr ¯66↑⍬⍕⍵
        head⍪,⍠ ⍳2ₓ ⊃join/year
    }⍵;
    dys←"Su" "Mo" "Tu" "We" "Th" "Fr" "Sa"
    months←"January" "February" "March" "April" "May" "June"
    months,←"July" "August" "September" "October" "November" "December"
    [yyyy mm]←⍵
    day←days [yyyy mm 1ₓ]
    [mms dds]←2↑1↓↓(date day+⍳31ₓ)ᵀ
    fmts←2 0↣⍕¨(mm=mms)#dds
    pad←7ₓ|day ↑ 0↑fmts
    dmat←⊃{⍺,' ',⍵}/dys⍪6 7⍴42↑pad,fmts
    head←(≤mm ⊃ months),⍺#' ',⍬⍕yyyy
    ⊃(⊂cntr ¯20↑head),↓{∨/⍵≠' ' # ⍵}dmat
}
packZ←{ ⍝ LZW: positive bit limit compresses; zero expands; negative returns dictionary.
    ⍺←12ₓ
    ⍺=0ₓ?{
        [shape bits alph]←⍵
        codes←⊥bitsᵀ
        expand←{
            [codes dict prev out]←⍵
            0ₓ=≢codes?out;
            n←↑codes
            text←{⍵<≢dict?⍵⊃dict;prev,↑prev}n
            next←dict , 0ₓ<≢prev # ⊂prev,↑text
            ∇ [1↓codes next text out,text]
        }
        shape⍴expand [codes ,¨alph 0↑alph 0↑alph]
    }⍵;
    shape←⍴⍵ ⋄ src←,⍵ ⋄ alph←∪src
    limit←2ₓ*|⍺
    limit<≢alph?•signal "DOMAIN";
    compress←{
        [rest dict word codes]←⍵
        0ₓ=≢rest?[codes,(0ₓ<≢word)#dict⍳⊂word dict];
        next←word,↑rest
        ⊂next ∊ dict?∇ [1↓rest dict next codes];
        extended←dict , limit>≢dict # ⊂next
        ∇ [1↓rest extended 1↑rest codes,dict⍳⊂word]
    }
    [codes dict]←compress [src ,¨alph 0↑alph ⍬]
    ⍺<0ₓ?⊃dict;
    width←⌈2⍟≥⌈/codes,1ₓ
    [shape ⍉(width⍴2ₓ)⊤codes alph]
}

unify←{ ⍝ Unify expressions; ⍺ lists variable symbols.
    vars←,⍺
    isvar←{∨/⍵↣≡¨vars}
    disagree←{
        ⍺≡⍵?⍬;
        isvar ⍺?[⍺ ⍵];
        isvar ⍵?[⍵ ⍺];
        0ₓ∊≡¨[⍺ ⍵]?•signal "DOMAIN";
        ~ ⍴⍺ ≡ ⍴⍵?•signal "LENGTH";
        i←,⍺≡¨⍵ ⍳ $f
        i⊃,⍺ ∇ i⊃,⍵
    }
    solve←{
        [x y]←⍵
        pair←x disagree y
        0ₓ=≢pair?x;
        [var val]←pair
        occurs←{⍵≡var?$t;0ₓ=≡⍵?$f;∨/,∇¨⍵}
        occurs val?•signal "DOMAIN";
        subst←{⍵≡var?val;0ₓ=≡⍵?⍵;∇¨⍵}
        ∇ [(subst x) (subst y)]
    }
    {solve [⍺ ⍵]}/⍵
}

ratsum←{
    sum←{[lrus mans rrus]←⍵
        [cr rru]←⍺ rrusum rrus
        [cm pman]←cr rsum mans
        [lru man]←cm lrusum [lrus pman]
        [lru man rru]
    }
    rrusum←{
        [co rru]←⍺ rsum ⍵
        co=⍺?[co rru];
        co rsum ⍵
    }
    lrusum←{[lrus man]←⍵
        [co lru]←⍺ rsum lrus
        co=⍺?[lru man];
        [cc dd]←⍺ rsum 2 ¯1↑lrus
        cc ∇ [¯1⌽lrus dd,man]
    }
    atab←{
        min←⍵⍳'0'
        vals←⍳≢⍵ - min
        ntab←vals+⊗vals
        base←2#⍴⍵
        vtab←-min-base⊤base⊥min+base⊤ntab
        ptab←↓[min+vtab]⌷⍵
        {ptab,⍵ ⍪ ⍵,⊂"0."}⍵,¨'.'
    }
    rsum←(atab ⍶~"{}"){
        [cov itot]←↓⍉⊃[↓⍉digs⍳⍵]⌷⍶
        '0'∧.=cov,⍺?['0' itot];
        [co tot]←'0'∇⊃[1↓cov,⍺ itot]
        [↑↑⌽'0'∇⊃co,⊂1↑cov tot]
    }
    compile←{
        [lmrs wids reps]←⍉⊃trans¨[⍺ ⍵]
        mpads←-wids-⊂⌈/wids
        rpads←2#⊂∧¨/reps
        padded←pad⍤1 ⍉[lmrs⋄mpads⋄rpads]
        ⊃¨↓⍉padded
    }
    trans←{
        ~∧/⍵∊digs,"<|.>"?err"bad char";
        ~"<>"≡ext⍵?err"bad <>s";
        ~"<>"≡⍵∩"<>"?err"bad <>s";
        ~"||"≡⍵∩"||"?err"bad ||s";
        [lru man rru]←'|'sepr ⍵~"<>"
        $t∊[lru man rru]∊[""]?err"null field";
        '.'∊lru,rru?err"bad number";
        ml←man⍳'.'
        mr←⍴man - ml+'.'∊man
        [[lru man rru] ml,mr ,⊃⍴¨[lru rru]]
    }
    pad←{
        [[l m r] [ml mr] [lr rr]]←⍵
        dot←'.'~m,mr↓'.'
        man←⌽ml⍴⌽l , m,dot,mr⍴r
        rru←rr⍴mr⌽r
        lru←⌽lr⍴⌽(-ml)⌽l
        [lru man rru]
    }
    norm←{crru xabs icon clru ⍵}
    clru←{[lru man rru]←⍵
        ∨/∧/optl=⊗lru?⍵;
        [lw rw]←⌈\⍴¨[lru rru]
        mw←⍴man~"."
        _lru←[digs⍳lru]⌷cdigs
        _man←". "repl(man≠'.')#⁻¹mw⍴_lru
        _rru←rw⍴mw⌽_lru
        _lmr←[_lru _man _rru]
        lmrr←[lru man rw⍴rru]
        ∇ '0' sum _lmr {[⍺⋄⍵]}¨ lmrr
    }
    icon←{[lru man rru]←⍵
        [⌽minrep⌽lru man (minrep rru)]
    }
    minrep←{
        facs←{0ₓ=⍵|⍴⍵ # ⍵}≥⍳⍴⍵
        reps←facs⍴¨⊂⍵
        seqs←⍴⍵ ⍴¨ reps
        seqs⍳⊂⍵ ⊃ reps
    }
    xabs←{[lru man rru]←⍵
        1ₓ=⍴man?⍵;
        '.'=↑⌽man?∇ [lru ¯1↓man rru];
        '.'∊man ∧ ↑⌽man = ↑⌽rru?∇ ¯1 amd ⍵;
        ~'.'∊2↑man ∧ ↑lru ≡ ↑man?∇ 1 amd ⍵;
        ⍵
    }
    amd←{[lru man rru]←⍵
        lru←⍺⌈0 ⌽ lru
        rru←⍺⌊0 ⌽ rru
        [lru ⍺↓man rru]
    }
    crru←{[lru man rru]←⍵
        ~rru≡,comp'0'?⍵;
        sig←×-/digs⍳rru,'0'
        inc←digs⍳'0' ⊃ sig⌽digs
        zero←'0'¨¨⍵
        inc sum ⍵ {[⍺⋄⍵]}¨ zero
    }
    sepr←{⍺{≢⍺ ↓¨ ⍺⍷⍵ ⊂ ⍵}⍺,⍵}
    join←{⊃⍺{⍺,⍶,⍵}/⍵}
    repl←{⊃⍵{⍺ join ⍵ sepr ⍶}/⍺}
    comp←{[(digs,⍵)⍳⍵]⌷(⌽digs),⍵}
    fmt←{'|' ".|"repl('|'join ⍵)join"<>"}
    ext←{⌽2↑¯1⌽⍵}
    err←{•signal "DOMAIN"}
    digs←⊃↓/(1 ¯1×"{}"≡ext ⍶),⊂⍶
    optl←'0' , '0'∊ext⁝digs # comp'0'
    cdigs←{
        '0'∊ext⍵?comp ⍵;
        ¯1-⊽⍵⍳'0' ⌽ ⌽⍵
    }digs
    xchars←" {}[]()<>\|/,:."
    $t∊xchars∊digs?err"bad digit";
    ~'0'∊digs?err"missing zero";
    ~digs≡∪digs?err"duplicate digits";
    dyad←$t ⋄ ⍺←0⊣dyad←$f ⋄ ~dyad?fmt norm ↑ trans comp ⍵;
    fmt norm '0' sum ⍺ compile ⍵
}
