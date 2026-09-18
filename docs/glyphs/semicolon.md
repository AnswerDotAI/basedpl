# `;` — Index-axis separator

Within indexing brackets, `;` separates axes. Empty fields select whole axes.

```apl
m←2 3⍴⍳6 ⋄ m[2;1] ⍝ 4
m←2 3⍴⍳6 ⋄ m[;2]  ⍝ 2 5
m←2 3⍴⍳6 ⋄ m[1;]  ⍝ 1 2 3
```
