//! Paths and the filesystem, named after Rust's `std::path` and `std::fs`. `•path` reads a path into its parts and `•path⁻¹` joins them.
//! `•metadata` and `•readdir` give tables of entries, and `•copy`, `•rename`, `•remove` and `•mkdir` change the filesystem.
use crate::{
    data::{text, Options},
    execution::Context,
    keyed, Error, ErrorAt, Number, Value,
};
use std::{fs, path::Path};

/// The parts of a path that `•path` gives. `•path⁻¹` joins `parent`, `stem` and `extension`. `name` follows from the last two, and
/// `•path⁻¹` ignores it.
const PARTS: [&str; 4] = ["parent", "stem", "extension", "name"];

/// The text of an `OsStr` part, or `""` without one.
fn part(p: Option<&std::ffi::OsStr>) -> Value { keyed::text(&p.map_or(String::new(), |p| p.to_string_lossy().into_owned())) }

/// The paths in `value`: one path, or an array of them.
fn paths(value: &Value, span: &Context<'_>) -> Result<(Vec<usize>, Vec<std::sync::Arc<str>>), Error> {
    keyed::text_items(value).ok_or_else(|| span.domain_error("a path must be text"))
}

/// `•path Y`: the parts of the path `Y`, or of each path in `Y`, as a record. The `absolute` option first joins a relative path to the
/// working directory, as `std::path::absolute` does. Neither form reads the disk.
pub(crate) fn path(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let absolute = Options::new("•path", left, None, &["absolute"], span)?.boolean("absolute", false, span)?;
    let (shape, paths) = paths(right, span)?;
    let paths = paths.iter().map(|p| match absolute {
        true => std::path::absolute(span.path(p)).map_err(|e| span.domain_error(format!("{p}: {e}"))),
        false => Ok(std::path::PathBuf::from(&**p)),
    });
    let rows = paths.map(|p| p.map(|p| vec![part(p.parent().map(Path::as_os_str)), part(p.file_stem()), part(p.extension()), part(p.file_name())]));
    keyed::table(&PARTS, &shape, rows.collect::<Result<_, _>>()?, vec![keyed::text(""); PARTS.len()]).error_at(span, "invalid path record")
}

/// `•path⁻¹ R`: the path that the record `R` gives, or one for each of its positions, joined from `parent`, `stem` and `extension`.
pub(crate) fn join(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (shape, fields) = keyed::fields(right).error_at(span, "•path⁻¹ needs a record of path parts that agree")?;
    let parts = keyed::slots(fields, &PARTS, 3).map_err(|name| span.domain_error(format!("unknown path part: {name}")))?;
    let part = |i: usize, p: usize| -> Result<String, Error> { parts[i].as_ref().map_or(Ok(String::new()), |items| text(&items[p], span)) };
    let paths = (0..shape.iter().product()).map(|p| {
        let [parent, stem, extension] = [0, 1, 2].map(|i| part(i, p));
        let mut name = stem?;
        let extension = extension?;
        if !extension.is_empty() { name = format!("{name}.{extension}"); }
        Ok(keyed::text(&Path::new(&parent?).join(name).to_string_lossy()))
    });
    keyed::texts(&shape, paths.collect::<Result<_, Error>>()?).error_at(span, "invalid paths")
}

/// The columns of a table of entries. They come from the path, `std::fs::Metadata`, the target that `std::fs::read_link` gives a
/// symbolic link, and access checks. On Unix the table adds the owner's numeric ID, the mode bits and the owner's name.
#[cfg(unix)]
const COLUMNS: &[&str] = &[
    "path",
    "name",
    "kind",
    "target",
    "len",
    "modified",
    "accessed",
    "created",
    "readonly",
    "hidden",
    "readable",
    "writable",
    "executable",
    "uid",
    "mode",
    "owner",
];
#[cfg(not(unix))]
const COLUMNS: &[&str] =
    &["path", "name", "kind", "target", "len", "modified", "accessed", "created", "readonly", "hidden", "readable", "writable", "executable"];

