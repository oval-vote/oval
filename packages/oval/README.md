# @oval-vote/oval

Check, compile, and format [Oval](https://oval.vote) voting guides from JavaScript, and answer the
questions an editor asks about them. This is the reference implementation's Rust code compiled to
WebAssembly, for Node.

```js
const fs = require("node:fs");
const oval = require("@oval-vote/oval");

const source = fs.readFileSync("guide.oval", "utf8");

oval.check(source);
// Every problem in the document, ordered by where each one begins.
// [{ code: "unused-declaration", severity: "warning", range: { start: 120, end: 124 }, message: "..." }]

oval.compile(source);
// The compiled guide, typed as Guide from @oval-vote/oval-schema.

oval.format(source);
// The source in the canonical style.

oval.languageMajor(); // 1, the major version of the language this package reads
oval.version(); // this package's version
```

## Errors

`compile` throws an `OvalError` when the document has errors, and `format` throws one when it has
lexical or syntax errors. Warnings never stop either one. An `OvalError` is an `Error` whose `name`
is `"OvalError"`, with the errors that stopped the call in `diagnostics`.

```js
try {
  oval.compile(source);
} catch (error) {
  if (error.name !== "OvalError") throw error;
  for (const { range, message } of error.diagnostics) console.log(range.start, message);
}
```

Every function, and the `Analysis` constructor, throws a plain `Error` when the source can't be an
Oval document at all, because it contains U+0000 or is too large.

## Editor queries

An `Analysis` checks a document once and then answers questions about it, so an editor can keep one
for each version of a document. A document with errors still has answers.

```js
const analysis = new oval.Analysis(source);

analysis.diagnostics(); // the same diagnostics as check(source)
analysis.outline(); // the ballot tree: [{ kind: "ballot", id, title, range, selection, children }, ...]
analysis.definition(index); // { entity: "candidate", id: "delgado", name: "Rosa Delgado", range, idRange } or null
analysis.occurrences(index); // every place that name is written: [{ range, idRange, isDeclaration }, ...]
analysis.hover(index); // { range, contents } in Markdown, or null
analysis.completions(index); // [{ label: "@delgado", kind: "candidate", detail: "Rosa Delgado", replace }, ...]
analysis.fixes(); // [{ diagnostic: 0, title: "Use `https`", edit: { range, text, placeholder } }, ...]

analysis.free();
```

The garbage collector frees an analysis in time, and `free()`, or a `using` declaration, frees it
at once.

A fix's `diagnostic` is the position of the diagnostic it repairs in `diagnostics()`. Its edit
replaces `range` with `text`, and `placeholder`, when it isn't `null`, is the part of `text` the
author should fill in, such as the value of a guide field the fix adds. An editor can offer it as a
snippet tab stop.

## Positions

Positions are JavaScript string indices, in and out, so `source.slice(range.start, range.end)` is
the text a range spans. An index between the halves of a surrogate pair means the character they
make up. A `placeholder` is in indices into its edit's `text`.

## Types

The package includes TypeScript declarations. A diagnostic's `code` is the identifier the
specification gives the problem, declared as a union of every identifier, so a `switch` over it is
checked.

The language is specified in [oval-vote/oval-spec](https://github.com/oval-vote/oval-spec) and
implemented in [oval-vote/oval](https://github.com/oval-vote/oval), which also provides the `oval`
command-line tool.
