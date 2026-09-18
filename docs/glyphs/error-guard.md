# `::` — Error guard

`codes::handler` catches matching later errors in a dfn. `0::` catches ordinary APL errors; a vector selects several error numbers.

```apl
{0::42 ⋄ 1÷⍵}0     ⍝ 42
{11::0 ⋄ ÷⍵}0      ⍝ 0
```

The handler sees local bindings restored to guard installation. Outer writes and output remain. The selected guard is inactive in its handler.

```apl
{x←1 ⋄ 0::x ⋄ x←2 ⋄ 1÷0}0 ⍝ 1
```

Cancellation and unsupported-feature errors bypass guards.
