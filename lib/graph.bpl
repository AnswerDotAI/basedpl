⍝ Graph dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/graph/graph.apl; Apache-2.0, see LICENSE-april
_array←•load "./array.bpl"

⍝⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_Graphs.htm into April APL

⍝⍝ Graph processing

⍝ From http://dfns.dyalog.com/c_assign.htm

assign ← {  ⍝ Hungarian method cost assignment.
  step_0←{step_1 ⌽⌈\⌽ρω ↑ ω}  ⍝ 0: at least as many rows as cols.
  step_1←{step_2⊃ ↓ω - ⌊/ω}
  step_2←{  ⍝ 2: mark independent zeros.
    stars←{  ⍝ independent zeros.
      ~$t∊,ω?α;  ⍝ no more zeros: done.
      next←{ω∧1ₓ=+\ω}{ω∧1ₓ=+⍀ω}ω
      mask←rows⁝next ∨ cols⁝next  ⍝ mask of dependent rows and cols.
      α∨next ∇ ω>mask  ⍝ α-accumulated star matrix.
    }
    zeros←{ω + $f stars ω}ω=0  ⍝ 1=>zero, 2=>independent zero.
    step_3 [ω zeros]  ⍝ next step: 3.
  }
  step_3←{[costs zeros]←ω  ⍝ 3: cover cols with starred zeros.
    stars←zeros=2ₓ  ⍝ starred zeros.
    covers←⊽cols stars  ⍝ covered cols.
    ~0ₓ∊,covers?stars;  ⍝ all cols covered: solution.
    step_4 [costs zeros covers]  ⍝ next step: 4.
  }
  step_4←{[costs zeros covers]←ω  ⍝ 4: adjust covering lines.
    mask←covers=0ₓ  ⍝ mask of uncovered elements.
    open←1ₓ=mask×zeros  ⍝ uncovered zeros.
    ~$t∊,open?(⌊/ ,mask # ,costs)step_6 ω;  ⍝ no uncovered zeros, next step :6.
    prime←first open  ⍝ choose first uncovered zero.
    prow←rows prime  ⍝ row containing prime.
    star←2ₓ=zeros×prow  ⍝ star in row containing prime.
    ~$t∊,star?prime step_5 {  ⍝ no star in row, next step :5,
      [costs ω prime]  ⍝ adjusted zeros matrix,
    }zeros+⊽prime;  ⍝ new primed zero (3).
    cnext←covers+prow-⊽cols⁝star  ⍝ adjusted covers.
    znext←zeros⌈3ₓ×prime  ⍝ primed zero.
    ∇ [costs znext cnext]  ⍝ adjusted zeros and covers
  }
  step_5←{[costs zeros prime]←ω  ⍝ 5: exchange starred zeros.
    star←cols⁝prime ∧ zeros=2ₓ  ⍝ next star.
    ~$t∊,star?step_3 α{  ⍝ no stars: next step :3.
      {[costs ω]}{ω-⊽ω=3ₓ}ω-α∧ω>1ₓ  ⍝ unstarred stars; starred primes.
    }zeros;  ⍝ adjusted zero markers.
    pnext←rows⁝star ∧ zeros=3ₓ  ⍝ next prime.
    α∨pnext∨star ∇ [costs zeros pnext]  ⍝ α-accumulated prime-star-··· path.
  }
  step_6←{[costs zeros covers]←ω  ⍝ 6: adjust cost matrix.
    cnext←costs+α×[¯1 1]ₓ+.×[0 3]ₓ=⊗covers
    znext←zeros + ×costs - ×cnext  ⍝ amended zeros marker.
    step_4 [cnext znext covers]  ⍝ next step: 4.
  }
  rows←{ρω ρ ↑⌽ρω # ∨/ω}
  cols←{ρω ρ ∨⌿ω}  ⍝ column propagation.
  first←{ρω ρ {ω∧1ₓ=+\ω},ω}
  ρω ↑ step_0 ω  ⍝ start with step 0.
}

⍝ From http://dfns.dyalog.com/n_alists.htm

gperm ← {⊂ω ι¨ [ω]⌷α}  ⍝ ω-permutation of vertices of graph α.

⍝ From http://dfns.dyalog.com/n_insnode.htm

insnode ← {  ⍝ Insert vertex ω in graph α.
  n←≥ω  ⍝ vertex ω is item ω, so the graph needs ω+1 items.
  n⌈≢α ↑ α,nρ⊂⍬  ⍝ extend graph with sufficient nulls.
}

⍝ From http://dfns.dyalog.com/n_remnode.htm

remnode ← {  ⍝ Remove vertex ω from graph α.
  new←ω≠ιρα # α~¨⊂[ω]  ⍝ graph with vertex ω removed,
  new-new>ω  ⍝ and edges adjusted.
}

⍝ From http://dfns.dyalog.com/n_inslink.htm

inslink ← {  ⍝ Graph α with new edge ω.
  [fm to]←ω  ⍝ edge
  ∪↢to∘,@fm α  ⍝ graph with new edge ω.
}

⍝ From http://dfns.dyalog.com/n_remlink.htm

remlink ← {  ⍝ Graph α without edge ω.
  [fm to]←ω  ⍝ edge
  ~↢[to]@fm α  ⍝ graph without edge ω.
}

⍝ From http://dfns.dyalog.com/n_search.htm

search ← {  ⍝ Breadth-first search of graph α.
  graph←α  ⍝ α is graph vector.
  (,ω){  ⍝ from starting vertex.
    ω≡⍬?α;  ⍝ no unvisited vertices: done.
    adjv←[ω]⌷graph  ⍝ adjacent vertices.
    next←∪ ,/adjv ~ α
    α,next ∇ next  ⍝ advance wave of visited vertices.
  }ω  ⍝ from starting vertex.
}

⍝ From http://dfns.dyalog.com/n_path.htm

path ← {  ⍝ Shortest path from/to ω in graph α.
  graph←α ◇ [fm to]←,¨ω  ⍝ graph and entry/exit vertex vectors
  fm{  ⍝ fm is the starting-from vertex
    α≡⍬?⍬;  ⍝ no vertices left: no path
    ∨/to∊α?⍬(⊃↢ω){  ⍝ found target: path from tree:
      ω<0ₓ?α;  ⍝ root: finished
      ω,α ∇ ⍶ ω
    }1↑α∩to;  ⍝ found vertex α
    next←[,α]⌷graph ∩¨ ⊂⍸ω=¯2ₓ
    back←,/α+0ₓ×next
    wave←,/next
    ∪wave ∇ back@wave ω  ⍝ advanced wave front
  }¯2ₓ+(ιρα)∊fm  ⍝ null spanning tree
}

⍝ From http://dfns.dyalog.com/n_span.htm

span ← {  ⍝ Breadth-first spanning tree for graph α.
  graph←α  ⍝ α is graph vector.
  (¯2ₓ + ιρα ∊ ,ω){  ⍝ α: partial spanning tree.
    ω≡⍬?α;  ⍝ no vertices: done.
    next←[ω]⌷graph ∩¨ ⊂⍸α=¯2ₓ  ⍝ untravelled edges
    back←ω+0ₓ×next  ⍝ back link per edge
    tree←(∊back)@ ∊next α  ⍝ partial spanning tree
    tree ∇ ∪∊next  ⍝ advanced wave front
  }ω  ⍝ ω: next wave of vertices to visit.
}

⍝ From http://dfns.dyalog.com/c_dfspan.htm

dfspan ← {  ⍝ Depth-first spanning tree: graph α from vertex ω.
  graph←α  ⍝ α is graph vector.
  trav←{  ⍝ initial vertex and parent
    ¯2ₓ≠α⊃ω?ω;  ⍝ vertex visited: backtrack
    next←⌽α⊃graph  ⍝ edges from vertex α
    tree←⍶@α ω  ⍝ ⍶ is α's parent
    tree (α⍢)/ next
  }  ⍝ :: tree ← vtx (vtx ⍢) tree
  ω(¯1ₓ trav)¯2ₓ¨α  ⍝ depth-first traversal of graph ω
}

⍝ From http://dfns.dyalog.com/s_scc.htm

show ← {⊃ 0⍕¨ιρω ,¨ " → "↣,¨0⍕¨ω}

⍝ From http://dfns.dyalog.com/c_scc.htm

scc ← {  ⍝ Strongly connected components (Tarjan).
  ⍝ State T: C L X x S
  ⍝ C: components; L: low-links; X: indices; x: next index; S: stack.
  TT←(3#⊂0ₓ¨G←ω),1ₓ ⍬  ⍝ state tuple T :: C L X x S
  [C L X x S]←ιρTT  ⍝ access names for items of tuple TT
  put←{(⍹⊃ω)@ ⊂[⍶ α] ω}  ⍝ ⍹ at α in field ⍶ of ω
  Lx←L put x  ⍝ α at x in lowlink vec :: T ← α ∇ T
  Xx←X put x  ⍝ α at x in indices vec :: T ← α ∇ T
  succ←{≥@x ω}  ⍝ successor of index x  :: T ←   ∇ T
  push←,@S
  α←$f ◇ trace←{⎕←0 _array.dsp α,ω ◇ ω}⍣α  ⍝ α: optional tracing   :: T ←   ∇ T
  comp←{v←α  ⍝ strongly connected component
    pops←≥+/∧\v≠stk←S⊃ω  ⍝ number of connected comps on stack
    C∆←(≥⌈/C⊃ω)@ pops↑stk C⊃ω  ⍝ extended strongly connected comps
    [pops↓stk C∆]@[S C]ω  ⍝ reduced stack; extended comps
  }  ⍝ :: T ← v ∇ T
  conn←{v←α  ⍝ connection of vertex v
    T_0←v trace ω  ⍝ optional tracing
    T_1←succ v push v Lx v Xx T_0  ⍝ successor state for x S L and X
    T_2←T_1 {w←α
      min_L←{(w⊃α⊃ω)⌊@ ⊂[L v] ω}
      0ₓ=w⊃X⊃ω?L min_L w conn ω;
      X min_L⍣ w∊S⊃ω ω  ⍝ low-link if w on stack
    }/ ⌽v⊃G  ⍝ for each edge from vertex v
    root←v⊃L⊃T_2 = v⊃X⊃T_2
    v comp⍣root T_2  ⍝ new component if root
  }  ⍝ :: T ← v ∇ T
  loop←{  ⍝ for each vertex in graph G
    vert←{α conn⍣ 0ₓ=α⊃X⊃ω ω}
    ω vert/ ⌽ιρG
  }  ⍝ :: T ← ∇ T
  ∪ι⊢ C⊃loop TT  ⍝ for each vertex
}

⍝ From http://dfns.dyalog.com/s_scc.htm

cond ← {  ⍝ Condensation of graph ω.
  c←scc ω  ⍝ strongly connected components
  v←⍷c  ⍝ component-grouped vertex indices
  e←c{⊂ω}⌸ω  ⍝ ..      ..      edges
  x←∪¨ ∊¨e ~¨ v  ⍝ out-of-component edges
  m←↓∨/¨x∊⊗v
  g←m#¨⊂ιρv  ⍝ condensed DAG
  [g v]  ⍝ ... and contracted vertices
}

⍝ From http://dfns.dyalog.com/c_stdists.htm

stdists ← {  ⍝ Spanning-tree path lengths.
  tree←ω  ⍝ spanning tree
  0ₓ{  ⍝ distance from root
    [next dvec]←ω  ⍝ chldren and distance vector
    next≡⍬?dvec;  ⍝ no children: finished
    ∆dvec←α@next dvec  ⍝ extended distance vector
    ∆next←⍸tree∊,next  ⍝ grandchildren
    ≥α ∇ [∆next ∆dvec]  ⍝ examine rest of tree
  }[ωι¯1ₓ ¯1ₓ¨ω]  ⍝ starting vertex and initial distances
}

⍝ From http://dfns.dyalog.com/n_stpath.htm

stpath ← {  ⍝ Path through spanning tree α to vertex ω.
  tree←α  ⍝ (partial) spanning tree.
  ⍬{  ⍝ path accumulator.
    ω<0ₓ?ω=¯2ₓ ↓ α;  ⍝ root or unvisited vertex: finished.
    ω,α ∇ ω⊃tree  ⍝ otherwise: prefix previous (parent) vertex.
  }ω
}

⍝ From http://dfns.dyalog.com/c_stpaths.htm

stpaths ← {  ⍝ Spanning tree paths.
  tree←ω  ⍝ spanning tree.
  root←ωι¯1ₓ  ⍝ index of root vertex.
  paths←root=ιρω ↑¨ root  ⍝ initial path vector.
  paths{  ⍝ path to current vertices.
    next←ω=⊂tree #¨ ⊂ιρtree  ⍝ vertices at next tree level.
    ⊂⍬ ∧.≡ next?α;  ⍝ all null: finished.
    exts←⊂¨[ω]⌷α ,¨¨ next  ⍝ paths to next tree level.
    indx←,/next
    paths←,/exts @indx α
    paths ∇ indx  ⍝ advance to next tree level.
  }root  ⍝ index of starting vertex.
}

⍝ From http://dfns.dyalog.com/c_X.htm

X ← {  ⍝ Exact cover: Knuth's Algorithm X.
  α←$t∨.∨ω  ⍝ all cols required by default.
  x←ιρα  ⍝ column indices.
  d←x~α#x =⊗ x
  z←{  ⍝ cover vector.
    [r c]←ρω  ⍝ number of rows and columns.
    c=0ₓ?rρ$f;  ⍝ empty matrix: success.
    n←+⌿ω  ⍝ number of covers per column.
    f←{ω∧1ₓ=+\ω}n=⌊/n #₋₁ ω
    ω ∇{  ⍝ α is constraint matrix.
      ~$t∊ω?$f;  ⍝ no rows: failure.
      f←{ω∧1ₓ=+\ω}ω
      c←,f#α  ⍝ cols selected by first row.
      r←∨/c#₋₁α  ⍝ rows covered by selected cols.
      s←⍶(~c)#₋₁(~r)#α
      s≡$f?α ∇ f<ω;  ⍝ failure: try a different row.
      f ∨ ~r #⁻ s  ⍝ success: row f included.
    } ,f  ⍝ ω is vector of marked rows.
  }ω⍪d  ⍝ exact cover.
  z≡$f?$f;  ⍝ failure: $f
  -+/~α ↓ z  ⍝ without dummy rows.
}

⍝ From http://dfns.dyalog.com/s_X.htm

sudokuMatrix ← {  ⍝ Matrix for ω ω-Sudoku puzzle.
  z←,⍠(ι6ₓ) ,⍠(6ₓ+ι4ₓ)ι10ρ⌊√ω  ⍝ cell coordinate properties.
  row←↓1 1 0 0 1 1 1 1 1 1#₋₁⊃z
  col←↓0 0 1 1 1 1 1 1 1 1#₋₁⊃z
  box←↓1 0 1 0 1 1 1 1 1 1#₋₁⊃z
  all←↓1 1 1 1 0 0 1 1 1 1#₋₁⊃z
  same←≡/∘ 1 0 0 0 1 0 0 0⊂  ⍝ matching pairs.
  same¨row,col,box,all  ⍝ constraints matrix for ω ω-puzzle.
}

sudokuX ← {[n n]←ρω  ⍝ Exact cover Sudoku solver.
  α←sudokuMatrix n  ⍝ generic ω×ω constraint matrix.
  r←∊(ω≠0ₓ)>(⊂≥ιn)=¨ω  ⍝ already placed rows.
  m←~r # α  ⍝ reduced matrix.
  f←X m  ⍝ exact cover.
  z←~r #⁻ f  ⍝ merge of placements.
  [n n]ρ z # ρz ρ ≥ιn  ⍝ solution matrix.
}

queensX ← {  ⍝ Exact cover N-Queens.
  m←ι3#ω  ⍝ cell coordinate properties.
  r←=/¨1 0 1↣#¨m  ⍝ each rank must contain one queen.
  f←=/¨0 1 1↣#¨m  ⍝ ..  file  ..     ..      ..
  dm←-/¨ι2#ω  ⍝ diagonals.
  du←<∪,dm  ⍝ unique diagonals.
  x←dm =⊗ du
  y←⊖dm =⊗ du
  m←,₀ ₁x,y,r,f  ⍝ constraints matrix.
  d←~ ι1↓ρm ∊ ι⊽ρdu  ⍝ mask of required cols.
  [ω ω]ρ d X m  ⍝ exact cover - matrix of queens.
}


⍝⍝ Weighted graph processing

wcost ← {  ⍝ Cost vector for path ω through weighted graph α.
  [graph costs]←↓α  ⍝ edges and edge-costs.
  2ₓ>≢ω?0;  ⍝ null path: no cost.
  {
    node←,α⊃graph  ⍝ exits from vertex.
    indx←nodeιω  ⍝ index of (α ω) vertex.
    indx⊃,α⊃costs  ⍝ ... associated cost.
  }/↕ω
}

⍝ From http://dfns.dyalog.com/c_wpath.htm

wpath ← {  ⍝ Quickest path fm/to ω in weighted graph α.
  [graph costs]←↓α  ⍝ graph structure and costs
  [fm to]←ω  ⍝ start and ending vertices
  tree←¯1ₓ¨graph  ⍝ initial spanning tree
  cost←0@fm ρcosts ρ ∞
  I←⊃¨↢⊂  ⍝ helper function: αth items of ω
  fm{  ⍝ from starting vertex.
    [acc to]←ω  ⍝ accumulator and next vertex
    to<0ₓ?to=¯2ₓ ↓ acc;  ⍝ root or unvisited vertex: finished
    α ∇ [to,acc to⊃α]  ⍝ otherwise: parent vertex prefix
  }{  ⍝ lowest spanning cost tree:
    [tree cost]←ω  ⍝ current tree and cost vectors
    α≡⍬?tree ⍶ [⍬ to];
    adjv←α I graph  ⍝ adjacent vertices
    accm←α I cost+costs  ⍝ costs to adjacent vertices
    best←adjv I¨ ⊂cost  ⍝ costs to beat
    mask←accm<best⌊to⊃cost  ⍝ mask of better routes
    next←mask#¨adjv  ⍝ next vertices to visit
    back←,/α+0ₓ×next
    cvec←,/mask#¨accm
    decr←{⍒cvec I ω}  ⍝ in decreasing order of cost
    wave←decr,/next
    new←[back cvec] {(decr α)@wave ω}¨ ω  ⍝ successor tree & cost
    ∪wave ∇ new  ⍝ wave spreads to adjacent vertices
  }[tree cost]  ⍝ initial spanning tree and cost vectors
}

⍝ From http://dfns.dyalog.com/n_wspan.htm

wspan ← {  ⍝ Spanning tree for weighted graph α from ω.
  [graph costs]←↓α  ⍝ graph structure and costs.
  tree←¯1ₓ¨graph  ⍝ initial spanning tree.
  cost←0@ω ρcosts ρ ∞
  I←⊃¨↢⊂  ⍝ helper function: αth items of ω
  ω{  ⍝ from starting vertex.
    [tree cost]←ω  ⍝ current tree and costs
    α≡⍬?tree;  ⍝ all vertices visited: done
    adjv←α I graph  ⍝ adjacent vertices
    accm←α I cost+costs  ⍝ cumulative cost via this vertex
    mask←accm < adjv I¨ ⊂cost  ⍝ mask of better routes
    cvec←,/mask#¨accm
    next←mask#¨adjv  ⍝ next vertices to visit
    back←,/α+0ₓ×next
    decr←(⍒cvec)I⊢  ⍝ in decreasing order of cost
    wave←decr,/next
    new←[back cvec] {(decr α)@wave ω}¨ ω  ⍝ successor tree & cost
    ∪wave ∇ new  ⍝ wave spreads to adjacent vertices
  }[tree cost]  ⍝ initial spanning tree and cost vectors
}

⍝ From http://dfns.dyalog.com/s_wmst.htm

wmst ← {  ⍝ Minimum Spanning Tree for wu-graph α.
  [graph costs]←↓α  ⍝ weighted, undirected graph
  xvec←ιρgraph  ⍝ index vector for graph
  (,ω){  ⍝ vertices inside tree: T
    [tree todo]←ω  ⍝ partial tree and unconnected vertices
    todo≡⍬?tree;  ⍝ all vertices connected: finished
    edges←graph∊¨⊂todo ∧ xvec∊α  ⍝ edges from T to G~T
    min←⌊/⌊/¨edges#¨costs  ⍝ minimum edge cost
    masks←edges∧min=costs  ⍝ lowest cost edge masks
    [fm to]←{⊃,/masks#¨ω}¨[xvec graph]
    fm≡⍬?tree;  ⍝ disjoint graph: quit
    α,to ∇ [fm@to tree;todo~to]  ⍝ vertices from G~T to T
  }[¯1ₓ¨graph xvec~,ω]  ⍝ initial tree and unconnected vertices
}
