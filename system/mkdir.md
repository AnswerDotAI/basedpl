

# `•mkdir` — Make a directory

`•mkdir path` makes the directory `path` and any missing parents, and
returns `path`, which isn’t displayed. With the option `unique`, it then
makes a directory with a new, unique name inside `path`, and returns
that directory’s path, which is displayed, as in
`["unique":1] •mkdir •host "temp"`. The option `prefix` sets the start
of the name, as in `["unique":1 "prefix":"run"] •mkdir dir`. The
directory stays until removed.

Errors: `IO` for file errors; `DOMAIN` for an invalid option, or
`prefix` without `unique`.
