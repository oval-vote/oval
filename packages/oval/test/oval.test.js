const assert = require("node:assert");
const { test } = require("node:test");

const oval = require("..");
const { version } = require("../package.json");

test("the package reports its version and the language it reads", () => {
    assert.strictEqual(oval.version(), version);
    assert.strictEqual(oval.languageMajor(), 1);
});
