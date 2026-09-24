use crate::{data::Options, display, execution::Context, keyed, primitive::real, Error, ErrorKind, Value};
use plotters::{
    coord::{
        ranged1d::{DefaultFormatting, KeyPointHint, Ranged},
        Shift,
    },
    prelude::*,
};

type Area<'a> = DrawingArea<SVGBackend<'a>, Shift>;

const PLOT: &[&str] = &["data", "mark", "title", "width", "height", "x", "y", "legend", "grid", "flip", "color", "size", "labels", "series", "_mime_"];
const FIGURE: &[&str] = &["data", "title", "width", "height", "widths", "heights", "share", "_mime_"];
const STYLE: &[&str] = &["mark", "color", "size", "labels"];
const PALETTE: [(&str, RGBColor); 10] = [
    ("blue", RGBColor(31, 119, 180)),
    ("orange", RGBColor(255, 127, 14)),
    ("green", RGBColor(44, 160, 44)),
    ("red", RGBColor(214, 39, 40)),
    ("purple", RGBColor(148, 103, 189)),
    ("brown", RGBColor(140, 86, 75)),
    ("pink", RGBColor(227, 119, 194)),
    ("gray", RGBColor(127, 127, 127)),
    ("olive", RGBColor(188, 189, 34)),
    ("cyan", RGBColor(23, 190, 207)),
];

