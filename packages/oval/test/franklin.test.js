const assert = require("node:assert");
const fs = require("node:fs");
const path = require("node:path");
const { test } = require("node:test");
const Ajv2020 = require("ajv/dist/2020");
const addFormats = require("ajv-formats");
const schema = require("@oval-vote/oval-schema/oval.schema.json");

const oval = require("..");

const fixture = (name) => fs.readFileSync(path.join(__dirname, "fixtures", name), "utf8");
const source = fixture("franklin.oval");
const expected = JSON.parse(fixture("franklin.oval.json"));

test("the specification's example guide has no problems", () => {
    assert.deepStrictEqual(oval.check(source), []);
});

test("the example guide compiles to the specification's output", () => {
    const compiled = oval.compile(source);
    assert.deepStrictEqual(compiled, expected);
    const validate = addFormats(new Ajv2020({ strict: false })).compile(schema);
    assert.ok(validate(compiled), JSON.stringify(validate.errors));
});

test("the example guide is already formatted", () => {
    assert.strictEqual(oval.format(source), source);
});

test("every identifier in the example guide names its declaration", () => {
    using analysis = new oval.Analysis(source);
    assert.deepStrictEqual(analysis.fixes(), []);
    for (const match of source.matchAll(/@([a-z0-9_-]+)/g)) {
        const definition = analysis.definition(match.index);
        assert.ok(definition, `${match[0]} at ${match.index}`);
        assert.strictEqual(definition.id, match[1]);
        assert.strictEqual(
            source.slice(definition.idRange.start, definition.idRange.end),
            match[1],
        );
        const occurrences = analysis.occurrences(match.index);
        assert.ok(occurrences.some((o) => o.range.start === match.index));
        assert.strictEqual(
            occurrences.filter((o) => o.isDeclaration)[0].range.start,
            definition.range.start,
        );
    }
});

test("the example guide's outline follows its compiled ballots", () => {
    using analysis = new oval.Analysis(source);
    const tree = (items, children) =>
        items.map((item) => [
            item.kind,
            item.id ?? item.title,
            tree(children(item) ?? [], children),
        ]);
    assert.deepStrictEqual(
        tree(analysis.outline(), (item) => item.children),
        tree(expected.ballots, (item) =>
            item.kind === "contest" || item.kind === "measure" ? [] : item.items,
        ),
    );
});
