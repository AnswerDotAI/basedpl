⍝⍝ setup
⍝ Fibonacci numbers by direct recursion.
fib←{⍵<2ₓ?⍵;∇⍵-1ₓ + ∇⍵-2ₓ}
n←65536
⍝⍝ calls
⍝ `fib 16ₓ` makes 3193 calls of a dfn, each with up to four primitive calls on single numbers. Its cost is mostly the evaluator's own work for each call.
⍝ Expected: 64 copies: 3,193 dfn calls of about 150 ns each, for a frame, a guard and four scalar primitive calls.
⍝ Held until studied on Linux and in wasm: `fib 16ₓ` has old Linux and wasm budgets that suggest about 400 and 370, more than twice its expected cost. CI measured 156 on Linux and 131 in wasm on 22 October 2026, and 158 to 160 and 130 to 138 in the two runs of 23 October. Those budgets stay at the old value. On macOS it measured 178.86 on 21 October and 97 on 23 October. Its macOS budget is twice its expected cost.
fib 16ₓ
⍝⍝: 130	800	740
