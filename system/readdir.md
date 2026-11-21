

# `•readdir` — List a directory

`•readdir dir` gives a table with the columns of
[`•metadata`](metadata.qmd) and a row for each entry of `dir`, sorted by
path. Each row’s `path` is `dir` joined with the entry’s path within
`dir`. With a glob on the left, `•readdir` keeps only the entries whose
path within `dir` matches the glob, as in `"*.csv" •readdir "data"`. As
in a shell, `*` matches within one directory and `**` matches across
directories. Options on the left: `glob`, and `recurse` (`1` also lists
the entries of subdirectories).

Errors: `IO` for a directory that can’t be read; `DOMAIN` for an invalid
glob or option.
