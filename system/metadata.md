

# `•metadata` — File metadata

`•metadata paths` gives a table with a row for each path, as a record of
columns. The columns follow Rust’s `std::fs::Metadata`:

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Column</th>
<th>Gives</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>path</code>, <code>name</code></td>
<td>The path as given, and its last part</td>
</tr>
<tr>
<td><code>kind</code></td>
<td><code>"file"</code>, <code>"dir"</code>, <code>"symlink"</code> or
<code>"other"</code>, or <code>"none"</code> for a path that doesn’t
exist</td>
</tr>
<tr>
<td><code>target</code></td>
<td>For a symbolic link, the path it points to, as Rust’s
<code>std::fs::read_link</code> gives it. Empty for any other entry</td>
</tr>
<tr>
<td><code>len</code></td>
<td>The size in bytes</td>
</tr>
<tr>
<td><code>modified</code>, <code>accessed</code>,
<code>created</code></td>
<td>Moments, in seconds since the Unix epoch, as <a
href="date.qmd"><code>•date</code></a> reads them. NaN where the system
records none</td>
</tr>
<tr>
<td><code>readonly</code></td>
<td>Whether the permissions forbid writing</td>
</tr>
<tr>
<td><code>hidden</code></td>
<td>Whether the name starts with <code>.</code></td>
</tr>
<tr>
<td><code>readable</code>, <code>writable</code>,
<code>executable</code></td>
<td>Whether this process may read, write or execute the entry</td>
</tr>
<tr>
<td><code>uid</code>, <code>mode</code>, <code>owner</code></td>
<td>On Unix only: the owner’s numeric ID, the mode bits and the owner’s
name</td>
</tr>
</tbody>
</table>

A symbolic link’s kind is `"symlink"`. Its other columns describe the
entry it points to.

Errors: `DOMAIN` when a path isn’t text.
