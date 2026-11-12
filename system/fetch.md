

# `•fetch` — Fetch a URL

`•fetch url` requests `url` and returns a record of the response:
`status`, a number; `headers`, a record of text; and `body`, the
response’s text. A missing page gives a result with `status` 404, not an
error. Header names are in lower case, and a repeated header’s values
are joined with `,`. `•fetch` follows redirects. Natively it runs the
system’s `curl`, which must be installed. In the browser it makes the
request from the page, and the server’s CORS rules apply.
`•json (•fetch url).body` reads a JSON response.

`X •fetch url` takes options on the left: `method`, which is `"GET"`, or
`"POST"` with a body; `headers`, a record of text; `body`, text or bytes
to send; and `binary` (`1` gives the body as a vector of byte values).

Errors: `IO` for getting no response, such as for an unknown host or a
refused connection, for a missing `curl`, and for a body that isn’t
UTF-8 without `binary`; `DOMAIN` for invalid options.
