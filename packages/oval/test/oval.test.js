const assert = require("node:assert");
const { test } = require("node:test");

const oval = require("..");
const { version } = require("../package.json");

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

test("check throws when the source can't be a document", () => {
    assert.throws(() => oval.check("oval 1\0"), Error);
});
