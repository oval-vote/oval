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

test("the package names the language major it reads", () => {
  assert.equal(oval.languageMajor(), 1);
});

test("diagnostics carry code, severity, range, and message", () => {
  const diagnostics = oval.check("oval 1\nguide yes\n");
  assert.deepEqual(
    diagnostics.map((d) => d.code),
    ["V2", "V2", "syntax"],
  );
  assert.deepEqual(diagnostics[2], {
    code: "syntax",
    severity: "error",
    range: { start: 13, end: 16 },
    message: "expected a string, found `yes`",
  });
});

test("a document without a version marker gets only that diagnostic", () => {
  assert.deepEqual(
    oval.check("guide yes\n").map((d) => d.code),
    ["V19"],
  );
});

test("compile and format throw the diagnostics for a broken document", () => {
  assert.throws(() => oval.compile("oval 1\nguide yes\n"), (thrown) => {
    assert.equal(thrown.length, 3);
    assert.ok(thrown.some((d) => d.code === "syntax"));
    return true;
  });
  assert.throws(() => oval.format("oval 1\nguide yes\n"), (thrown) => {
    assert.equal(thrown.length, 3);
    return true;
  });
});
