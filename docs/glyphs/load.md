# `•load` — Load

`•load 'lib/numeric.apl'` evaluates a UTF-8 APL file in the current lexical scope. Paths are relative to the working directory. The repository's [APL libraries](../../lib/README.md) provide numeric, array, graph, string, power and tree dfns.

Definitions remain available after the call. Like [Execute](execute.md), Load returns the last expression's result, preserving its display suppression. Output and assignments before an error remain in effect. Diagnostics include the source filename and line.

Use the same expression in the REPL, an APL file, or Python: `apl("•load 'lib/numeric.apl'")`.
