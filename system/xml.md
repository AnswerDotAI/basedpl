

# `•xml` — XML text

`•xml text` reads XML into an element tree with the same meaning.
Attribute values become text. Names keep their prefixes, and namespace
declarations become `xmlns` attributes. Comments, processing
instructions and text that is only whitespace are dropped. [XML and
SVG](../xml.ipynb) covers element trees.

`•xml⁻ tree` writes the XML text of an element tree. It escapes `&`,
[`<`](https://answerdotai.github.io/basedpl/glyphs/less-than.html),
[`>`](https://answerdotai.github.io/basedpl/glyphs/greater-than.html)
and `"` in text and attribute values. An attribute whose value is a
numeric vector is written as space-separated numbers. Children are text,
elements or vectors of children. An element with no children is written
as a self-closing tag.

Errors: `DOMAIN` for malformed XML, and for invalid names or attribute
values given to `•xml⁻`.
