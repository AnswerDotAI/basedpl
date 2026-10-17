// A canvas driver for BPL-computed drawings. Its value is a function, `(ctx, data, options)`, which draws `data` on the 2D
// context `ctx`. `•canvas` takes this source. A page that runs BPL can also call the function directly, as the gallery does.
//
// A frame is a record of a picture and up to three matrices, each matrix with one row per shape. Coordinates, sizes and widths are
// in canvas pixels. Colours and opacities run from 0 to 1. The picture is drawn first, then triangles, lines and points.
// - `image`: a picture as `•image` takes it, with rows, columns and optionally a channel axis: grey, grey and opacity, RGB, or RGB
//   and opacity. It's scaled smoothly to fill the canvas.
// - `triangles`: x1 y1 x2 y2 x3 y3 r g b opacity.
// - `lines`: x1 y1 x2 y2 width r g b opacity.
// - `points`: x y radius r g b inner outer stroke. Without a stroke, a point is a disc whose opacity runs from `inner` at its
//   centre to `outer` at its edge. A positive stroke draws a ring of that width at the `inner` opacity. The stroke column is
//   optional.
// A vector is one row. A point with a radius, or a line with a width, that isn't positive isn't drawn.
//
// Options:
// - `width` and `height` set the canvas's size.
// - `background` is a grey level or an RGB vector. Without it, the canvas is transparent.
// - `fade` is how much each frame covers the previous one with the background. 1, the default, starts each frame afresh. 0 keeps
//   everything drawn.
// - `blend`: "add" adds the light of overlapping shapes.
// - `pixelated`: with it, a picture keeps sharp pixels when it's scaled up.
// - `fps`: with it, `data` is a vector of frames, played in a loop at that rate. Without it, `data` is one frame.
(() => {
    // The canvases drawn on. The first frame drawn on a canvas covers it completely.
    const drawn = new WeakSet();
    const rgba = (r, g, b, a) => `rgba(${r * 255},${g * 255},${b * 255},${a})`;
    // Calls `f` with the data, offset and length of each row of the matrix `m`.
    const rows = (m, f) => {
        if (!m) return;
        const [n, k] = m.shape.length === 1 ? [1, m.shape[0]] : m.shape;
        for (let o = 0; o < n * k; o += k) f(m.data, o, k);
    };

    // A canvas that holds the frame's picture at its own size, before it's scaled onto the canvas drawn on.
    let scratch;
    // Draws the picture `m` scaled to fill `ctx`'s canvas, smoothly unless `pixelated`.
    const picture = (ctx, m, pixelated) => {
        if (!m?.data.length) return;
        const [h, w, c = 1] = m.shape, d = m.data, pixels = new Uint8ClampedArray(4 * h * w);
        // A second or fourth channel is opacity. Grey has one value for red, green and blue.
        for (let i = 0, j = 0; i < pixels.length; i += 4, j += c) {
            pixels[i] = 255 * d[j];
            pixels[i + 1] = 255 * d[c < 3 ? j : j + 1];
            pixels[i + 2] = 255 * d[c < 3 ? j : j + 2];
            pixels[i + 3] = c % 2 ? 255 : 255 * d[j + c - 1];
        }
        if (scratch?.width !== w || scratch.height !== h) scratch = new OffscreenCanvas(w, h);
        scratch.getContext('2d').putImageData(new ImageData(pixels, w, h), 0, 0);
        Object.assign(ctx, { imageSmoothingEnabled: !pixelated, imageSmoothingQuality: 'high' });
        ctx.drawImage(scratch, 0, 0, ctx.canvas.width, ctx.canvas.height);
    };

    return (ctx, data, options) => {
        const { canvas } = ctx, { width = canvas.width, height = canvas.height, fade = 1, blend, fps, background, pixelated } = options ?? {};
        const bg = typeof background === 'number' ? [background, background, background] : background?.data;
        if (canvas.width !== width || canvas.height !== height) {
            Object.assign(canvas, { width, height });
            drawn.delete(canvas);
        }
        // Covers the canvas with the background at opacity `alpha`. Without a background, it fades the canvas towards transparent.
        const cover = alpha => {
            ctx.globalCompositeOperation = bg ? 'source-over' : 'destination-out';
            const [r, g, b] = bg ?? [0, 0, 0];
            ctx.fillStyle = rgba(r, g, b, alpha);
            ctx.fillRect(0, 0, canvas.width, canvas.height);
        };
        const draw = (frame, fresh) => {
            if (fresh || fade > 0) cover(fresh ? 1 : fade);
            ctx.globalCompositeOperation = blend === 'add' ? 'lighter' : 'source-over';
            ctx.lineCap = 'round';
            picture(ctx, frame.image, pixelated);
            rows(frame.triangles, (d, o) => {
                ctx.beginPath();
                ctx.moveTo(d[o], d[o + 1]);
                ctx.lineTo(d[o + 2], d[o + 3]);
                ctx.lineTo(d[o + 4], d[o + 5]);
                ctx.fillStyle = rgba(d[o + 6], d[o + 7], d[o + 8], d[o + 9]);
                ctx.fill();
            });
            rows(frame.lines, (d, o) => {
                // The canvas ignores a `lineWidth` of 0 and keeps the previous width.
                if (!(d[o + 4] > 0)) return;
                ctx.beginPath();
                ctx.moveTo(d[o], d[o + 1]);
                ctx.lineTo(d[o + 2], d[o + 3]);
                ctx.lineWidth = d[o + 4];
                ctx.strokeStyle = rgba(d[o + 5], d[o + 6], d[o + 7], d[o + 8]);
                ctx.stroke();
            });
            rows(frame.points, (d, o, k) => {
                const [x, y, radius, inner, outer] = [d[o], d[o + 1], d[o + 2], d[o + 6], d[o + 7]], stroke = k > 8 ? d[o + 8] : 0;
                if (!(radius > 0)) return;
                const colour = a => rgba(d[o + 3], d[o + 4], d[o + 5], a);
                ctx.beginPath();
                ctx.arc(x, y, radius, 0, 2 * Math.PI);
                if (stroke > 0) {
                    ctx.lineWidth = stroke;
                    ctx.strokeStyle = colour(inner);
                    ctx.stroke();
                    return;
                }
                if (inner === outer) ctx.fillStyle = colour(inner);
                else {
                    const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
                    gradient.addColorStop(0, colour(inner));
                    gradient.addColorStop(1, colour(outer));
                    ctx.fillStyle = gradient;
                }
                ctx.fill();
            });
        };

        if (!fps) {
            draw(data, !drawn.has(canvas));
            drawn.add(canvas);
            return;
        }
        // Frame `i` of the endless sequence is due `i/fps` seconds after the start, and draws `frames[i % n]`. Each tick draws every
        // due frame in order, and never skips a frame that adds to the canvas. After a long gap, such as a hidden tab, drawing
        // restarts at the first frame of the current loop. That frame covers the whole canvas.
        const frames = data.data, n = frames.length, start = performance.now();
        if (!n) return;
        let next = 0;
        const tick = t => {
            if (canvas.isConnected === false) return;
            const last = Math.floor((t - start) * fps / 1000);
            next = Math.max(next, last - last % n);
            for (; next <= last; next++) draw(frames[next % n], next % n === 0);
            requestAnimationFrame(tick);
        };
        requestAnimationFrame(tick);
    };
})()
