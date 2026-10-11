

# `•element` — XML elements

`•element tag` returns an element function for the XML tag `tag`. Call
it with attributes on the left and children on the right, as in
`["r":10] circle ""`. An empty right argument, `""` or `⍬`, gives no
children. The result is a keyed vector with `tag`, `attrs` and
`children` entries. Notebooks display it as HTML. In that HTML, the text
of a `script` or `style` element isn’t escaped, as browsers require. A
`script` element can hold JavaScript for the page. [XML and
SVG](../xml.ipynb) covers element trees.

Errors: `DOMAIN` for invalid tag or attribute names.
