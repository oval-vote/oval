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
