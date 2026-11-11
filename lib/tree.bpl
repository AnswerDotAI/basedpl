⍝ Tree dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/tree/tree.apl; Apache-2.0, see LICENSE-april
⍝⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_Trees.htm into April APL

⍝ Shared by every tree's fmt.
hang ← {  ⍝ node label α with formatted subtrees ω hung below it.
  key_val←α
  null←0 0ρ""  ⍝ format of null tree.
  fmts←{⊖ω}\"┌└"{  ⍝ hang subtrees by ┌─ ─┐ branches.
    [0 0]ₓ≡ρω?ω;  ⍝ null: done.
    mask←∧\' '=↑↓⌽ωᵀ
    ⍉⌽⊃[⌽α,mask#'│'],↓⌽ωᵀ
  }¨{⊖ω}\ω  ⍝ formatted subtrees.
  case←~[null null]≡¨fmts  ⍝ non-null subtree cases.
  join←⊥case ⊃ "∘┐┘┤"
  join≡'∘'?⊃,↓key_val;
  dent←' '¨key_val  ⍝ subtree padding.
  pads←{↓,/dent,⊂ω}¨fmts
  ⊃{α,(↓key_val,join),ω}/pads
}  ⍝ :: k ∇ [f f] → [-;]

⍝ Shared by every tree. A node holds its key=value pair first and its subtrees last, with a balance or colour between them in avl
⍝ and redblack.
vec ← {  ⍝ vector of key=value pairs.
  ω≡0?⍬;  ⍝ null tree: null vector.
  [lft rgt]←↑⌽ω  ⍝ subtrees.
  ∇lft , ⊂↑ω , ∇rgt  ⍝ left_vec, key=val, right_vec.
}  ⍝ :: ∇ t → [k v]

⍝ Helpers shared by sbst and splay.
fmt ← {  ⍝ formatted tree ω.
  ω≡0?0 0ρ"";  ⍝ null tree: null format.
  [[key val] subs]←ω  ⍝ node info.
  ⊃,/(⍬↣⍕)¨[key '=' val] hang ∇¨subs
}  ⍝ :: ∇ t → [-;]

chk ← {  ⍝ tree stats / integrity check.
  0ₓ=≡ω?[0≡ω 0ₓ 0ₓ 0ₓ ⍬];  ⍝ null: ok ht=0 sz=0 depth=0 range=⍬.
  [[key _] subs]←ω  ⍝ node info and subtrees.
  stats←≥α ∇¨ subs  ⍝ subtree stats.
  [oks szs dps hts krs]←↓⍉⊃stats
  keys←key{α,[⍶],ω}/krs
  okkey←{ω≡ιρω}⍋⊃keys
  okstr←[2 2]ₓ ≡ ρω , ρ↑⌽ω
  ok←okkey∧okstr∧∧/oks  ⍝ good tree.
  sz←≥+/szs  ⍝ subtree size.
  dp←α++/dps  ⍝ total node depth.
  ht←≥⌈/hts  ⍝ node height.
  kr←⌽2ρ¯1⌽keys  ⍝ key range for subtree.
  α>0ₓ?[ok sz dp ht kr];  ⍝ subtree: ok size tot_dep height range.
  [ok sz ⌊0.5+dp÷ₐsz ht]  ⍝ root: ok size mean_depth height.
}  ⍝ :: ∇ t → y s d h {r}

⍝ Shared by sbst, redblack and splay.
wise ← {⊽⍶ ↑ 3ρω}  ⍝ parameterise direction.

⍝ From http://dfns.dyalog.com/c_avl.htm

avl ← {  ⍝ Adelson-Velskii, Landis trees.
  get←{  ⍝ value for key ω from AVL tree α.
    α≡0?;  ⍝ key not in tree: no value.
    [[k v] _ subs]←α  ⍝ next key, value and subtrees.
    k≡ω?v;  ⍝ match: value from tree.
    dir←-/⍋⊃[ω k]
    [sub sib]←dir wise subs  ⍝ subtree to search and its sibling.
    sub ∇ ω  ⍝ value from subtree.
  }  ⍝ :: t ∇ k → v
  put←{  ⍝ tree α with key=value ω.
    α≡0?[[ω 0ₓ [0 0]] $t];  ⍝ null tree: new leaf, height incr.
    [[kv obal subs] [key val]]←[α ω]  ⍝ next node and key=val pair.
    key≡↑kv?[[ω obal subs] $f];
    [dir obv]←[1 ¯1]ₓ×-/⍋⊃[key ↑kv]
    [sub sib]←dir wise subs  ⍝ subtree to search and its sibling.
    [nsub inc]←sub ∇ ω  ⍝ new subtree and height increment.
    new←obv proj [kv obal [nsub sib]]  ⍝ tree with new node.
    ~inc?[new $f];  ⍝ no height increment: done.
    [(dir balance new) obal=0ₓ]  ⍝ height increment if OLD balance is 0.
  }  ⍝ :: t ∇ k v → t i
  rem←{  ⍝ tree α without key ω.
    α≡0?[0 $f];  ⍝ key not in tree: done.
    ω≡↑↑α?rmnode α;
    [dir obv]←[1 ¯1]ₓ×-/⍋⊃[ω ↑↑α]
    [kv obal [sub sib]]←obv proj α  ⍝ subtree to search and its sibling.
    [nsub inc]←sub ∇ ω  ⍝ new subtree and height increment.
    new←obv proj [kv obal [nsub sib]]  ⍝ tree with node removed.
    ~inc?[new $f];  ⍝ no height decrement: done.
    [nkv nbal nsubs]←obv balance new  ⍝ balanced tree.
    [[nkv nbal nsubs] nbal=0ₓ]  ⍝ height decrement if NEW balance is 0.
  }  ⍝ :: t ∇ k → t i
  rmnode←{  ⍝ node ω removed.
    [kv obal subs]←ω  ⍝ subnodes.
    0∊subs?[(subsι0)⊃⌽subs $t];  ⍝ either sub null: the other.
    [lft rgt]←subs  ⍝ left and right non-null subtrees.
    [[sk sv] _ _]←rgt limt ¯1  ⍝ successor key=val.
    [rm inc]←rgt rem sk  ⍝ right subtree with successor removed.
    new←[[sk sv] obal [lft rm]]  ⍝ target node replaced with successor.
    ~inc?[new $f];  ⍝ no height decrement: done.
    [nkv nbal nsubs]←¯1ₓ balance new  ⍝ new balanced node.
    [[nkv nbal nsubs] nbal=0ₓ]  ⍝ height decrement if NEW balance is 0.
  }  ⍝ :: ∇ t → t i
  limt←{  ⍝ ω-most node of (sub)tree α.
    sub←↑ ω wise ↑⌽α
    sub≡0?α;  ⍝ null: this.
    sub ∇ ω  ⍝ else: ω-most of sub.
  }  ⍝ :: t ∇ d → t
  balance←{  ⍝ tree ω with balancing moment α.
    [kv obal subs]←ω  ⍝ original tree.
    new←α+obal  ⍝ new balance.
    0ₓ∊[obal new]?[kv new subs];  ⍝ balance bits absorb moment: done.
    [[_ Bbal _] _]←obal wise subs  ⍝ otherwise:
    Bbal≠-obal?-obal srot ω;  ⍝ single or
    -obal drot ω  ⍝ double rotation.
  }  ⍝ :: m ∇ t → t
  srot←{                                     ⍝ single α-rotation of tree ω.
    [Akv Abal [B r]]←α proj ω                ⍝    <<A         yB>
    [Bkv Bbal [p q]]←α proj B                ⍝     / \        / \      where x y z
    AAbal←-α×Bbal=0ₓ                         ⍝   <Bx  r  =>  p  <Az    are such that:
    BBbal←+α×Bbal=0ₓ                         ⍝   / \            / \    <B> →  B> <A
    AA←α proj [Akv AAbal [q r]]              ⍝  p   q          q   r   <B  → <B> <A>
    α proj [Bkv BBbal [p AA]]
  }                                          ⍝ :: d ∇ t → t

  drot←{                                     ⍝ double α-rotation of tree ω.
    [Akv Abal [B s]]←α proj ω                ⍝    <<A         <<A          <C>
    [Bkv Bbal [p C]]←α proj B                ⍝     / \         ⌿ \         / \
    [Ckv Cbal [q r]]←α proj C                ⍝    B>  s  =>  <C.  s  =>  <By xA>
    AAbal←α×Cbal=-α                          ⍝   / ⍀         / \         / \ / \
    BBbal←-α×Cbal=α                          ⍝  p  xCy     <B.  r       p  q r  s
    BB←α proj [Bkv BBbal [p q]]              ⍝     / \     / \
    AA←α proj [Akv AAbal [r s]]              ⍝    q   r   p   q
    α proj [Ckv 0ₓ [BB AA]]                  ⍝                   where:
  }                                          ⍝ :: d ∇ t → t      x y∊"<>" "< " " >"
  chk←{  ⍝ tree stats / integrity check.
    0ₓ=≡ω?[ω≡0 0ₓ 0ₓ 0ₓ ⍬];  ⍝ null: ok maxbal=0 height=0 key-range.
    [[key _] bal subs]←ω  ⍝ key, balance and subtrees.
    stats←≥α ∇¨ subs  ⍝ subtrees stats.
    [oks szs dps hts krs]←↓⍉⊃stats
    keys←key{α,[⍶],ω}/krs
    okkey←{ω≡ιρω}⍋⊃keys
    okhgt←bal=--/hts  ⍝ balance is height difference.
    okbal←bal∊[¯1 0 1]ₓ  ⍝ balance is in range.
    ok←okkey∧okbal∧okhgt∧∧/oks  ⍝ subtree is good.
    sz←≥+/szs  ⍝ subtree size.
    dp←α++/dps  ⍝ total node depth.
    ht←≥⌈/hts  ⍝ subtree height.
    kr←⌽2ρ¯1⌽keys  ⍝ key range for subtree.
    α>0ₓ?[ok sz dp ht kr];  ⍝ subtree: ok size tot_dep height range.
    [ok sz ⌊0.5+dp÷ₐsz ht]  ⍝ root: ok size mean_depth height.
  }  ⍝ :: ∇ t → y s d h {r}
  fmt←{  ⍝ formatted tree ω, its key=value after decoration α.
    ω≡0?0 0ρ"";  ⍝ null tree: null format.
    [[key val] bal subs]←ω  ⍝ node info.
    deco←≥bal ⊃ "><" '─' "<>"
    α,,/(⍬↣⍕)¨[key '=' val] hang deco∇¨subs
  }  ⍝ :: ∇ t → [-;]
  proj←{α=[0 0 ¯1]ₓ ⌽¨ ω}  ⍝ α-projection of node ω.
  wise←{α=1ₓ ⌽ ω}  ⍝ subtrees ω in -α, +α order.
  '∪'≡⍶?↑ α put ω;
  '⍎'≡⍶?α get ω;  ⍝ value for key α in tree ω.
  '~'≡⍶?↑ α rem ω;
  '⍕'≡⍶?""fmt ω;  ⍝ formatted tree ω.
  '∊'≡⍶?vec ω;  ⍝ vector of key=value pairs for tree ω.
  '?'≡⍶?4↑ 0ₓ chk ω  ⍝ stats for tree ω: ok size dpth height.
}

⍝ From http://dfns.dyalog.com/s_avl.htm

fibtree ← {  ⍝ Depth-ω worst-case fibonacci tree.
  ↑1ₓ{
    ω=0ₓ?[0 α];  ⍝ f 0 → []
    ω=1ₓ?[[α 0ₓ [0 0]] ≥α];  ⍝ f 1 → α [] []
    [l m]←α ∇ ω-2ₓ  ⍝ left subtree and next value
    [r n]←≥m ∇ ≤ω  ⍝ right ..  ..  ..  ..  ..
    [[m 1ₓ [l r]] n]  ⍝ f ω+1 → α [f ω-2] [f ω-1]
  }ω  ⍝ :: tree next ← next ∇ depth
}

⍝ From http://dfns.dyalog.com/c_sbst.htm

sbst ← {  ⍝ Simple Binary Search Trees.
  put←{  ⍝ tree α with key=value ω.
    α≡0?[[ω [0 0]] 0];  ⍝ null: new node.
    [[[nxt _] subs] [key _]]←[α ω]  ⍝ node and search key/val.
    nxt≡key?[[ω subs] 0];  ⍝ match: new value.
    α ∇ search ω  ⍝ natch: try subtrees.
  }  ⍝ :: t ∇ k v → t _
  get←{  ⍝ value for key ω from tree α.
    α≡0?[α '?'];  ⍝ null: key not in tree: no value.
    [[[nxt val] _] [key _]]←[α ω]  ⍝ node and search key.
    nxt≡key?[α val];  ⍝ match: tree & value.
    α ∇ search ω  ⍝ natch: try subtrees.
  }  ⍝ :: t ∇ k _ → _ v
  rem←{  ⍝ tree α without key ω.
    α≡0?[α 0];  ⍝ null: key not in tree: no change.
    [[[nxt _] subs] [key _]]←[α ω]  ⍝ node and key.
    ~nxt≡key?α ∇ search ω;  ⍝ natch: value.
    0 0≡subs?0 0;  ⍝ leaf: done.              X~X → Y
    0∊subs?[(subsι0)⊃⌽subs 0];  ⍝ one null: use other.    /       \
    rrotα ∇ ω  ⍝ remove from rgt-rotn:  Y         X~X
  }  ⍝ :: t ∇ k _ → t _
                         ⍝    B   →   A
  rrot←{                 ⍝   / \     / \
    [B [[A [p q]] r]]←ω  ⍝  A   r   p   B
    [A [p [B [q r]]]]    ⍝ / \         / \
  }                      ⍝ p   q       q   r
  search←{  ⍝ search subtree α for key ⊃ω.
    [inf [lft rgt]]←α  ⍝ parts of node.
    dir←1ₓ-⊽>/⍋⊃↑¨[inf ω]
    [_ nxt]←dir wise [lft rgt]  ⍝ nxt subtree to search.
    [sub val]←nxt ⍶ ω
    subs←dir wise [lft sub rgt]  ⍝ lft and rgth subtrees.
    [[inf subs] val]  ⍝ new node and value.
  }  ⍝ :: t (t ∇ k v → t v) ⍢ k v → t v
  bal←{  ⍝ dsw-balancing.
    [vine size]←[0 0ₓ] list ω  ⍝ vine of 0-leaves and size.
    log←⌊ₓ2⊛≥size  ⍝ largest complete tree ≤ ω.
    rem←≥size-2ₓ*log  ⍝ no of surplus nodes.
    cmps←¯2ₓ+2ₓ*≥ιlog
    vine cmp/ 1↓cmps,⊽rem
  }  ⍝ :: ∇ t → t
  cmp←{  ⍝ compress of alternate vine sections.
    α=0ₓ?ω;  ⍝ far enough: terminal leaf.
    [inf [lft rgt]]←ω  ⍝ parts of node.
    lev←≤α ∇ lft  ⍝ leftmost vine leaf.
    2ₓ|α?[inf [lev rgt]];  ⍝ copying of alternate vine sections.
    rrot [inf [lev rgt]]  ⍝ rotation of alternate vine sections.
  }  ⍝ :: n ∇ v → t
  list←{                                     ⍝ list (0-vine) from tree ω.         /
    0≡ω?α;                                    ⍝ null: accumlated vine.    /       C
    [inf [lft rgt]]←ω                        ⍝ node info & subtrees.    B   →   /
    [lev s]←α ∇ lft                          ⍝ left vine & size,       / \     B
    [[inf [lev 0]] ≥s]∇ rgt                 ⍝ ++ right vine.         A   C   /
  }                                          ⍝ :: v ∇ t → v s                A
  '∪'≡⍶?↑ α put ω;
  '⍎'≡⍶?↑⌽ ω get [α 0];
  '~'≡⍶?↑ α rem [ω 0];
  '⍕'≡⍶?fmt ω;  ⍝ formatted tree.
  '∊'≡⍶?vec ω;  ⍝ vector of key=value pairs.
  '?'≡⍶?4↑ 0ₓ chk ω;  ⍝ tree stats and integrity check.
  '='≡⍶?bal ω  ⍝ balanced tree ω.
}

⍝ From http://dfns.dyalog.com/c_redblack.htm

redblack ← {  ⍝ Red-black trees.
  ins←{  ⍝ tree α with key=val ω.        [ins]
    α≡0?base [ω $t [0 0]];  ⍝ null: new <red> node and path.
    [[[nxt _] red subs] [key _]]←[α ω]  ⍝ parts of node.
    nxt≡key?done [ω red subs];  ⍝ match: new value, long path.
    [node path]←α ∇ search ω  ⍝ natch: search subtrees.
    2ₓ≠ρpath?[node path];  ⍝ not grandparent: continue.
    [p c]←path  ⍝ parent and child sides.
    ~ node isred p?[node path];  ⍝ parent black [insB]: no change.
    node isred -p?node insRR p;  ⍝ uncle red [insRR]:
    p=c?node insRBo p;  ⍝ C is P's outer child [insRBo]:
    (node rot↢p sub p)insRBo p  ⍝ C is P's inner child [insRBi]:
  }  ⍝ t p ← t ∇ k v
  insRR←{  ⍝ red uncle.                    [insRR]
    g←flip α  ⍝ [G] → <G>
    u←g flip sub ω  ⍝ <U> → [U]
    base u flip sub -ω  ⍝ <P> → [P]
  }  ⍝ :: t ∇ p → t p
  insRBo←{  ⍝ C is outer child.             [insRBo]
    g←flip α  ⍝ [G] → <G>
    p←g rot -ω  ⍝ ⌽ G-P
    done flip p  ⍝ <P> → [P]
  }  ⍝ :: t ∇ p → t p
  get←{  ⍝ value for key ω from tree α.
    α≡0?;  ⍝ key not in tree: no value.
    [[nxt val] _ subs]←α  ⍝ next key, value and subtrees.
    nxt≡ω?val;  ⍝ match: value from tree.
    (>/⍋⊃[ω nxt] ⊃ subs)∇ ω
  }  ⍝ v ← t ∇ k
  rem←{  ⍝ remove key ω from tree α.
    α≡0?done0;  ⍝ null: key not in tree.
    [[nxt _] red [lft rgt]]←α  ⍝ node key and subtrees.
    ~nxt≡↑ω?bal α ∇ search ω;
    0≡lft?bal rgt rep red;  ⍝ no left subtree: replace with right.
    0≡rgt?bal lft rep red;  ⍝ no right subtree: replace with left.
    kv←left rgt  ⍝ two subtrees: right-successor node.
    [sub path]←rgt ∇ kv  ⍝ right subtree with successor removed.
    bal [[kv red [lft sub]] 1ₓ,path]  ⍝ replaced with successor's key=val.
  }  ⍝ :: t ∇ k _ → t p
  left←{  ⍝ left-most key=val in subtree ω.
    [kv _ [lft _]]←ω  ⍝ key=val and left subtree.
    0≡lft?kv;  ⍝ no left node: successor key.
    ∇ lft  ⍝ examine left subtree.
  }  ⍝ :: ∇ t → kv
  rep←{  ⍝ repaint deleted node's child. [rep]
    ω?done α;  ⍝ red X: [C]                    [repR]
    α≡0?dblk0;  ⍝ child null: [[∘]]             [repBB]
    [inf red subs]←α  ⍝ child non-null.
    red?done [inf $f subs];  ⍝ child red: <C> →  [C]         [repBR]
    dblk α  ⍝ child blk: [C] → [[C]]        [repBB]
  }  ⍝ :: t ∇ r → t p
  bal←{  ⍝ double-black balancing.       [bal]
    [sub path]←ω  ⍝ subtree and path to new child.
    2ₓ≠ρpath?ω;  ⍝ not parent: continue.
    [dir ddblk]←path×[¯1 1]ₓ  ⍝ direction and 0 => double-black.
    ddblk≠0ₓ?ω;  ⍝ not double black: continue.
    sub isred dir?sub balR dir;  ⍝ red sibling: [balR]
    sub balB dir  ⍝ blk sibling: [balB]
  }  ⍝ :: ∇ t p → t p
  balR←{  ⍝ sibling red                   [balR]
    p_0←flip α  ⍝ [P] → <P>
    p_1←p_0 rot -ω  ⍝ ⌽ P-S
    p_2←flip p_1  ⍝ <S> → [S]
    [Pinf Pred Psubs]←p_2  ⍝ new parent node.
    [N S]←ω wise Psubs  ⍝ [[N]] and [S]
    [Nn path]←N balB ω  ⍝ sibling now black: [balB]
    P_subs←ω wise [Nn S]  ⍝ new subtrees.
    [[Pinf Pred P_subs] path]  ⍝ balanced tree.
  }  ⍝ :: t ∇ r → t p
  balB←{  ⍝ sibling black                 [balB]
    [near far]←α↣isred¨ω×1ₓ,¨[¯1 1]ₓ  ⍝ nephew colours.
    near⊽far?α balBbb ω;  ⍝ both nephews black: [balBbb]
    far?α balB_r ω;  ⍝ far nephew red: [balB_r]
    α balBrb ω  ⍝ far nephew black: [balBrb]
  }  ⍝ :: t ∇ r → t p
  balBbb←{  ⍝ both nephews black            [balBbb]
    pred←α isred ⍬  ⍝ parent colour.
    p_0←α flip sub ω  ⍝ [S] → <S>
    pred?done flip p_0;  ⍝ <P> → [P]
    dblk p_0  ⍝ [P] → [[P]]
  }  ⍝ :: t ∇ r → t p
  balB_r←{  ⍝ far nephew red.               [balB_r]
    colr←{[kv _ lr]←α ◇ [kv ω lr]}  ⍝ ω-coloured tree α.
    pred←α isred ⍬  ⍝ parent colour.
    p_0←α colr $f  ⍝ (P) → [P]
    p_1←p_0 rot -ω  ⍝ ⌽ P-S
    p_2←p_1 flip sub ω  ⍝ <f> → [f]
    p_3←p_2 colr pred  ⍝ (S) → (P)
    done p_3  ⍝ balance resolved.
  }  ⍝ :: t ∇ r → t p
  balBrb←{  ⍝ far nephew black              [balBrb]
    p_0←α flip sub ω  ⍝ [S] → <S>
    p_1←p_0 rot↢ω sub ω  ⍝ ⌽ S-n
    p_2←p_1 flip sub ω  ⍝ <n> → [n]
    p_2 balB_r ω  ⍝ [balB_r]
  }  ⍝ :: t ∇ r → t p
  root←{  ⍝ root:
    0≡↑ω?0;
    [[inf _ subs] _]←ω  ⍝ tree and path.
    [inf $f subs]  ⍝ root: black - rule [I].
  }  ⍝ t ← ∇ t p
  fmt←{  ⍝ formatted tree ω.
    ω≡0?⊃,↓"[∘]";  ⍝ null tree: [∘].
    [[key val] red subs]←ω  ⍝ node info.
    [l r]←red⊃"[]" "<>"
    ⊃,/(⍬↣⍕)¨[l key '=' val r] hang ∇¨subs
  }  ⍝ :: ∇ t → [-;]
  chk←{  ⍝ tree statistics.
    0ₓ=≡ω?[0≡ω 0ₓ 0ₓ 0ₓ ⍬ 1ₓ $t];  ⍝ null: ok size dep ht range blks isblk.
    [[key _] red subs]←ω  ⍝ parts of node.
    blk←~red  ⍝ black node.
    stats←≥α ∇¨ subs  ⍝ subtree stats.
    [oks ss ds hs ks bs bks]←↓⍉⊃stats
    keys←key{α,[⍶],ω}/ks
    okK←{ω≡ιρω}⍋⊃keys
    okR←blk∨∧/bks  ⍝ check rule [R].
    okB←=/bs  ⍝ check rule [B].
    ok←okK∧okR∧okB∧∧/oks  ⍝ subtree ok.
    kr←⌽2ρ¯1⌽keys  ⍝ key range.
    blks←blk+⌈/bs  ⍝ black count increment.
    [ht sz]←≥(⌈/hs),+/ss  ⍝ tree height and size.
    dp←α++/ds  ⍝ total depths.
    α>0ₓ?[ok sz dp ht kr blks blk];  ⍝ subtree stats.
    [ok sz ⌊0.5+dp÷ₐsz ht]  ⍝ root: ok size mean_depth height.
  }  ⍝ ok size tot_dep ht rng blks blk ← ∇ t
  search←{  ⍝ search subtree α for key ⊃ω.
    [inf red [lft rgt]]←α  ⍝ parts of node.
    dir←1ₓ-⊽>/⍋⊃↑¨[inf ω]
    [_ nxt]←dir wise [lft rgt]  ⍝ nxt subtree to search.
    [sub path]←nxt ⍶ ω
    subs←dir wise [lft sub rgt]  ⍝ new subtrees.
    [[inf red subs] dir,path]  ⍝ new node and extended path.
  }  ⍝ :: t (t ∇ k v → t p) ⍢ k v → t p
  rot←{                                      ⍝ ω-rotation of node α.
    [Ninf Nred Nsubs]←α                      ⍝        N   →   L_
    [[Linf Lred Lsubs] R]←ω wise Nsubs       ⍝       ⌿ \     / \
    [ll lr]←ω wise Lsubs                     ⍝      L   R  ll   N_
    Nn←[Ninf Nred (ω wise [lr R])]           ⍝     / \         / \
    [Linf Lred (ω wise [ll Nn])]             ⍝   ll   lr     lr   R
  }                                          ⍝ :: t ∇ r → t
  sub←{  ⍝ apply ⍶ to subtree ω of tree α.
    [inf red subs]←α  ⍝ node info and subs.
    [lft rgt]←ω wise subs  ⍝ right subtree is target.
    sub←⍶ rgt
    [inf red (ω wise [lft sub])]  ⍝ reassembled tree.
  }
  isred←{  ⍝ colour for subtree α, path ω.
    [inf col [lft rgt]]←α  ⍝ (possibly null) node colour and subs.
    ω≡⍬?col;  ⍝ colour of (possibly null) node.
    (↑⌽ ↑ω wise [lft rgt])∇ 1↓ω
  }  ⍝ :: t ∇ p → red
  flip←{[kv b lr]←ω ◇ [kv ~b lr]}  ⍝ flip colour of node ω.
  done←{[α ω]}↢[0 0 0]ₓ  ⍝ node with long path.
  base←{[α ω]}↢⍬  ⍝ new child: node with null path.
  dblk←{[α ω]}↢[0ₓ]  ⍝ double black node.
  '∪'≡⍶?root α ins ω;  ⍝ insert/replace value in tree.
  '~'≡⍶?root α rem [ω 0];  ⍝ remove key=value from tree.
  '⍎'≡⍶?ω get α;  ⍝ search for value for key.
  '⍕'≡⍶?fmt ω;  ⍝ formatted tree.
  '∊'≡⍶?vec ω;  ⍝ vector of key=value pairs.
  '?'≡⍶?4↑ 0ₓ chk ω  ⍝ tree stats: ok size mean_depth height.
}

⍝ From http://dfns.dyalog.com/c_splay.htm

splay ← {  ⍝ Splay trees.
  put←{  ⍝ tree α with key=value ω.
    α≡0?[ω [0 0]];  ⍝ null: new leaf.
    [[[nxt _] subs] [key _]]←[α ω]  ⍝ node info and subtrees.
    nxt≡key?[ω subs];  ⍝ match: new value.
    α ∇ search ω  ⍝ natch: search subtrees.
  }  ⍝ :: t ∇ k v → t
  rem←{  ⍝ tree α without key ω.
    α≡0?0;  ⍝ null: key not in tree: no change.
    [[[nxt _] subs] [key _]]←[α ω]  ⍝ node and search key.
    ~nxt≡key?α ∇ search ω;  ⍝ natch: remove from subtree.
    0 0≡subs?0;  ⍝ leaf: replace with null.
    0∊subs?subsι0 ⊃ ⌽subs;  ⍝ one null: use the other.
    (α rot 1ₓ)∇ ω  ⍝ neither: remove from rotated tree.
  }  ⍝ :: t ∇ k → t
  search←{  ⍝ search subtree α for key ⊃ω.
    [inf [lft rgt]]←α  ⍝ parts of node.
    dir←1ₓ-⊽>/⍋⊃↑¨[inf ω]
    [_ nxt]←dir wise [lft rgt]  ⍝ nxt subtree to search.
    sub←nxt ⍶ ω
    [inf (dir wise [lft sub rgt])]  ⍝ new node.
  }  ⍝ :: t (t ∇ k _ → t) ⍢ k _ → t
  get←{  ⍝ value for key ω from tree α.
    α≡0?0 0 0;  ⍝ null: no value.
    [[key val] [lft rgt]]←α  ⍝ parts of node.
    key≡ω?[val α ⍬];  ⍝ match: value tree path.
    dir←1ₓ-⊽>/⍋⊃[key ω]
    [_ nxt]←dir wise [lft rgt]  ⍝ next subtree to search.
    [rslt sub path]←nxt ∇ ω  ⍝ value, subtree and path to value.
    ∆path←dir,path  ⍝ extended path to target.
    cand←[[key val] [lft sub rgt]]  ⍝ node info and subtree candidates.
    tree←(dir wise)\cand bal ∆path  ⍝ possibly rebalanced tree.
    [rslt tree ∆path]  ⍝ value, tree and path to value.
  }  ⍝ :: t ∇ k → v t p
  bal←{  ⍝ improve balance of tree.
    2ₓ≠ρω?α;  ⍝ not grandchild: continue.
    [pos neg]←[1 ¯1]ₓ×↑ω
    =/ω?(α rot neg)rot neg;  ⍝ same dirns: double ¯ω-rotation.
    [C [BpA s]]←(neg wise)\α  ⍝ diff dirns:
    ∆C←(neg wise)\[C [(BpA rot pos) s]]
    ∆C rot neg  ⍝ ¯ω-ω-rotation.       (see notes)
  }  ⍝ :: t ∇ p → t
  rot←{  ⍝ single ω-wise rotation of tree α.
    [B [Apq r]]←(ω wise)\α  ⍝     B       A
    [A [p q]]←(ω wise)\Apq  ⍝    / \  →  / \
                            ⍝   A   r   p   B
    Bqr←(ω wise)\[B [q r]]  ⍝  / \         / \
    (ω wise)\[A [p Bqr]]    ⍝ p   q       q   r
  }  ⍝ :: t ∇ r → t
  lift←{  ⍝ lift child of root.
    [val root path]←ω  ⍝ val and revised tree.
    0∊path?;  ⍝ no value: quit.
    1ₓ≠ρpath?[val root];  ⍝ val not child of root: done.
    [val (root rot -↑path)]
  }  ⍝ :: ∇ v t p → v t
  dep←{  ⍝ depth of key ω in tree α.
    α≡0?0ₓ;  ⍝ key not found: failure.
    [[key val] subs]←α  ⍝ parts of tree.
    key≡ω?1ₓ;  ⍝ key found: at depth 1.
    dir←1-⊽>/⍋⊃[key ω]
    [_ sub]←dir wise subs  ⍝ next subtree to search.
    {ω+×ω} sub ∇ ω  ⍝ incremental depth.
  }  ⍝ :: t ∇ k → d
  '∪'≡⍶?α put ω;  ⍝ insert/replace value in tree.
  '⍎'≡⍶?lift ω get α;  ⍝ search for value for key.
  '~'≡⍶?α rem [ω 0];  ⍝ remove key=value from tree.
  '?'≡⍶?4↑ 0ₓ chk ω;  ⍝ tree stats: ok size depth height.
  '⍕'≡⍶?fmt ω;  ⍝ formatted tree.
  '∊'≡⍶?vec ω;  ⍝ list of key=value pairs.
  '≡'≡⍶?ω dep α  ⍝ depth of key α in tree ω.
}

⍝ From http://dfns.dyalog.com/c_tfmt.htm

tfmt ← {  ⍝ Char matrix from tree.
  α←""  ⍝ default: no indentation.
  1ₓ=≡,ω?⊃,↓α,ω;
  node←α,↑ω
  subs←α,4↑'·' ↣∇¨1↓ω  ⍝ subtrees.
  ⊃[node],,/↓¨subs
}

⍝ From http://dfns.dyalog.com/c_tnest.htm

tnest ← {  ⍝ Array from TreeView style tree.
  [dvec ivec]←ω  ⍝ depth and items vectors.
  1ₓ=≢dvec?↑ivec;
  node←1↑ivec  ⍝ tree: node value.
  [dsub isub]←1ₓ=dvec ↣⊂¨ω  ⍝ sub treeviews.
  node,∇¨↓⍉[≤dsub◇isub]
}

⍝ From http://dfns.dyalog.com/c_trav.htm

trav ← {  ⍝ Generic depth-first tree traversal.
  (α⍶ω)∇⍨/⌽α⍹ω  ⍝ visits parent before children.
}

⍝ From http://dfns.dyalog.com/c_ravt.htm

ravt ← {  ⍝ Generic depth-first tree traversal.
  α∇⍨/⌽α⍹ω ⍶ ω  ⍝ visits children before parent.
}

⍝ From http://dfns.dyalog.com/c_tview.htm

tview ← {  ⍝ TreeView style tree from nested array.
  α←0ₓ  ⍝ default depth 0.
  1ₓ=≡,ω?α,⊂,⊂ω;  ⍝ atom: leaf node depth and value.
  node←α,⊂1↑ω  ⍝ tree:      node depth and value.
  subs←≥α ∇¨ 1↓ω  ⍝ sub-trees.
  ,⌿node⍪⊃subs
}
