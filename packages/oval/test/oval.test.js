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
  assert.equal(compiled.oval, "0.1");
  assert.equal(compiled.counts.contests, 44);
  assert.equal(compiled.counts.measures, 28);
  assert.equal(compiled.ballots[0].kind, "ballot");
  assert.equal(compiled.questions.length, 72);
});

test("format leaves a formatted document unchanged", () => {
  assert.equal(oval.format(fixture), fixture);
});

test("diagnostics carry code, severity, range, and message", () => {
  const diagnostics = oval.check("guide yes\n");
  assert.deepEqual(
    diagnostics.map((d) => d.code),
    ["V2", "V2", "syntax"],
  );
  assert.deepEqual(diagnostics[2], {
    code: "syntax",
    severity: "error",
    range: { start: 6, end: 9 },
    message: "expected a string, found `yes`",
  });
});

test("compile and format throw the diagnostics for a broken document", () => {
  assert.throws(() => oval.compile("guide yes\n"), (thrown) => {
    assert.equal(thrown.length, 3);
    assert.ok(thrown.some((d) => d.code === "syntax"));
    return true;
  });
  assert.throws(() => oval.format("guide yes\n"), (thrown) => {
    assert.equal(thrown.length, 3);
    return true;
  });
});
