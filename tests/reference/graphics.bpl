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

⍝ xml-roundtrip — Writing what •xml read gives back the same text
•xml⁻¹ •xml "<a href=""x"">hi</a>"   ⍝ "<a href=""x"">hi</a>"

⍝ xml-names — A vector of names gives a vector of element functions
[text circle]←•element ["text" "circle"]
•nc "text" "circle"   ⍝ [3 3]ₓ

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
im←•image 0.5×[0 1⋄1 0]
≢¨•mime¨[1-im ⍉im im,im +/im ⍴im]
⍝ =>
[2 2 2 1 1]ₓ

⍝ image-png — PNG bytes decode to the picture they encode
•image "png" •image⁻¹ 0.2×[0 1⋄2 3⋄4 5]
⍝ =>
0.2×[0 1⋄2 3⋄4 5]

⍝ image-jpeg — JPEG drops only alpha, so a grey picture stays grey
⍴•image "jpeg" •image⁻¹ 2 2⍴0 0.5 1 0.25
⍝ =>
[2 2]ₓ

⍝ image-animation — A rank-4 array, or frames given fps, displays as an animated PNG, and a still picture doesn't
apng←{∨/"acTL"⍷•ucs "image/png"⊃•mime ⍵}
(apng •image 2 3 4 3⍴0.5) (apng ["fps":12]•image 5 3 4⍴0.5) (apng •image 3 4⍴0.5)   ⍝ $t $t $f

⍝ image-animation-inverse — An animated PNG decodes to its frames, and a single frame to a picture
f←0.2×2 2 3 3⍴5|⍳36ₓ
g←0.2×3 2 2 1⍴5|⍳12ₓ
s←⍴•image •image⁻¹ 1 2 3 3⍴0.2
(f≡•image •image⁻¹ f) (g≡•image •image⁻¹ g) (s≡[2 3 3])   ⍝ $t $t $t

⍝ image-inverse-fps — With fps, •image⁻¹ encodes grey frames that have no channel axis
h←0.2×3 2 2⍴5|⍳12ₓ
(3 2 2 1⍴h)≡•image ["fps":12]•image⁻¹ h   ⍝ $t

⍝ image-delay — delay sets each frame's seconds, stored exactly as 1/n for a whole rate n, otherwise in milliseconds or seconds
delays←{t←⍸"fcTL"⍷•ucs ⍵ ⋄ {256⊥2 2⍴⍵}⍤1 (⊂t+⊗24+⍳4)⌷⍵}
delays ["delay":[1÷24 0.04 1.6 100]]•image⁻¹ 4 2 2⍴0.2
⍝ =>
[1 24⋄1 25⋄1600 1000⋄100 1]

⍝ image-delay-time — delay makes the first axis time, and one delay applies to every frame
h←0.2×3 2 2⍴5|⍳12ₓ
(3 2 2 1⍴h)≡•image ["delay":0.1]•image⁻¹ h   ⍝ $t

⍝ image-delay-length — delay needs one number, or one for each frame
["delay":[0.1 0.2 0.3]]•image⁻¹ 2 2 2⍴0.2
⍝ =>
⍝ error: LENGTH ERROR

⍝ image-jpeg-animation — Only PNG holds an animation
"jpeg" •image⁻¹ 2 3 4 3⍴0.5
⍝ =>
⍝ error: DOMAIN ERROR

⍝ canvas-html — A canvas is a record of its source, options and data, and displays natively as HTML that draws it
d←•canvas "(ctx, data) => ctx.fillRect(0, 0, 9, 9)"
c←["width":40] d 2 2⍴⍳4
c.options.width (∨/"<canvas>"⍷"text/html"⊃•mime c)   ⍝ 40 $t

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
