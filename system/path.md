

# `•path` — Paths

`•path text` splits a path into a record of its parts, named after
Rust’s: `parent`, `stem`, `extension` and `name`. It never reads the
disk. With the option `absolute`, as in `["absolute":1] •path text`,
`•path` first joins a relative path to the working directory, as Rust’s
`std::path::absolute` does, without resolving links. `•path⁻` joins
`parent`, `stem` and `extension` into a path, and ignores `name`. For an
array of paths, `•path` returns a record of arrays.

``` bpl
•path "data/sales.csv"                            ⍝ ["parent":"data" "stem":"sales" "extension":"csv" "name":"sales.csv"]
{ω.extension←"tsv" ◇ ω}@•path "data/sales.csv"   ⍝ "data/sales.tsv"
```

Errors: `DOMAIN` when a path isn’t text, for an unknown part, or for an
`absolute` path that is empty or has no working directory to join.
