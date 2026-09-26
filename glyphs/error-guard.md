

# `::` — Error guard

`codes::handler` catches matching later errors in a dfn. `0::` catches
ordinary APL errors; a vector selects several error numbers.

``` apl
{0::42 ⋄ 1÷⍵}0     ⍝ 42
{11::0 ⋄ ÷⍵}0      ⍝ 0
```

The handler sees local bindings restored to guard installation. Outer
writes and output remain. The selected guard is inactive in its handler.

``` apl
{x←1 ⋄ 0::x ⋄ x←2 ⋄ 1÷0}0 ⍝ 1
```

Cancellation and unsupported-feature errors bypass guards.

## Signal

`•signal "DOMAIN ERROR"` raises an ordinary APL error. It is monadic and
does not return a value. Existing `::` guards catch it in the same way
as an error from a primitive.

``` apl
positive←{⍵≤0:•signal "DOMAIN ERROR" ⋄ ⍵}
safe←{11::0 ⋄ positive ⍵}
safe ¯3   ⍝ 0
safe 4    ⍝ 4
```

The argument must be one of these uppercase names: `SYNTAX ERROR`,
`INDEX ERROR`, `RANK ERROR`, `LENGTH ERROR`, `VALUE ERROR`,
`LIMIT ERROR`, or `DOMAIN ERROR`. Their guard numbers are 2, 3, 4, 5, 6,
10 and 11 respectively. Numeric codes and other names are rejected with
`DOMAIN ERROR`. Interrupts, timeouts and unsupported-feature errors
cannot be signalled.
