

# `•load` — Load

`•load path` runs the BPL file at `path` as a [module](../modules.qmd),
with names of its own. It returns a record of the module’s public names:
the names its top level assigns that don’t start with `_`. The file adds
no names to the caller, and shows nothing except explicit output. Each
call runs the file again.

Destructure the record to take names, as in `[a b]←•load path`, or keep
it and read names with a dot, as in `m←•load path` and then `m.a`.

A path that starts with `./` or `../` is relative to the file holding
the code. Other relative paths are relative to the working directory.
`•nget` and `•nput` follow the same rule.

Errors: `IO` for a file that can’t be read; `DOMAIN` for a file that
loads itself, directly or through other files.