/// The row of a table of entries for `path`, shown as `shown`. A missing entry has kind `"none"`, and NaN for its times.
fn row(path: &Path, shown: &str) -> Vec<Value> {
    // Times stay 64-bit, as `•date` gives them.
    let time_value = |n: f64| Value::Number(Number::float(n, crate::array::FloatWidth::F64));
    let flag = |b: bool| Value::Number(Number::from_bool(b));
    let link = fs::symlink_metadata(path).ok();
    let meta = fs::metadata(path).ok().or_else(|| link.clone());
    let kind = match (&link, &meta) {
        (Some(l), _) if l.file_type().is_symlink() => "symlink",
        (_, Some(m)) if m.is_dir() => "dir",
        (_, Some(m)) if m.is_file() => "file",
        (_, Some(_)) => "other",
        _ => "none",
    };
    let time = |t: Option<std::io::Result<std::time::SystemTime>>| time_value(t.and_then(Result::ok).map_or(f64::NAN, |t| crate::date::seconds(t.into())));
    let name = Path::new(shown).file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned());
    let mut row = vec![
        keyed::text(shown),
        keyed::text(&name),
        keyed::text(kind),
        keyed::text(&fs::read_link(path).map_or(String::new(), |t| t.to_string_lossy().into_owned())),
        Value::Number(Number::from_integer(meta.as_ref().map_or(0, |m| m.len() as i64))),
        time(meta.as_ref().map(fs::Metadata::modified)),
        time(meta.as_ref().map(fs::Metadata::accessed)),
        time(meta.as_ref().map(fs::Metadata::created)),
        flag(meta.as_ref().is_some_and(|m| m.permissions().readonly())),
        flag(name.starts_with('.')),
    ];
    row.extend(access(path).map(flag));
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let uid = meta.as_ref().map(MetadataExt::uid);
        let owner = uid.and_then(|u| nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(u)).ok().flatten()).map_or(String::new(), |u| u.name);
        row.extend([Value::Number(Number::from_integer(uid.map_or(0, i64::from))), Value::Number(Number::from_integer(meta.as_ref().map_or(0, |m| i64::from(m.mode())))), keyed::text(&owner)]);
    }
    row
}

/// Whether this process may read, write and execute `path`.
#[cfg(unix)]
fn access(path: &Path) -> [bool; 3] { use rustix::fs::{access, Access}; [Access::READ_OK, Access::WRITE_OK, Access::EXEC_OK].map(|a| access(path, a).is_ok()) }
/// Whether this process may read, write and execute `path`: readable when it exists, and writable unless it is read-only.
#[cfg(not(unix))]
fn access(path: &Path) -> [bool; 3] { let meta = fs::metadata(path).ok(); [meta.is_some(), meta.is_some_and(|m| !m.permissions().readonly()), false] }

/// The columns of a table of entries that hold text. The others hold numbers.
const TEXT_COLUMNS: [&str; 5] = ["path", "name", "kind", "target", "owner"];

/// A table of `rows`, one for each entry.
fn entries(rows: Vec<Vec<Value>>, span: &Context<'_>) -> Result<Value, Error> {
    let empty = COLUMNS.iter().map(|c| if TEXT_COLUMNS.contains(c) { keyed::text("") } else { Value::Number(Number::from_integer(0)) });
    keyed::table(COLUMNS, &[rows.len()], rows, empty.collect()).error_at(span, "invalid table of entries")
}

/// `•metadata Y`: a table with a row for the path `Y`, or for each path in `Y`.
pub(crate) fn metadata(_: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let (_, paths) = paths(right, span)?;
    entries(paths.iter().map(|p| row(&span.path(p), p)).collect(), span)
}

/// `•readdir Y`: a table with a row for each entry of the directory `Y`, sorted by path. The left argument is a glob that each entry's
/// path within `Y` must match, or options `glob` and `recurse`, which also lists entries of subdirectories.
pub(crate) fn readdir(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let opts = Options::new("•readdir", left, Some("glob"), &["glob", "recurse"], span)?;
    let dir = text(right, span)?;
    let glob = match opts.values.get("glob") {
        // As in a shell, `*` stays within one directory and `**` crosses them.
        Some(g) => Some(
            globset::GlobBuilder::new(&text(g, span)?)
                .literal_separator(true)
                .build()
                .map_err(|e| span.domain_error(format!("invalid glob: {e}")))?
                .compile_matcher(),
        ),
        None => None,
    };
    let depth = if opts.boolean("recurse", false, span)? { usize::MAX } else { 1 };
    let root = span.path(&dir);
    let mut rows = Vec::new();
    for entry in walkdir::WalkDir::new(&root).min_depth(1).max_depth(depth).sort_by_file_name() {
        span.check()?;
        let entry = entry.map_err(|e| span.io_error(&dir, e))?;
        let within = entry.path().strip_prefix(&root).expect("walkdir gives paths within its root");
        if glob.as_ref().is_some_and(|g| !g.is_match(within)) { continue; }
        rows.push(row(entry.path(), &Path::new(&dir).join(within).to_string_lossy()));
    }
    entries(rows, span)
}

