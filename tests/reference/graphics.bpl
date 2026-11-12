⍝⍝ XML, SVG and MIME display

⍝ xml-tree — Elements preserve case and escape text/attributes
t←•element "text"
attrs←["data-X":"a&""b" "x":¯2ₓ]
•xml⁻ attrs t "x<y & y>z"
⍝ =>
"<text data-X=""a&amp;&quot;b"" x=""-2"">x&lt;y &amp; y&gt;z</text>"

⍝ xml-prefix — Prefixed names pass through unchanged
root←•element "x:root"
leaf←•element "x:leaf"
•xml⁻ ["xmlns:x":"urn:example"] root leaf "Hi"
⍝ =>
"<x:root xmlns:x=""urn:example""><x:leaf>Hi</x:leaf></x:root>"

⍝ xml-numeric — Numeric attributes use XML rather than APL spelling
path←•element "path" ◇ •xml⁻ ["points":[¯2ₓ 1r2 1E3]] path ⍬
"<path points=""-2 0.5 1000""/>"

⍝ xml-roundtrip — Writing what •xml read gives back the same text
•xml⁻ •xml "<a href=""x"">hi</a>"   ⍝ "<a href=""x"">hi</a>"

⍝ html-raw — HTML display writes script text unescaped, closes only void elements itself, and keeps XML rules inside svg
[script div br svg]←•element "script" "div" "br" "svg"
"text/html"⊃•mime div [script "a<b && c";div ⍬;br ⍬;svg script "a<b"]
⍝ =>
"<div><script>a<b && c</script><div></div><br/><svg><script>a&lt;b</script></svg></div>"

⍝ xml-names — A vector of names gives a vector of element functions
[text circle]←•element ["text" "circle"]
•nc "text" "circle"   ⍝ [3 3]ₓ

⍝ svg-tree — SVG renders its current tree
c←•element "circle"
pic←•svg ["cx":50 "cy":50 "r":20] c ⍬
pic.children.attrs.r←30
("image/svg+xml"⊃•mime pic)≡•xml⁻ pic
⍝ =>
$t

⍝ mime-renderer — A renderer attached with •mime displays the current value
total←{["text/html":"<b>",(⍕+/ω.items),"</b>"]}ᵘ •mime ["items":[1 2 3]]
total.items₁←10
"text/html"⊃•mime total   ⍝ "<b>14</b>"

⍝ mime-keep — Pervasive and structural functions keep a renderer, and other functions drop it
im←•image 0.5×[0 1◇1 0]
≢¨•mime¨[1-im ⍉im im,im +/im ρim]
⍝ =>
[2 2 2 1 1]ₓ

⍝ mime-trace — A gradient at an array with a renderer runs from a program, as at any array
⍝⍝⍝ route trace
im←•image 0.5×[0 1◇1 0] ◇ {+/,ω×ω}∂im   ⍝ [0 1◇1 0]

⍝ image-png — PNG bytes decode to the picture they encode
•image "png" •image⁻ 0.2×[0 1◇2 3◇4 5]
⍝ =>
0.2×[0 1◇2 3◇4 5]

⍝ image-jpeg — JPEG drops only alpha, so a grey picture stays grey
ρ•image "jpeg" •image⁻ 2 2ρ0 0.5 1 0.25
⍝ =>
[2 2]ₓ

⍝ image-animation — A rank-4 array, or frames given fps, displays as an animated PNG, and a still picture doesn't
apng←{∨/"acTL"⍷•ucs "image/png"⊃•mime ω}
(apng •image 2 3 4 3ρ0.5) (apng ["fps":12]•image 5 3 4ρ0.5) (apng •image 3 4ρ0.5)   ⍝ $t $t $f

⍝ image-animation-inverse — An animated PNG decodes to its frames, and a single frame to a picture
f←0.2×2 2 3 3ρ5ₓ|ι36ₓ
g←0.2×3 2 2 1ρ5ₓ|ι12ₓ
s←ρ•image •image⁻ 1 2 3 3ρ0.2
[•image •image⁻ f;•image •image⁻ g;s]
⍝ =>
[0.2×2 2 3 3ρ5ₓ|ι36ₓ;0.2×3 2 2 1ρ5ₓ|ι12ₓ;[2 3 3]ₓ]

⍝ image-inverse-fps — With fps, •image⁻ encodes grey frames that have no channel axis
h←0.2×3 2 2ρ5ₓ|ι12ₓ
•image ["fps":12]•image⁻ h
⍝ =>
0.2×3 2 2 1ρ5ₓ|ι12ₓ

⍝ image-delay — delay sets each frame's seconds, stored exactly as 1/n for a whole rate n, otherwise in milliseconds or seconds
delays←{t←⍸"fcTL"⍷•ucs ω ◇ {256⊥2 2ρω}¹ (⊂t+⊗24ₓ+ι4ₓ)⌷ω}
delays ["delay":[1÷24 0.04 1.6 100]]•image⁻ 4 2 2ρ0.2
⍝ =>
[1 24◇1 25◇1600 1000◇100 1]

⍝ image-delay-time — delay makes the first axis time, and one delay applies to every frame
h←0.2×3 2 2ρ5ₓ|ι12ₓ
•image ["delay":0.1]•image⁻ h
⍝ =>
0.2×3 2 2 1ρ5ₓ|ι12ₓ

⍝ image-delay-length — delay needs one number, or one for each frame
["delay":[0.1 0.2 0.3]]•image⁻ 2 2 2ρ0.2
⍝ =>
⍝ error: LENGTH ERROR

⍝ image-jpeg-animation — Only PNG holds an animation
"jpeg" •image⁻ 2 3 4 3ρ0.5
⍝ =>
⍝ error: DOMAIN ERROR

⍝ canvas-html — A canvas is a record of its source, options and data, and displays natively as HTML that draws it
d←•canvas "(ctx, data) => ctx.fillRect(0, 0, 9, 9)"
c←["width":40] d 2 2ρι4
c.options.width (∨/"<canvas>"⍷"text/html"⊃•mime c)   ⍝ 40 $t

⍝ canvas-text — A canvas's text form is one line, which counts frames when the options give fps
d←•canvas "x => x"
("text/plain"⊃•mime d 1 2) ("text/plain"⊃•mime ["fps":24] d 0 1 2)   ⍝ "canvas" "canvas: 3 frames"

⍝ mime-match — Match ignores renderers
(•image 2 2ρ0.5)≡2 2ρ0.5
⍝ =>
$t

⍝ mime-type — Values given one MIME type by separate calls share its renderer, so catenation keeps it
a←"text/markdown" •mime "*a*"
b←"text/markdown" •mime "b"
≢•mime a,b   ⍝ 2ₓ

⍝ json-hooks — JSON export omits keyed entries that hold functions
•json⁻ ["a":"x" "f":{ω}]
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
