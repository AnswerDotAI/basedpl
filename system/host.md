

# `•host` — Host facts

`•host name` gives the host fact called `name`.

<table>
<colgroup>
<col style="width: 50%" />
<col style="width: 50%" />
</colgroup>
<thead>
<tr>
<th>Name</th>
<th>Gives</th>
</tr>
</thead>
<tbody>
<tr>
<td><code>"args"</code></td>
<td>The arguments after the program on the command line, as a vector of
strings. <code>bpl prog.bpl a b</code> gives <code>"a" "b"</code></td>
</tr>
<tr>
<td><code>"version"</code></td>
<td>BPL’s version</td>
</tr>
<tr>
<td><code>"env"</code></td>
<td>The environment variables, as a record of strings</td>
</tr>
<tr>
<td><code>"width"</code></td>
<td>The width of the terminal that output goes to, or <code>⍬</code>
without one</td>
</tr>
<tr>
<td><code>"height"</code></td>
<td>The height in lines of the terminal that output goes to, or
<code>⍬</code> without one</td>
</tr>
<tr>
<td><code>"cwd"</code></td>
<td>The working directory</td>
</tr>
<tr>
<td><code>"temp"</code></td>
<td>The directory for temporary files</td>
</tr>
</tbody>
</table>

Errors: `DOMAIN` for any other name.
