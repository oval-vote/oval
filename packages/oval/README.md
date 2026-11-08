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

oval.symbols(source);
// { declarations: [{ name: "delgado", kind: "candidate", range, nameRange }, ...],
//   references: [{ name: "delgado", range, nameRange, declaration: 0 }, ...] }

oval.outline(source);
// The ballot tree: [{ kind: "ballot", title, name, range, selection, children: [...] }, ...]

oval.completions(source, byteOffset);
// What the grammar accepts there: [{ label: "@delgado", kind: "candidate", detail: "Rosa Delgado", replace }, ...]

oval.hover(source, byteOffset);
// { range, contents: "**candidate** `@delgado`\n\nRosa Delgado\n\n..." } or undefined

oval.fixes(source);
// Repairs for diagnostics that have one: [{ code: "insecure-url", range, title: "Use https", edits }, ...]

oval.catalog();
// Every diagnostic the package can report:
// [{ identifier: "illegal-tab", severity: "error", title: "Illegal Tab",
//    url: "https://spec.oval.vote/draft/#sec-Illegal-Tab" }, ...]

oval.languageMajor(); // 1, the major version of the language this package reads
oval.version(); // the package's own version
```

Ranges are byte offsets into the UTF-8 source. Diagnostic codes are the identifiers of the
specification's entries, and the typings declare them as a string union, so a `switch` over a
code is checked by TypeScript.

The language is specified in [oval-vote/oval-spec](https://github.com/oval-vote/oval-spec) and
implemented in [oval-vote/oval](https://github.com/oval-vote/oval), which also provides the `oval`
command-line tool.