fn invalid(span: &Context<'_>, message: impl Into<String>) -> Error { span.error(ErrorKind::Domain, message) }

fn drawn<T, E: std::fmt::Display>(result: Result<T, E>, span: &Context<'_>) -> Result<T, Error> {
    result.map_err(|e| invalid(span, format!("plot drawing failed: {e}")))
}

/// `X •plot Y` returns a spec holding the data `Y`, the settings in `X` and a `_mime_` renderer. Plain text `X` is shorthand for `mark`.
pub(crate) fn plot(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let settings = match left {
        Some(mark) if keyed::name(mark).is_some() => Some(keyed::vector(vec!["mark".into()], vec![mark.clone()])),
        left => left.cloned().map(Ok),
    };
    let spec = keyed::vector(vec!["data".into()], vec![right.clone()]).and_then(|spec| settings.map_or(Ok(spec.clone()), |s| keyed::merge(&spec, &s?)));
    spec.and_then(|spec| display::with_renderer(&spec, "plot-renderer", render)).map_err(|k| span.error(k, "•plot settings must be a keyed vector or a mark"))
}

fn render(_: Option<&Value>, spec: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let data = keyed::field(spec, "data").ok_or_else(|| invalid(span, "•plot needs data"))?;
    let figure = is_figure(&data);
    let opts = Options::new("•plot", Some(spec), None, if figure { FIGURE } else { PLOT }, span)?;
    let size = (opts.number("width", 600., span)? as u32, opts.number("height", 400., span)? as u32);
    let mut svg = String::new();
    {
        let root = SVGBackend::with_string(&mut svg, size).into_drawing_area();
        drawn(root.fill(&WHITE), span)?;
        if figure { draw_figure(&opts, &data, &root, span)? }
        else { Chart::new(&opts, span)?.draw(&root, span)? }
        drawn(root.present(), span)?;
    }
    display::svg(keyed::text(&svg)).map_err(|k| span.error(k, "invalid plot MIME bundle"))
}

fn record(opts: &Options, key: &str, allowed: &[&str], span: &Context<'_>) -> Result<Options, Error> {
    Options::new("•plot", opts.values.get(key), None, allowed, span)
}

fn titled<'a>(area: &Area<'a>, title: &str, span: &Context<'_>) -> Result<Area<'a>, Error> {
    if title.is_empty() { Ok(area.clone()) } else { drawn(area.titled(title, ("sans-serif", 18)), span) }
}

fn numbers(value: &Value, span: &Context<'_>) -> Result<Vec<f64>, Error> {
    value.elements().map(|e| real(&e, span).and_then(|v| if v.is_finite() { Ok(v) } else { Err(invalid(span, "•plot values must be finite")) })).collect()
}

fn names(value: &Value, axis: usize) -> Vec<String> { value.keys(axis).map_or(vec![], |k| k.names().iter().map(|n| n.to_string()).collect()) }

fn axis_name(value: &Value, axis: usize) -> String { value.axis_name(axis).map_or(String::new(), |n| n.to_string()) }

fn positions(n: usize) -> Vec<f64> { (1..=n).map(|i| i as f64).collect() }

/// Tick label text: up to ten decimals, without trailing zeros.
fn label(v: f64) -> String {
    let text = format!("{v:.10}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" { "0".into() } else { text.into() }
}

/// The data's structure supplies x positions, category labels, series and titles.
#[derive(Default)]
struct Data {
    x: Vec<f64>,
    categories: Vec<String>,
    series: Vec<(Option<String>, Vec<f64>)>,
    x_title: String,
    legend_title: String,
}

fn parse(data: &Value, span: &Context<'_>) -> Result<Data, Error> {
    let mut d = Data::default();
    match *data.shape() {
        [_, cols] => {
            let rows = names(data, 0);
            d.series = numbers(data, span)?.chunks(cols.max(1)).enumerate().map(|(i, y)| (rows.get(i).cloned(), y.to_vec())).collect();
            (d.x, d.categories, d.x_title, d.legend_title) = (positions(cols), names(data, 1), axis_name(data, 1), axis_name(data, 0));
        }
        [_] if data.keys(0).is_some() && data.elements().all(|e| !e.is_atom()) => {
            let columns = keyed::pairs(data).map_err(|k| span.error(k, "invalid •plot table"))?;
            d.x = positions(columns.first().map_or(0, |(_, c)| c.len()));
            for (name, column) in columns {
                if &*name != "x" {
                    d.series.push((Some(name.to_string()), numbers(&column, span)?));
                } else if let Some(labels) = column.elements().map(|e| keyed::name(&e).map(|s| s.to_string())).collect::<Option<Vec<_>>>() {
                    (d.x, d.categories) = (positions(labels.len()), labels);
                } else { d.x = numbers(&column, span)?; }
            }
        }
        [n] => (d.x, d.categories, d.series) = (positions(n), names(data, 0), vec![(None, numbers(data, span)?)]),
        _ => return Err(span.error(ErrorKind::Rank, "•plot needs a vector, matrix or table")),
    }
    if d.series.iter().any(|(_, y)| y.len() != d.x.len()) { return Err(span.error(ErrorKind::Length, "•plot series need one value per x")); }
    Ok(d)
}

/// `inner` hides the labels of an axis that a neighbouring plot in a figure labels.
struct Axis {
    lo: f64,
    hi: f64,
    log: bool,
    title: String,
    ticks: Option<Value>,
    categories: Vec<String>,
    inner: bool,
}

impl Axis {
    fn new(opts: &Options, title: String, categories: Vec<String>, span: &Context<'_>) -> Result<Self, Error> {
        let log = match opts.text("scale", Some("linear"), span)?.as_str() {
            "linear" => false,
            "log" => true,
            _ => return Err(invalid(span, "•plot scale must be 'linear' or 'log'")),
        };
        Ok(Self { lo: 0., hi: 1., log, title: opts.text("title", Some(&title), span)?, ticks: opts.values.get("ticks").cloned(), categories, inner: false })
    }

    fn map(&self, v: f64, span: &Context<'_>) -> Result<f64, Error> {
        if !self.log { Ok(v) } else if v > 0. { Ok(v.log10()) } else { Err(invalid(span, "log scales need positive values")) }
    }

    /// Fit the range to `values`, padded by `fraction` of the range and at least `pad`.
    fn fit<'a>(&mut self, values: impl Iterator<Item = &'a f64>, zero: bool, fraction: f64, pad: f64) {
        let (mut lo, mut hi) = values.fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        if zero && !self.log { (lo, hi) = (lo.min(0.), hi.max(0.)); }
        if lo > hi { (lo, hi) = (0., 1.); }
        let pad = if !self.categories.is_empty() { 0.5 } else if hi > lo { pad.max(fraction * (hi - lo)) } else { pad.max(1.) };
        (self.lo, self.hi) = (lo - pad, hi + pad);
    }

    /// Tick positions and labels. `count` is the default number of ticks.
    fn ticks(&self, count: f64, span: &Context<'_>) -> Result<Vec<(f64, String)>, Error> {
        let ticks = match &self.ticks {
            Some(t) if t.keys(0).is_some() => keyed::pairs(t)
                .map_err(|k| span.error(k, "invalid •plot ticks"))?
                .into_iter()
                .map(|(name, v)| Ok((self.map(real(&v, span)?, span)?, name.to_string())))
                .collect::<Result<Vec<_>, Error>>()?,
            Some(t) if !t.is_atom() => numbers(t, span)?.into_iter().map(|v| Ok((self.map(v, span)?, label(v)))).collect::<Result<_, Error>>()?,
            None if !self.categories.is_empty() => self.categories.iter().enumerate().map(|(i, c)| ((i + 1) as f64, c.clone())).collect(),
            _ if self.log => {
                // Short log ranges also tick at 2 and 5 times each power of 10.
                let steps: &[f64] = if self.hi - self.lo < 3. { &[1., 2., 5.] } else { &[1.] };
                (self.lo.floor() as i32..=self.hi.ceil() as i32)
                    .flat_map(|e| steps.iter().map(move |m| m * 10f64.powi(e)))
                    .map(|v| (v.log10(), label(v)))
                    .collect()
            }
            given => {
                let count = given.as_ref().map_or(Ok(count), |c| real(c, span))?;
                let raw = (self.hi - self.lo) / count.max(1.);
                let magnitude = 10f64.powf(raw.log10().floor());
                let step = [1., 2., 5., 10.].into_iter().map(|m| m * magnitude).find(|&s| s >= raw).unwrap_or(raw);
                ((self.lo / step).ceil() as i64..=(self.hi / step).floor() as i64).map(|i| (i as f64 * step, label(i as f64 * step))).collect()
            }
        };
        Ok(ticks.into_iter().filter(|(t, _)| (self.lo..=self.hi).contains(t)).collect())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mark { Line, Point, Bar }

fn mark(value: &Value, span: &Context<'_>) -> Result<Mark, Error> {
    match keyed::name(value).as_deref() {
        Some("line") => Ok(Mark::Line),
        Some("point") => Ok(Mark::Point),
        Some("bar") => Ok(Mark::Bar),
        _ => Err(invalid(span, "•plot mark must be 'line', 'point' or 'bar'")),
    }
}

fn color(value: &Value, span: &Context<'_>) -> Result<RGBColor, Error> {
    let name = keyed::name(value).ok_or_else(|| invalid(span, "•plot colours must be text"))?;
    if let Some(n) = name.strip_prefix('#').filter(|h| h.len() == 6).and_then(|h| u32::from_str_radix(h, 16).ok()) {
        return Ok(RGBColor((n >> 16) as u8, (n >> 8) as u8, n as u8));
    }
    PALETTE.iter().find(|(n, _)| **n == *name).map(|(_, c)| *c).ok_or_else(|| invalid(span, format!("unknown colour: {name}")))
}

/// Point radii: a scalar is pixels; a vector is data scaled so that area follows value.
fn radii(size: Option<&Value>, n: usize, span: &Context<'_>) -> Result<Vec<f64>, Error> {
    let Some(size) = size else { return Ok(vec![4.; n]) };
    if size.is_atom() { return Ok(vec![real(size, span)?; n]); }
    let values = numbers(size, span)?;
    if values.len() != n { return Err(span.error(ErrorKind::Length, "•plot size needs one value per point")); }
    let max = values.iter().fold(0f64, |m, &v| m.max(v));
    Ok(values.iter().map(|&v| if max > 0. { 12. * (v.max(0.) / max).sqrt() } else { 0. }).collect())
}

/// Data labels: `1` labels each point with its value; a vector gives one label per point.
fn texts(labels: Option<&Value>, values: &[f64], span: &Context<'_>) -> Result<Vec<String>, Error> {
    let Some(labels) = labels else { return Ok(vec![]) };
    if labels.is_atom() { return Ok(if real(labels, span)? != 0. { values.iter().map(|&v| label(v)).collect() } else { vec![] }); }
    if labels.len() != values.len() { return Err(span.error(ErrorKind::Length, "•plot labels need one value per point")); }
    labels.elements().map(|e| keyed::name(&e).map_or_else(|| real(&e, span).map(label), |s| Ok(s.to_string()))).collect()
}

struct Series {
    name: Option<String>,
    raw: Vec<f64>,
    y: Vec<f64>,
    mark: Mark,
    color: RGBColor,
    size: Option<Value>,
    labels: Option<Value>,
}

/// Where series names appear: a boxed legend in a corner of the plot area, or beside each series' last point.
#[derive(Clone)]
enum Legend { Box(SeriesLabelPosition), End }

struct Chart {
    title: String,
    x: Axis,
    y: Axis,
    xs: Vec<f64>,
    series: Vec<Series>,
    flip: bool,
    grid: bool,
    legend: Option<Legend>,
    border: bool,
    legend_title: String,
}

impl Chart {
    fn new(opts: &Options, span: &Context<'_>) -> Result<Self, Error> {
        let data = parse(opts.values.get("data").ok_or_else(|| invalid(span, "•plot needs data"))?, span)?;
        let mut x = Axis::new(&record(opts, "x", &["title", "scale", "ticks"], span)?, data.x_title, data.categories, span)?;
        let mut y = Axis::new(&record(opts, "y", &["title", "scale", "ticks"], span)?, String::new(), vec![], span)?;
        let styles = opts.values.get("series").map_or(Ok(vec![]), keyed::pairs).map_err(|k| span.error(k, "•plot series must be keyed by series name"))?;
        if let Some((name, _)) = styles.iter().find(|(name, _)| !data.series.iter().any(|(n, _)| n.as_deref() == Some(&**name))) {
            return Err(invalid(span, format!("•plot has no series named {name}")));
        }
        let mut series = Vec::new();
        for (i, (name, raw)) in data.series.into_iter().enumerate() {
            let own = Options::new("•plot", styles.iter().find(|(k, _)| name.as_deref() == Some(&**k)).map(|(_, v)| v), None, STYLE, span)?;
            let get = |key: &str| own.values.get(key).or_else(|| opts.values.get(key));
            let y_values = raw.iter().map(|&v| y.map(v, span)).collect::<Result<_, _>>()?;
            series.push(Series {
                mark: get("mark").map_or(Ok(Mark::Line), |v| mark(v, span))?,
                color: get("color").map_or(Ok(PALETTE[i % PALETTE.len()].1), |v| color(v, span))?,
                size: get("size").cloned(),
                labels: get("labels").cloned(),
                name,
                raw,
                y: y_values,
            });
        }
        let xs = data.x.iter().map(|&v| x.map(v, span)).collect::<Result<Vec<_>, _>>()?;
        let bars = series.iter().any(|s| s.mark == Mark::Bar);
        x.fit(xs.iter(), false, 0.05, if bars { spacing(&xs) / 2. } else { 0. });
        let labelled = series.iter().any(|s| s.labels.is_some());
        y.fit(series.iter().flat_map(|s| &s.y), bars, if labelled { 0.12 } else { 0.05 }, 0.);
        let legend = Options::new("•plot", opts.values.get("legend"), Some("position"), &["position", "border"], span)?;
        let corner = |position| Some(Legend::Box(position));
        let position = match legend.text("position", Some("none"), span)?.as_str() {
            "none" => None,
            "end" => Some(Legend::End),
            "top-left" => corner(SeriesLabelPosition::UpperLeft),
            "top" => corner(SeriesLabelPosition::UpperMiddle),
            "top-right" => corner(SeriesLabelPosition::UpperRight),
            "left" => corner(SeriesLabelPosition::MiddleLeft),
            "center" => corner(SeriesLabelPosition::MiddleMiddle),
            "right" => corner(SeriesLabelPosition::MiddleRight),
            "bottom-left" => corner(SeriesLabelPosition::LowerLeft),
            "bottom" => corner(SeriesLabelPosition::LowerMiddle),
            "bottom-right" => corner(SeriesLabelPosition::LowerRight),
            _ => return Err(invalid(span, "unknown •plot legend position")),
        };
        Ok(Self {
            title: opts.text("title", Some(""), span)?,
            flip: opts.boolean("flip", false, span)?,
            grid: opts.boolean("grid", true, span)?,
            legend: position.filter(|_| series.iter().any(|s| s.name.is_some())),
            border: legend.boolean("border", true, span)?,
            legend_title: data.legend_title,
            x,
            y,
            xs,
            series,
        })
    }

    /// The horizontal and vertical axes.
    fn axes(&mut self) -> [&mut Axis; 2] { if self.flip { [&mut self.y, &mut self.x] } else { [&mut self.x, &mut self.y] } }

    fn draw(&self, area: &Area<'_>, span: &Context<'_>) -> Result<(), Error> {
        let area = titled(area, &self.title, span)?;
        let (h, v) = if self.flip { (&self.y, &self.x) } else { (&self.x, &self.y) };
        let (width, height) = area.dim_in_pixel();
        let (h_ticks, v_ticks) = (h.ticks((width as f64 / 100.).max(2.), span)?, v.ticks((height as f64 / 60.).max(2.), span)?);
        let font = ("sans-serif", 12).into_font();
        let size = |text: &str| font.box_size(text).map_or((0, 0), |(w, h)| (w as i32, h as i32));
        let boxed = match &self.legend { Some(Legend::Box(position)) => Some(position.clone()), _ => None };
        let end = matches!(self.legend, Some(Legend::End));
        let right = if end { self.series.iter().filter_map(|s| s.name.as_deref()).map(|n| size(n).0 + 18).max().unwrap_or(10) } else { 10 };
        let mut chart = drawn(
            ChartBuilder::on(&area)
                .margin(10)
                .margin_right(right)
                .x_label_area_size(if h.inner { 0 } else { 40 })
                .y_label_area_size(if v.inner { 0 } else { 60 })
                .build_cartesian_2d(
                    Coord { lo: h.lo, hi: h.hi, ticks: h_ticks.iter().map(|t| t.0).collect() },
                    Coord { lo: v.lo, hi: v.hi, ticks: v_ticks.iter().map(|t| t.0).collect() },
                ),
            span,
        )?;
        let tick = |ticks: &[(f64, String)], at: f64| ticks.iter().find(|(t, _)| *t == at).map_or(String::new(), |(_, s)| s.clone());
        let (h_label, v_label) = (|at: &f64| tick(&h_ticks, *at), |at: &f64| tick(&v_ticks, *at));
        let mut mesh = chart.configure_mesh();
        mesh.x_label_formatter(&h_label).y_label_formatter(&v_label).x_desc(&h.title).y_desc(&v.title);
        mesh.label_style(("sans-serif", 12)).axis_desc_style(("sans-serif", 14));
        if !self.grid { mesh.disable_mesh(); }
        drawn(mesh.draw(), span)?;
        if boxed.is_some() && !self.legend_title.is_empty() {
            drawn(chart.draw_series(std::iter::empty::<PathElement<(f64, f64)>>()), span)?.label(&self.legend_title);
        }
        let at = |x: f64, y: f64| if self.flip { (y, x) } else { (x, y) };
        let bars = self.series.iter().filter(|s| s.mark == Mark::Bar).count();
        let slot = 0.8 * spacing(&self.xs) / bars.max(1) as f64;
        let base = if self.y.log { self.y.lo } else { 0. };
        let mut bar = 0;
        let (mut labels, mut ends) = (Vec::new(), Vec::new());
        for s in &self.series {
            let points: Vec<_> = self.xs.iter().zip(&s.y).map(|(&x, &y)| at(x, y)).collect();
            let (mark, color) = (s.mark, s.color);
            let anno = match mark {
                Mark::Line => {
                    let width = s.size.as_ref().map_or(Ok(2.), |v| real(v, span))?;
                    drawn(chart.draw_series(std::iter::once(PathElement::new(points.clone(), color.stroke_width(width as u32)))), span)?
                }
                Mark::Point => {
                    let radii = radii(s.size.as_ref(), points.len(), span)?;
                    drawn(chart.draw_series(points.iter().zip(radii).map(|(&p, r)| Circle::new(p, r, color.filled()))), span)?
                }
                Mark::Bar => {
                    let offset = (bar as f64 - (bars - 1) as f64 / 2.) * slot;
                    bar += 1;
                    let rect = |(&x, &y): (&f64, &f64)| Rectangle::new([at(x + offset - slot / 2., base), at(x + offset + slot / 2., y)], color.filled());
                    drawn(chart.draw_series(self.xs.iter().zip(&s.y).map(rect)), span)?
                }
            };
            if let (Some(name), true) = (&s.name, boxed.is_some()) {
                anno.label(name).legend(move |(x, y)| match mark {
                    Mark::Line => PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(2)).into_dyn(),
                    Mark::Point => Circle::new((x + 10, y), 4, color.filled()).into_dyn(),
                    Mark::Bar => Rectangle::new([(x + 4, y - 5), (x + 16, y + 5)], color.filled()).into_dyn(),
                });
            }
            for (text, p) in texts(s.labels.as_ref(), &s.raw, span)?.into_iter().zip(&points).filter(|(t, _)| !t.is_empty()) {
                labels.push((text, chart.backend_coord(p), BLACK));
            }
            if let (true, Some(name), Some(p)) = (end, &s.name, points.last()) { ends.push((name.clone(), chart.backend_coord(p), color)); }
        }
        if let Some(position) = boxed {
            let border = if self.border { BLACK.mix(0.4) } else { TRANSPARENT };
            drawn(chart.configure_series_labels().position(position).border_style(border).background_style(WHITE.mix(0.8)).draw(), span)?;
        }
        // Data labels sit above their points and move up. End labels sit right of each series' last point and move down.
        let above = labels
            .iter()
            .map(|(t, (x, y), _)| {
                let (w, h) = size(t);
                [x - w / 2, y - 6 - h, x - w / 2 + w, y - 6]
            })
            .collect();
        let beside = ends
            .iter()
            .map(|(t, (x, y), _)| {
                let (w, h) = size(t);
                [x + 8, y - h / 2, x + 8 + w, y - h / 2 + h]
            })
            .collect();
        let origin = area.get_base_pixel();
        for ((text, _, color), [x0, y0, ..]) in labels.iter().zip(place(above, true)).chain(ends.iter().zip(place(beside, false))) {
            drawn(area.draw(&Text::new(text.as_str(), (x0 - origin.0, y0 - origin.1), font.color(color))), span)?;
        }
        Ok(())
    }
}

/// A linear plotters axis with ticks from `Axis::ticks`. Labels come from the mesh formatters.
struct Coord { lo: f64, hi: f64, ticks: Vec<f64> }

impl Ranged for Coord {
    type FormatOption = DefaultFormatting;
    type ValueType = f64;
    fn map(&self, v: &f64, (start, end): (i32, i32)) -> i32 { (start as f64 + (v - self.lo) / (self.hi - self.lo) * (end - start) as f64).round() as i32 }
    fn key_points<Hint: KeyPointHint>(&self, hint: Hint) -> Vec<f64> { if hint.weight().allow_light_points() { vec![] } else { self.ticks.clone() } }
    fn range(&self) -> std::ops::Range<f64> { self.lo..self.hi }
}

/// The smallest gap between distinct x values, or 1.
fn spacing(xs: &[f64]) -> f64 {
    let mut sorted = xs.to_vec();
    sorted.sort_by(f64::total_cmp);
    let gap = sorted.windows(2).map(|w| w[1] - w[0]).filter(|&d| d > 0.).fold(f64::INFINITY, f64::min);
    if gap.is_finite() { gap } else { 1. }
}

/// Label boxes `[x0, y0, x1, y1]` in pixels, each moved vertically just clear of the boxes placed before it.
/// Boxes are placed nearest-first in the direction they move: upward if `up`, otherwise downward.
fn place(boxes: Vec<[i32; 4]>, up: bool) -> Vec<[i32; 4]> {
    let mut order: Vec<usize> = (0..boxes.len()).collect();
    order.sort_by_key(|&i| if up { -boxes[i][3] } else { boxes[i][1] });
    let mut placed = boxes.clone();
    for (n, &i) in order.iter().enumerate() {
        let mut b = boxes[i];
        while let Some(o) = order[..n].iter().map(|&j| placed[j]).find(|o| b[0] < o[2] && o[0] < b[2] && b[1] < o[3] && o[1] < b[3]) {
            let dy = if up { o[1] - 1 - b[3] } else { o[3] + 1 - b[1] };
            (b[1], b[3]) = (b[1] + dy, b[3] + dy);
        }
        placed[i] = b;
    }
    placed
}
/// A figure's data holds plot specs, with `⍬` for empty cells.
fn is_figure(data: &Value) -> bool {
    let record = |e: &Value| e.shape().len() == 1 && e.keys(0).is_some() && !e.is_empty();
    !data.is_atom() && data.elements().any(|e| record(&e)) && data.elements().all(|e| e.is_empty() || record(&e))
}

fn draw_figure(opts: &Options, data: &Value, root: &Area<'_>, span: &Context<'_>) -> Result<(), Error> {
    let area = titled(root, &opts.text("title", Some(""), span)?, span)?;
    let [rows, cols] = match *data.shape() {
        [n] => [1, n],
        [r, c] => [r, c],
        _ => return Err(span.error(ErrorKind::Rank, "a •plot figure needs a vector or matrix of plots")),
    };
    // A plot repeated in adjacent cells spans them: [top, left, bottom, right) cell edges.
    let mut cells: Vec<(Value, [usize; 4])> = Vec::new();
    for (i, cell) in data.elements().enumerate().filter(|(_, c)| !c.is_empty()) {
        let (r, c) = (i / cols, i % cols);
        match cells.iter_mut().find(|(p, _)| *p == cell) {
            Some((_, b)) => *b = [b[0].min(r), b[1].min(c), b[2].max(r + 1), b[3].max(c + 1)],
            None => cells.push((cell, [r, c, r + 1, c + 1])),
        }
    }
    let mut charts = cells.iter().map(|(p, _)| Chart::new(&Options::new("•plot", Some(p), None, PLOT, span)?, span)).collect::<Result<Vec<_>, _>>()?;
    if opts.boolean("share", false, span)? {
        // Plots in the same columns share their horizontal range, and plots in the same rows their vertical range.
        // Only the bottom and left plots of a group label the shared axis.
        let ranges: Vec<[(f64, f64); 2]> = charts.iter_mut().map(|c| c.axes().map(|a| (a.lo, a.hi))).collect();
        for (i, chart) in charts.iter_mut().enumerate() {
            let b = cells[i].1;
            for (side, axis) in chart.axes().into_iter().enumerate() {
                let group = |c: &[usize; 4]| if side == 0 { (c[1], c[3]) } else { (c[0], c[2]) };
                for (j, (_, c)) in cells.iter().enumerate().filter(|(_, (_, c))| group(c) == group(&b)) {
                    (axis.lo, axis.hi) = (axis.lo.min(ranges[j][side].0), axis.hi.max(ranges[j][side].1));
                    axis.inner |= if side == 0 { c[0] >= b[2] } else { c[3] <= b[1] };
                }
            }
        }
    }
    let (width, height) = area.dim_in_pixel();
    let (xs, ys) = (edges(opts, "widths", cols, width, span)?, edges(opts, "heights", rows, height, span)?);
    for (chart, (_, [top, left, bottom, right])) in charts.iter().zip(&cells) {
        chart.draw(&area.clone().shrink((xs[*left], ys[*top]), (xs[*right] - xs[*left], ys[*bottom] - ys[*top])), span)?;
    }
    Ok(())
}

/// Pixel offsets of the cell edges along one side, from the relative sizes in option `key`.
fn edges(opts: &Options, key: &str, n: usize, size: u32, span: &Context<'_>) -> Result<Vec<u32>, Error> {
    let weights = opts.values.get(key).map_or(Ok(vec![1.; n]), |w| numbers(w, span))?;
    if weights.len() != n { return Err(span.error(ErrorKind::Length, format!("•plot {key} needs one size per cell"))); }
    let total: f64 = weights.iter().sum();
    let mut sum = 0.;
    Ok(std::iter::once(0)
        .chain(weights.iter().map(|w| {
            sum += w;
            (size as f64 * sum / total).round() as u32
        }))
        .collect())
}
