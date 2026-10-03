

# Modules

[Glyph index](glyphs.qmd)

A BPL file that [`•load`](system-functions.qmd#load) runs is a module. A
module has names of its own. `•load` returns a record of its public
names, and adds no names to the code that loads it.

``` bpl
n←•load "lib/numeric.bpl"
n.factors 360              ⍝ 2 2 2 3 3 5
```

BPL’s modules follow BQN’s
[namespaces](https://mlochbaum.github.io/BQN/doc/namespace.html). In
BQN, a namespace is a kind of value of its own. In BPL, `•load` returns
an ordinary record.

## Taking names

Destructuring takes names from the record by key. Names the list doesn’t
mention stay in the record. [Assignment](assignment.ipynb#destructuring)
covers destructuring.

``` bpl
[factors factorial]←•load "lib/numeric.bpl"
factorial 5                ⍝ 120
```

A record holds operators as well as functions and arrays. A dot path or
a destructured name gives the operator back.

``` bpl
a←•load "lib/array.bpl"
10 -a.foldl 1 2 3          ⍝ 4
[foldl]←a
10 -foldl 1 2 3            ⍝ 4
```

Each `•load` runs the file again and returns a new record. Load a file
again after you edit it to get the new version.

## Writing a module

A module is a file of BPL statements, run in order each time it’s
loaded. Loading shows no results, but explicit `⎕←` output appears.
Every name the top level assigns is public, except a name that starts
with `_`. A private name stays out of the record. The module’s own
functions can still use it.

``` text
⍝ stats.bpl
_sq←{⍵×⍵}
mean←+/÷≢
var←{mean _sq ⍵-mean ⍵}
```

`•load "stats.bpl"` returns a record that holds `mean` and `var`. `var`
still uses `_sq` when you call it.

## Modules that use other modules

A path that starts with `./` or `../` is relative to the file that holds
the code. A module finds the files beside it this way, whatever the
working directory is. Other relative paths are relative to the working
directory. [`•nget`](system-functions.qmd#files) and `•nput` follow the
same rule. A module reads a data file beside it with
`•nget "./table.csv"`.

A module that uses another module usually keeps it under a private name.
The other module’s names then stay out of its record:

``` text
⍝ report.bpl
_stats←•load "./stats.bpl"
summary←{[_stats.mean ⍵;_stats.var ⍵]}
```

A public name exports on purpose. With `stats←•load "./stats.bpl"`, the
record of `report.bpl` would hold the `stats` record too. Destructured
names are public as well: `[mean]←•load "./stats.bpl"` makes `mean` one
of the module’s names.

A file that loads itself, directly or through other files, is a `DOMAIN`
error.

## The library

The files in `lib/` are modules. They load each other with `./` paths
and private names: `lib/graph.bpl` holds `lib/array.bpl` as `_array`,
and uses `_array.dsp`.

## Names in the session

The names of a loaded module aren’t session names. `]names` and name
completion don’t list them. Take the names you use, or keep the record
and use dot paths.

From Python, `bpl('•load "lib/array.bpl"').py` gives the record as a
dict. Its functions are `Function` objects. Its operators are `Operator`
objects, which you call with their operands to get a function:

``` python
from basedpl import bpl
a = bpl('•load "lib/array.bpl"').py
a['foldl'](bpl.fn('-'))(10, [1, 2, 3]).py   # 4
```
