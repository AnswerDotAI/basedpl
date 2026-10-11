

# `•image` — Images

`•image Y` returns a picture: numbers from 0 to 1, with axes for rows,
columns and up to four channels, that notebooks display as an image. One
channel is grey, and three are red, green and blue. A second or fourth
channel is alpha. `Y` is the path of a PNG or JPEG file, the bytes of
one, or the numbers themselves. Values outside 0 to 1 are clipped when
the picture is displayed or encoded.

`•image⁻¹ Y` encodes the picture `Y` as PNG bytes. Its left argument
takes the options `format`, which is `"png"` or `"jpeg"`, `fps` and
`delay`. Text alone gives the format, as in `"jpeg" •image⁻¹ Y`. JPEG
drops alpha. [`•nput`](nput.qmd) writes the bytes to a file, as in
`"out.png" •nput •image⁻¹ Y`.

An animation has frames along its first axis. A rank-4 array, frames by
rows by columns by channels, is an animation. `["fps":12] •image Y` or
`["delay":0.1] •image Y` makes the first axis of `Y` frames whatever its
rank. A grey animation then needs no channel axis. `fps` sets the frame
rate, 24 frames a second by default. `delay` sets the seconds each frame
stays on screen: one number for every frame, or one per frame. It takes
precedence over `fps`. `["delay":(9⍴0.1),2] •image Y` holds the last of
10 frames for 2 seconds. Notebooks and the
[playground](../playground.qmd) play an animation as an animated PNG.
`•image⁻¹` encodes an animation as one, and takes the same options.
`•image` reads an animated PNG back as a rank-4 array, with a channel
axis even for grey frames.

Errors: `DOMAIN` for an unknown `format`, invalid image bytes, values
that aren’t real numbers, an `fps` or `delay` that isn’t positive, or an
animation encoded as JPEG; `LENGTH` for a `delay` whose length is
neither 1 nor the number of frames; `RANK` for a shape that isn’t a
picture or an animation; `IO` for file errors.
