

# `•readdir` — List a directory

`•readdir dir` gives the table of [`•metadata`](metadata.qmd) with a row
for each entry of `dir`, sorted by path. Each row’s `path` is `dir`
joined with the entry’s path within `dir`. A glob on the left keeps the
entries whose path within `dir` matches it, as in
`"*.csv" •readdir "data"`. As in a shell, `*` stays within one directory
and `**` crosses them. Options on the left: `glob`, and `recurse` (`1`
also lists the entries of subdirectories).

Errors: `IO` for a directory that can’t be read; `DOMAIN` for an invalid
glob or option.
