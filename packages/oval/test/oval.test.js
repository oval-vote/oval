const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");

const oval = require("../pkg/oval_wasm.js");

const fixture = fs.readFileSync(
    path.join(__dirname, "../../../crates/oval-syntax/tests/fixtures/franklin.oval"),
    "utf8",
);

test("a clean document has no diagnostics", () => {
    assert.deepEqual(oval.check(fixture), []);
});

test("compile returns the canonical document", () => {
    const compiled = oval.compile(fixture);
    assert.equal(compiled.oval, 1);
    assert.equal(compiled.counts.contests, 44);
    assert.equal(compiled.counts.measures, 28);
    assert.equal(compiled.ballots[0].kind, "ballot");
    assert.equal(compiled.questions.length, 72);
});

test("format leaves a formatted document unchanged", () => {
    assert.equal(oval.format(fixture), fixture);
});

test("the package names the language major it reads and its own version", () => {
    assert.equal(oval.languageMajor(), 1);
    assert.equal(oval.version(), require("../package.json").version);
});

test("the catalog lists every diagnostic with its entry on the specification site", () => {
    const catalog = oval.catalog();
    assert.equal(catalog.length, 55);
    assert.deepEqual(catalog[0], {
        identifier: "illegal-tab",
        severity: "error",
        title: "Illegal Tab",
        url: "https://spec.oval.vote/draft/#sec-Illegal-Tab",
    });
    assert.ok(catalog.every((entry) => entry.url.startsWith("https://spec.oval.vote/")));
    assert.equal(new Set(catalog.map((entry) => entry.identifier)).size, catalog.length);
    const warnings = catalog
        .filter((entry) => entry.severity === "warning")
        .map((e) => e.identifier);
    assert.ok(warnings.includes("unused-declaration") && warnings.includes("insecure-url"));
    assert.equal(warnings.at(-1), "empty-string");
});

test("symbols list declarations and references with their ranges", () => {
    const source =
        'oval 1\nguide "G"\nelection "E"\ndate 2026-11-03\n' +
        'candidate @delgado "Rosa Delgado"\n' +
        'ballot @city "City"\n  contest @attorney "City Attorney" single\n' +
        "    campaign @delgado\n    recommend\n      support @delgado\n";
    const { declarations, references } = oval.symbols(source);
    assert.deepEqual(
        declarations.map((d) => [d.name, d.kind]),
        [
            ["delgado", "candidate"],
            ["city", "ballot"],
            ["attorney", "contest"],
        ],
    );
    const first = source.indexOf("@delgado");
    assert.deepEqual(declarations[0].range, { start: first, end: first + 8 });
    assert.deepEqual(declarations[0].nameRange, {
        start: first + 1,
        end: first + 8,
    });
    assert.equal(references.length, 2);
    assert.ok(references.every((r) => r.name === "delgado" && r.declaration === 0));
    assert.equal(
        source.slice(references[1].nameRange.start, references[1].nameRange.end),
        "delgado",
    );
});

test("symbols of the fixture all resolve", () => {
    const { declarations, references } = oval.symbols(fixture);
    assert.ok(declarations.length > 100);
    assert.ok(references.every((r) => typeof r.declaration === "number"));
});

test("the outline nests the ballot tree", () => {
    const items = oval.outline(fixture);
    const compiled = oval.compile(fixture);
    assert.equal(items.length, compiled.ballots.length);
    assert.equal(items[0].kind, "ballot");
    assert.equal(items[0].title, compiled.ballots[0].title);
    assert.equal(
        fixture.slice(items[0].selection.start, items[0].selection.end),
        'ballot @federal "Federal Government"',
    );
    const section = items[0].children[0];
    assert.equal(section.kind, "section");
    assert.equal(section.name, undefined);
    assert.ok(section.children.every((item) => item.kind === "contest" || item.kind === "measure"));
});

test("completions offer keywords and identifiers for a position", () => {
    const source =
        'oval 1\nguide "G"\nelection "E"\ndate 2026-11-03\n' +
        'candidate @delgado "Rosa Delgado"\n' +
        'ballot @city "City"\n  contest @attorney "City Attorney" single\n' +
        "    campaign @delgado\n    recommend\n      support @de\n";
    const atSupport = source.lastIndexOf("@de") + 3;
    const found = oval.completions(source, atSupport);
    assert.deepEqual(
        found.map((c) => [c.label, c.kind, c.detail]),
        [["@delgado", "candidate", "Rosa Delgado"]],
    );
    assert.deepEqual(found[0].replace, { start: atSupport - 3, end: atSupport });
    const atKind = source.indexOf('"City Attorney" ') + '"City Attorney" '.length;
    assert.deepEqual(
        oval.completions(source, atKind).map((c) => c.label),
        ["single", "multiple", "ranked", "retention"],
    );
    assert.deepEqual(oval.completions(source, source.indexOf("Rosa") + 2), []);
});

