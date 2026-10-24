

# `•date` — Dates

`•date text` reads a date and time as a moment: the seconds since the
Unix epoch, midnight UTC on 1 January 1970. Without a pattern it reads
ISO 8601: RFC 3339 text such as `"2024-05-06T10:20:30Z"`, a date and
time with no offset, or a date alone. An array of texts gives an array
of moments. `•time 0` is the current moment.

`•date⁻¹ t` writes the moment `t` as a record of fields named after
chrono’s: `year`, `month`, `day`, `hour`, `minute`, `second`,
`nanosecond`, `weekday` (1 for Monday), `ordinal` (the day of the year)
and `iso_week`. For an array of moments, each field is an array. `•date`
reads such a record back from its first seven fields. A missing month or
day is 1, and a missing time field is 0. `•date` ignores `weekday`,
`ordinal` and `iso_week`.

A pattern on the left reads or writes text in another layout, with
chrono’s strftime specifiers, such as `%Y-%m-%d`. The options are
`pattern`, `zone` and, for `•date⁻¹` only, `locale`. `zone` is `"local"`
or a whole number of seconds east of UTC. It applies to text and fields
without an offset, and is UTC by default. `locale` is a POSIX locale,
such as `"fr_FR"`, for month and weekday names.

The browser build has no `locale` option.

``` bpl
•date "2024-05-06T10:20:30Z"                     ⍝ 1714990830
"%d %B %Y" •date⁻¹ 1714990830                     ⍝ "06 May 2024"
["pattern":"%A" "locale":"fr_FR"] •date⁻¹ 0       ⍝ "jeudi"
(•date⁻¹ 0).weekday                               ⍝ 4ₓ
86400+ @ "%Y-%m-%d"↣•date "2024-05-06"          ⍝ "2024-05-07"
```

Errors: `DOMAIN` for text that the pattern doesn’t match, an invalid
date, an unknown field or locale, or an invalid pattern.
