# @oval-vote/oval

Parse, validate, compile, and format [Oval](https://oval.vote) voting guides from JavaScript.
This is the reference implementation's Rust code compiled to WebAssembly, for Node.

```js
const oval = require("@oval-vote/oval");

const source = fs.readFileSync("guide.oval", "utf8");

oval.check(source);
// [{ code: "unused-declaration", severity: "warning", range: { start: 120, end: 124 }, message: "..." }]

oval.compile(source);
// The canonical JSON model described by the specification's guide.schema.json.
// Throws the diagnostics when the document has errors.

oval.format(source);
// The formatted source. Throws the diagnostics when the document has syntax errors.
```

Ranges are byte offsets into the UTF-8 source.

The language is specified in [oval-vote/oval-spec](https://github.com/oval-vote/oval-spec) and
implemented in [oval-vote/oval](https://github.com/oval-vote/oval), which also provides the `oval`
command-line tool.