test("hover describes identifiers and keywords", () => {
    const source =
        'oval 1\nguide "G"\nelection "E"\ndate 2026-11-03\n' +
        'candidate @delgado "Rosa Delgado"\n' +
        'ballot @city "City"\n  contest @attorney "City Attorney" single\n' +
        "    campaign @delgado\n    recommend\n      support @delgado\n";
    const identifier = oval.hover(source, source.lastIndexOf("@delgado") + 2);
    assert.ok(identifier.contents.startsWith("**candidate** `@delgado`\n\nRosa Delgado"));
    assert.equal(source.slice(identifier.range.start, identifier.range.end), "@delgado");
    const keyword = oval.hover(source, source.indexOf("contest") + 1);
    assert.ok(keyword.contents.includes("#sec-Contest"));
    assert.equal(oval.hover(source, source.indexOf('"City"') + 2), undefined);
});

test("hover accepts offsets past the end and inside a character", () => {
    const source = 'oval 1\ncandidate @jose "José"\n';
    const accent = Buffer.byteLength(source.slice(0, source.indexOf("é")));
    assert.equal(oval.hover(source, -1), undefined);
    assert.equal(oval.hover(source, Buffer.byteLength(source) + 5), undefined);
    assert.equal(oval.hover(source, accent + 1), undefined);
});

test("fixes repair insecure urls and near-miss references", () => {
    const source =
        'oval 1\nguide "G"\nelection "E"\ndate 2026-11-03\n' +
        'candidate @delgado "Rosa Delgado"\n' +
        'ballot @city "City"\n  contest @attorney "City Attorney" single\n' +
        "    campaign @delgado\n      website http://delgado.example\n" +
        "    recommend\n      support @delgato\n";
    const found = oval.fixes(source);
    assert.deepEqual(
        found.map((fix) => [fix.code, fix.title]),
        [
            ["insecure-url", "Use https"],
            ["undeclared-reference", "Change to `@delgado`"],
        ],
    );
    const [url] = found;
    assert.equal(source.slice(url.edits[0].range.start, url.edits[0].range.end), "http://");
    assert.equal(url.edits[0].text, "https://");
    assert.equal(url.edits[0].placeholders, undefined);
});

test("a missing guide field is added with its value as a placeholder", () => {
    const source =
        'oval 1\nguide "G"\ndate 2026-11-03\nballot @b "B"\n  measure @m "M"\n    recommend yes\n';
    const [fix] = oval.fixes(source);
    assert.equal(fix.title, "Add `election`");
    const [edit] = fix.edits;
    assert.equal(edit.text, '\nelection ""');
    assert.deepEqual(edit.placeholders, [{ start: 11, end: 11 }]);
});

test("compiled output validates against the specification's schema", () => {
    const Ajv = require("ajv/dist/2020");
    const addFormats = require("ajv-formats");
    const schema = require("@oval-vote/oval-schema/oval.schema.json");
    const ajv = new Ajv({ allErrors: true });
    addFormats(ajv);
    const validate = ajv.compile(schema);
    const compiled = oval.compile(fixture);
    assert.ok(validate(compiled), JSON.stringify(validate.errors, null, 2));
});

test("the typings name the diagnostic identifiers", () => {
    const typings = fs.readFileSync(path.join(__dirname, "../pkg/oval_wasm.d.ts"), "utf8");
    assert.ok(typings.includes("export type DiagnosticIdentifier ="));
    assert.ok(typings.includes("export function check(source: string): Diagnostic[];"));
    assert.ok(typings.includes("export function catalog(): CatalogEntry[];"));
    assert.ok(typings.includes("export function symbols(source: string): Symbols;"));
    assert.ok(typings.includes("export function outline(source: string): OutlineItem[];"));
    assert.ok(
        typings.includes(
            "export function completions(source: string, offset: number): Completion[];",
        ),
    );
    assert.ok(
        typings.includes(
            "export function hover(source: string, offset: number): Hover | undefined;",
        ),
    );
    assert.ok(typings.includes("export function fixes(source: string): Fix[];"));
    assert.ok(typings.includes('export type Compiled = import("@oval-vote/oval-schema").Guide;'));
});

test("diagnostics carry code, severity, range, and message", () => {
    const diagnostics = oval.check("oval 1\nguide yes\n");
    assert.deepEqual(
        diagnostics.map((d) => d.code),
        ["missing-guide-field", "missing-guide-field", "unexpected-token", "empty-document"],
    );
    assert.deepEqual(diagnostics[2], {
        code: "unexpected-token",
        severity: "error",
        range: { start: 13, end: 16 },
        message: "expected a string, found `yes`",
    });
});

test("a document without a version marker gets only that diagnostic", () => {
    assert.deepEqual(
        oval.check("guide yes\n").map((d) => d.code),
        ["unsupported-version"],
    );
});

test("compile and format throw the diagnostics for a broken document", () => {
    assert.throws(
        () => oval.compile("oval 1\nguide yes\n"),
        (thrown) => {
            assert.equal(thrown.length, 4);
            assert.ok(thrown.some((d) => d.code === "unexpected-token"));
            return true;
        },
    );
    assert.throws(
        () => oval.format("oval 1\nguide yes\n"),
        (thrown) => {
            assert.equal(thrown.length, 4);
            return true;
        },
    );
});
