•prefs ["kinds":"static" "int":32]

⍝⍝ Static kinds with 32-bit integers

⍝ — The int pref sets the width at which integers compute and wrap
[2147483647ₓ+1ₓ (•storage [1 2]ₓ).kind]
¯2147483648ₓ "int32"

⍝ — Under static kinds with the int pref at 32, folds, scans and inner products wrap at 32 bits, as arithmetic does. ⌈.+ wraps each sum before comparing it
a←[2147483647 5]ₓ ⋄ [+/a +\a -/[¯2147483648 1]ₓ ×/a,2ₓ a+.×[1 1]ₓ a⌈.+[1 1]ₓ (•storage +\a).kind]
[¯2147483644ₓ [2147483647 ¯2147483644]ₓ 2147483647ₓ ¯10ₓ ¯2147483644ₓ 6ₓ "int32"]
