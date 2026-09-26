

# `⍪` — Table / Catenate first

`⍪Y` makes a matrix: first axis becomes rows, remaining axes become
columns.

``` apl
⍪1 2 3             ⍝ 3 1⍴1 2 3
⍴⍪2 3 4⍴0          ⍝ 2ₓ 12ₓ
```

`X⍪Y` joins on the first axis. `⍪⍠K` follows [catenate](comma.qmd).

``` apl
1 2⍪3 4            ⍝ 1 2 3 4
```
