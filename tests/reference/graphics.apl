⍝⍝ XML, SVG and MIME display

⍝ xml-tree — Elements preserve case and escape text/attributes
t←•element "text"
attrs←("data-X":"a&""b" ⋄ "x":¯2x)
•xml attrs t "x<y & y>z"
⍝ =>
"<text data-X=""a&amp;&quot;b"" x=""-2"">x&lt;y &amp; y&gt;z</text>"

⍝ xml-prefix — Prefixed names pass through unchanged
root←•element "x:root" ⋄ leaf←•element "x:leaf"
•xml ("xmlns:x":"urn:example") root leaf "Hi"
⍝ =>
"<x:root xmlns:x=""urn:example""><x:leaf>Hi</x:leaf></x:root>"

⍝ xml-numeric — Numeric attributes use XML rather than APL spelling
•xml ("points":¯2x 1r2 1E3) (•element "path") ⍬
"<path points=""-2 0.5 1000""/>"

⍝ svg-tree — SVG renders its current tree
c←•element "circle"
pic←•svg ("cx" "cy" "r":50 50 20) c ⍬
pic.children.attrs.r←30
("image/svg+xml"⊃•mime pic)≡•xml pic
⍝ =>
1ₓ

⍝ mime-field — A function in _mime_ renders the current keyed vector
total←("items":1 2 3)
total._mime_←{("text/html":"<b>",(⍕+/⍵.items),"</b>")}
total.items.[2]←10
"text/html"⊃•mime total
⍝ =>
"<b>14</b>"

⍝ json-hooks — JSON export omits keyed entries that hold functions
•tojson •svg ⍬
"{""tag"":""svg"",""attrs"":{""xmlns"":""http://www.w3.org/2000/svg"",""viewBox"":""0 0 100 100""},""children"":[]}"

⍝⍝ Plots

⍝ plot-spec — •plot keeps its data and options, and edits create nested settings
p←("mark":"bar") •plot 1 2 3
p.y.scale←"log" ⋄ p.series.a.color←"red"
(p.data ⋄ p.mark ⋄ p.y ⋄ p.series)
⍝ =>
(1 2 3 ⋄ "bar" ⋄ ("scale":"log") ⋄ ("a":("color":"red")))

⍝ plot-svg — The renderer draws the current spec as SVG
"<svg"≡4↑"image/svg+xml"⊃•mime •plot 3 1 4
⍝ =>
1ₓ

⍝ plot-unknown — Rendering reports unknown fields
p←•plot 1 2 3 ⋄ p.titel←'x' ⋄ •mime p
⍝ error: DOMAIN ERROR
