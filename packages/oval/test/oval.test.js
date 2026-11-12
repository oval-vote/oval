const assert = require("node:assert");
const { test } = require("node:test");

test("the package loads", () => {
    assert.ok(require("../pkg/oval_wasm.js"));
});
