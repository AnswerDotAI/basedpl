⍝ Power dfns — adapted for BPL from April
⍝ Source: https://dfns.dyalog.com/n_contents.htm (individual sources below)
⍝ April: libraries/dfns/power/power.apl; Apache-2.0, see LICENSE-april
⍝⍝ Ported from Dyalog's dfns at http://dfns.dyalog.com/n_contents.htm into April APL

⍝ From http://dfns.dyalog.com/c_for.htm

for ← {  ⍝ Multiple selection of function list.
  (¯1↓α)⍶{1ₓ≠ρα?α ⍶ ω;
    ⍶⍣ ↑α ω
  }⍹⍣ ↑⌽α ω
}

⍝ From http://dfns.dyalog.com/c_invr.htm
invr ← {  ⍝ Approx inverse of real-valued function.
  α←1+1e¯14+0×ω
  ∆x←√1e¯14
  -↢ω∘⍶{
    ⍹ ω?ω;
    [y y∆]←⍶¨[0 ∆x]+⊂ω
    ∇ ω-y×∆x÷y∆-y  ⍝ refined estimate.
  }(ω↣≡∘⍶)α
}

⍝ From http://dfns.dyalog.com/s_limit.htm
GM ← ×/*÷∘(≢ₐ)  ⍝ Geometric mean of floats: the count is a float too.
AGM ← {↑(+/÷≢ , GM)⍣∞ ω}  ⍝ Arithmetic-geometric mean.

ArcTan ← {
  ⍝ Inverse trigonometric tangent - gqr 19-11-2002.
  ⍝ For a number ω: (ArcTan ω)≡3○⁻ω
  ⍝
  ⍝ Limit's operand function returns a 2-vector. Repeated application
  ⍝ of the function produces two sequences, both of which converge and
  ⍝ have the same limit:
  ⍝
  ⍝    A≡ {a(0),a(1), ... , a(n), ...   }
  ⍝    G≡ {g(0),g(1), ... , g(n), ...   }
  ⍝
  ⍝  Starting with arguments:
  ⍝       a(0)←(1+ω*2)*-÷2  and  g(0)←1
  ⍝  the iteration produces:
  ⍝       a(n+1)← arithmetic mean of a(n)   and g(n)
  ⍝       g(n+1)← geometric  mean of a(n+1) and g(n)
  ⍝  until:
  ⍝   a(n+1)=a(n) and g(n+1)=g(n) and a(n+1)=g(n+1)

  next←{  ⍝ next term in sequence.
    (+/÷≢ ω),GM(+/÷≢ ω),1↓ω
  }
  start←≥ω² * -÷2
  finish←↑ next⍣∞ start,1
  ω×start÷finish
}

⍝ From http://dfns.dyalog.com/s_pow.htm

rl   ← {≤2ₓ*31ₓ | ω×7ₓ*5ₓ}  ⍝ next random link  LCG(7*5, ¯1+2*31)

roll ← {⌊ω×α÷ₐ≤2ₓ*31ₓ}  ⍝ roll ω with random link α.

⍝ From http://dfns.dyalog.com/c_traj.htm

traj ← {  ⍝ Function limit 'trajectory'.
  α←⍬  ⍝ Initial null history.
  ⊂ω ∊ α?α;  ⍝ Argument in history: finished.
  α,⊂ω ∇ ⍶ ω
}

⍝ From http://dfns.dyalog.com/s_traj.htm

nr ← {  ⍝ Newton-Raphson.
  α←1e¯14
  [y ∆y]←⍶¨ω+[0 α]
  ω + α×y ÷ y-∆y  ⍝ next estimate.
}

traj_2 ← {¯1↣↓ ,↢⊂↢⍶↢↑↢⌽⍨⍣(∊⍨↢⊂↢↑↢⌽⍨)∘(,⊂) ω}

⍝ From http://dfns.dyalog.com/c_while.htm

while ← {  ⍝ Conditional function power.
  ⍹ ω?∇ ⍶ ω;
  ω  ⍝ Otherwise: finished.
}

⍝ From http://dfns.dyalog.com/c_until.htm

until ← {  ⍝ Conditional function power.
  ⍶{
    ⍹ ω?ω;
    ∇ ⍶ ω
  }⍹ ⍶ ω
}