/// A path argument's text and the file it names.
fn argument(value: &Value, span: &Context<'_>) -> Result<(String, std::path::PathBuf), Error> {
    let path = text(value, span)?;
    let file = span.path(&path);
    Ok((path, file))
}

/// `to •copy from`: copies the file or directory `from` to `to`, and gives `to`.
pub(crate) fn copy(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    let ((to, dest), (from, source)) = (argument(left.expect("•copy is dyadic"), span)?, argument(right, span)?);
    if source.is_dir() {
        for entry in walkdir::WalkDir::new(&source) {
            span.check()?;
            let entry = entry.map_err(|e| span.io_error(&from, e))?;
            let copied = dest.join(entry.path().strip_prefix(&source).expect("walkdir gives paths within its root"));
            let done = if entry.file_type().is_dir() { fs::create_dir_all(&copied) } else { fs::copy(entry.path(), &copied).map(drop) };
            done.map_err(|e| span.io_error(&copied.to_string_lossy(), e))?;
        }
    }
    else { fs::copy(&source, &dest).map_err(|e| span.io_error(&format!("{from} to {to}"), e))?; }
    Ok((keyed::text(&to), true))
}

/// `to •rename from`: moves `from` to `to`, and gives `to`.
pub(crate) fn rename(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    let ((to, dest), (from, source)) = (argument(left.expect("•rename is dyadic"), span)?, argument(right, span)?);
    fs::rename(&source, &dest).map_err(|e| span.io_error(&format!("{from} to {to}"), e))?;
    Ok((keyed::text(&to), true))
}

/// `•remove Y`: removes the file or empty directory `Y`, and gives `Y`. The `recurse` option removes a directory and everything in it.
pub(crate) fn remove(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    let recurse = Options::new("•remove", left, None, &["recurse"], span)?.boolean("recurse", false, span)?;
    let (path, file) = argument(right, span)?;
    let removed = match (file.symlink_metadata().is_ok_and(|m| m.is_dir()), recurse) {
        (true, true) => fs::remove_dir_all(&file),
        (true, false) => fs::remove_dir(&file),
        (false, _) => fs::remove_file(&file),
    };
    removed.map_err(|e| span.io_error(&path, e))?;
    Ok((keyed::text(&path), true))
}

/// `•mkdir Y`: makes the directory `Y` and any missing parents, and gives `Y`, which isn't displayed. The `unique` option then makes a
/// directory with a new, unique name inside `Y`, starting with the `prefix` option, and gives its path, which is displayed. It stays
/// until removed.
pub(crate) fn mkdir(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<(Value, bool), Error> {
    let unique = Unique::new(&Options::new("•mkdir", left, None, &["unique", "prefix"], span)?, "•mkdir", span)?;
    let (path, file) = argument(right, span)?;
    fs::create_dir_all(&file).map_err(|e| span.io_error(&path, e))?;
    match unique {
        Some(unique) => Ok((unique.make(&path, span, |builder, dir| builder.tempdir_in(dir).map(|made| made.keep()))?, false)),
        None => Ok((keyed::text(&path), true)),
    }
}

/// A uniquely named file or directory, which the `unique` option asks for. Its name starts with the `prefix` option.
pub(crate) struct Unique { prefix: Option<String> }

impl Unique {
    /// `None` without the `unique` option. The `prefix` option without `unique` is a DOMAIN error.
    pub(crate) fn new(opts: &Options, function: &str, span: &Context<'_>) -> Result<Option<Self>, Error> {
        let prefix = opts.values.get("prefix").map(|p| text(p, span)).transpose()?;
        if opts.boolean("unique", false, span)? { return Ok(Some(Self { prefix })); }
        if prefix.is_some() { return Err(span.domain_error(format!("the {function} prefix option needs unique"))); }
        Ok(None)
    }
    /// Makes the entry in the directory `dir` names through `make`, and gives the entry's path: `dir` joined to its new name.
    pub(crate) fn make(
        &self,
        dir: &str,
        span: &Context<'_>,
        make: impl FnOnce(&tempfile::Builder<'_, '_>, &Path) -> std::io::Result<std::path::PathBuf>,
    ) -> Result<Value, Error> {
        let mut builder = tempfile::Builder::new();
        if let Some(prefix) = &self.prefix { builder.prefix(prefix); }
        let made = make(&builder, &span.path(dir)).map_err(|e| span.io_error(dir, e))?;
        let name = made.file_name().expect("a new entry has a name");
        Ok(keyed::text(&Path::new(dir).join(name).to_string_lossy()))
    }
}
