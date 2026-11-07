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
    assert.equal(catalog.length, 54);
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

test("the typings name the diagnostic identifiers", () => {
    const typings = fs.readFileSync(path.join(__dirname, "../pkg/oval_wasm.d.ts"), "utf8");
    assert.ok(typings.includes("export type DiagnosticIdentifier ="));
    assert.ok(typings.includes("export function check(source: string): Diagnostic[];"));
    assert.ok(typings.includes("export function catalog(): CatalogEntry[];"));
    assert.ok(typings.includes("export function symbols(source: string): Symbols;"));
    assert.ok(typings.includes("export function outline(source: string): OutlineItem[];"));
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
