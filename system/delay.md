

# `•delay` — Delay

`•delay s` pauses for `s` seconds and returns the seconds it actually
waited, as a [shy](../terms.qmd#term-shy) result. An interrupt stops it,
and `•delay ∞` waits for one.

Errors: `DOMAIN` for a negative number of seconds, or an `s` that isn’t
a number.

The browser build has no `•delay`.
