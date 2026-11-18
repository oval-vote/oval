const assert = require("node:assert");
const { test } = require("node:test");
const Ajv2020 = require("ajv/dist/2020");
const addFormats = require("ajv-formats");
const schema = require("@oval-vote/oval-schema/oval.schema.json");

const oval = require("..");
const { version } = require("../package.json");

const GUIDE = `oval 1
guide "Guía"
election "General"
date 2026-11-03
ballot @city "City"
  measure @m "Measure M"
    recommend yes
`;

function thrown(run) {
    try {
        run();
    } catch (error) {
        return error;
    }
    assert.fail("expected a throw");
}

test("the package reports its version and the language it reads", () => {
    assert.strictEqual(oval.version(), version);
    assert.strictEqual(oval.languageMajor(), 1);
});

test("a diagnostic's range indexes the source string", () => {
    const declaration = 'party @p "Pé"';
    const source = `oval 1\nguide "🗳 Guía"\nelection "E"\ndate 2026-11-03\n${declaration}\n`;
    const [unused] = oval.check(source);
    assert.strictEqual(unused.code, "unused-declaration");
    assert.strictEqual(unused.severity, "warning");
    assert.strictEqual(source.slice(unused.range.start, unused.range.end), declaration);
});

test("a rejected source throws an Error at the string index", () => {
    for (const run of [oval.check, oval.compile, oval.format]) {
        const error = thrown(() => run("🗳\0"));
        assert.ok(error instanceof Error);
        assert.strictEqual(error.name, "Error");
        assert.strictEqual(error.message, "the document contains U+0000 at index 2");
    }
});

test("compile returns a guide the schema accepts", () => {
    const validate = addFormats(new Ajv2020({ strict: false })).compile(schema);
    const compiled = oval.compile(GUIDE);
    assert.ok(validate(compiled), JSON.stringify(validate.errors));
    assert.strictEqual(compiled.guide.name, "Guía");
});

test("compile throws the document's errors but not its warnings", () => {
    const source = GUIDE.replace("date 2026-11-03", "date 2026-02-30")
        .replace("ballot @city", 'party @p "P"\nballot @city')
        .replace("    recommend yes\n", "");
    assert.ok(oval.check(source).some((d) => d.severity === "warning"));
    const error = thrown(() => oval.compile(source));
    assert.ok(error instanceof Error);
    assert.strictEqual(error.name, "OvalError");
    assert.strictEqual(error.message, "the document has 3 errors");
    assert.deepStrictEqual(
        error.diagnostics.map((diagnostic) => diagnostic.code),
        ["invalid-date", "missing-recommendation", "missing-block"],
    );
});

test("format formats an unfinished guide but throws on a syntax error", () => {
    assert.strictEqual(oval.format('oval 1\nguide   "G"  # name\n'), 'oval 1\nguide "G" # name\n');
    const error = thrown(() => oval.format('oval 1\nguide "G\ndate 2026-02-30\n'));
    assert.strictEqual(error.name, "OvalError");
    assert.strictEqual(error.message, "the document has an error");
    assert.deepStrictEqual(
        error.diagnostics.map((diagnostic) => diagnostic.code),
        ["unterminated-string"],
    );
});

test("an analysis answers in string indices", () => {
    const source = `oval 1\nguide "🗳 Guía"\nparty @dem "Demócrata"\ncandidate @n "Nadia"\n  party @dem\nballot @b "Ciudad"\n  contest @k "Alcalde" single\n    campaign @n\n    recommend\n      support @n\n`;
    const analysis = new oval.Analysis(source);
    try {
        const text = (range) => source.slice(range.start, range.end);
        assert.deepStrictEqual(analysis.diagnostics(), oval.check(source));

        const definition = analysis.definition(source.lastIndexOf("@dem"));
        assert.strictEqual(definition.entity, "party");
        assert.strictEqual(definition.id, "dem");
        assert.strictEqual(definition.name, "Demócrata");
        assert.strictEqual(text(definition.range), "@dem");
        assert.strictEqual(definition.range.start, source.indexOf("@dem"));
        assert.strictEqual(analysis.definition(source.indexOf("Nadia")), null);

        const occurrences = analysis.occurrences(source.lastIndexOf("@n") + 2);
        assert.deepStrictEqual(
            occurrences.map((o) => [text(o.range), text(o.idRange), o.isDeclaration]),
            [
                ["@n", "n", true],
                ["@n", "n", false],
                ["@n", "n", false],
            ],
        );

        const hover = analysis.hover(source.indexOf("single") + 2);
        assert.strictEqual(text(hover.range), "single");
        assert.strictEqual(analysis.hover(source.indexOf("Ciudad")), null);

        const [ballot] = analysis.outline();
        assert.strictEqual(ballot.kind, "ballot");
        assert.strictEqual(text(ballot.selection), "@b");
        assert.strictEqual(ballot.children[0].title, "Alcalde");
        assert.ok(text(ballot.range).endsWith("support @n\n"));
    } finally {
        analysis.free();
    }
});

test("an analysis of a rejected source throws an Error", () => {
    assert.throws(() => new oval.Analysis("\0"), {
        name: "Error",
        message: "the document contains U+0000 at index 0",
    });
});

test("an analysis completes and fixes in string indices", () => {
    const source = `# Guía 🗳\noval 1\nguide "G"\ndate 2026-11-03\nparty @dem "Demócrata"\ncandidate @n "Nadia"\n  party @d`;
    const analysis = new oval.Analysis(source);
    try {
        const [dem] = analysis.completions(source.length);
        assert.deepStrictEqual(dem, {
            label: "@dem",
            kind: "party",
            detail: "Demócrata",
            replace: { start: source.length - 2, end: source.length },
        });

        const diagnostics = analysis.diagnostics();
        const fix = analysis
            .fixes()
            .find((fix) => diagnostics[fix.diagnostic].code === "missing-guide-field");
        const { range, text, placeholder } = fix.edit;
        const fixed = source.slice(0, range.start) + text + source.slice(range.end);
        assert.ok(fixed.includes('guide "G"\nelection ""\ndate'));
        assert.strictEqual(text.slice(0, placeholder.start), 'election "');
    } finally {
        analysis.free();
    }
});
