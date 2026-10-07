⍝⍝ XML, SVG and MIME display

⍝ xml-tree — Elements preserve case and escape text/attributes
t←•element "text"
attrs←["data-X":"a&""b" "x":¯2ₓ]
•xml⁻¹ attrs t "x<y & y>z"
⍝ =>
"<text data-X=""a&amp;&quot;b"" x=""-2"">x&lt;y &amp; y&gt;z</text>"

⍝ xml-prefix — Prefixed names pass through unchanged
root←•element "x:root"
leaf←•element "x:leaf"
•xml⁻¹ ["xmlns:x":"urn:example"] root leaf "Hi"
⍝ =>
"<x:root xmlns:x=""urn:example""><x:leaf>Hi</x:leaf></x:root>"

⍝ xml-numeric — Numeric attributes use XML rather than APL spelling
•xml⁻¹ ["points":[¯2ₓ 1r2 1E3]] (•element "path") ⍬
"<path points=""-2 0.5 1000""/>"

⍝ svg-tree — SVG renders its current tree
c←•element "circle"
pic←•svg ["cx":50 "cy":50 "r":20] c ⍬
pic.children.attrs.r←30
("image/svg+xml"⊃•mime pic)≡•xml⁻¹ pic
⍝ =>
$t

⍝ mime-renderer — A renderer attached with •mime displays the current value
total←{["text/html":"<b>",(⍕+/⍵.items),"</b>"]}ᵘ •mime ["items":[1 2 3]]
total.items₁←10
"text/html"⊃•mime total   ⍝ "<b>14</b>"

⍝ mime-keep — Pervasive and structural functions keep a renderer, and other functions drop it
im←•image 0.5×[0 1 ⋄ 1 0]
≢¨•mime¨[1-im ⍉im im,im +/im ⍴im]
⍝ =>
[2 2 2 1 1]ₓ

⍝ image-png — PNG bytes decode to the picture they encode
•image "png" •image⁻¹ 0.2×[0 1 ⋄ 2 3 ⋄ 4 5]
⍝ =>
0.2×[0 1 ⋄ 2 3 ⋄ 4 5]

⍝ image-jpeg — JPEG drops only alpha, so a grey picture stays grey
⍴•image "jpeg" •image⁻¹ 2 2⍴0 0.5 1 0.25
⍝ =>
[2 2]ₓ

⍝ mime-match — Match ignores renderers
(•image 2 2⍴0.5)≡2 2⍴0.5
⍝ =>
$t

⍝ mime-type — Values given one MIME type by separate calls share its renderer, so catenation keeps it
a←"text/markdown" •mime "*a*"
b←"text/markdown" •mime "b"
≢•mime a,b   ⍝ 2ₓ

⍝ json-hooks — JSON export omits keyed entries that hold functions
•json⁻¹ ["a":"x" "f":{⍵}]
⍝ =>
"{""a"":""x""}"

⍝⍝ Plots

⍝ plot-spec — •plot keeps its data and options, and edits create nested settings
p←["mark":"bar"] •plot 1 2 3
p.y.scale←"log"
p.series.a.color←"red"
[p.data p.mark p.y p.series]
⍝ =>
[[1 2 3] "bar" ["scale":"log"] ["a":["color":"red"]]]

⍝ plot-svg — The renderer draws the current spec as SVG
"<svg"≡4↑"image/svg+xml"⊃•mime •plot 3 1 4
⍝ =>
$t

⍝ plot-unknown — Rendering reports unknown fields
p←•plot 1 2 3
p.titel←'x'
•mime p
⍝ =>
⍝ error: DOMAIN ERROR
