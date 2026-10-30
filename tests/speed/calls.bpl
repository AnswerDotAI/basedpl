⍝⍝ setup
⍝ Fibonacci numbers by direct recursion.
fib←{⍵<2ₓ?⍵;∇⍵-1ₓ + ∇⍵-2ₓ}
n←65536
v←¿n⍴0
⍝ A dfn that writes one item of its argument in place where it holds the argument alone.
step←{[[0]]⌷⍵ ←0 ⋄ ⍵}
⍝⍝ calls
⍝ `fib 16ₓ` makes 3193 calls of a dfn, each with up to four primitive calls on single numbers. Its cost is mostly the evaluator's own work for each call.
⍝ Expected: 64 copies: 3,193 dfn calls of about 150 ns each, for a frame, a guard and four scalar primitive calls.
⍝ Linux expects 109 copies, because scalar work costs about 1.7 times as many copies on CI's x86 runner, as `v⍳w`'s comment in `arrays.bpl` says.
⍝⍝⍝ held macos 105 linux 189 wasm 156: each dfn call takes about 270 ns on macOS against the expected 150 (open.md, "Dfn calls")
fib 16ₓ
⍝⍝: 96	170	96

⍝ A train whose tine is a dfn that writes one item of its argument, in the spelling that the style page prefers to `1+(step ⍵-0.5)`. The train gives its argument to `step`, which writes in place.
⍝ Expected: 4.6 copies, what `{1+(step ⍵-0.5)} v` costs on macOS: two passes for `-` and `+`, and `step`'s call, which writes its item in place.
⍝⍝⍝ allows forced "←": an indexed write computes its pending target whole before it writes
{1+step ⍵-0.5} v
⍝⍝: 9.2	9.2	9.2
