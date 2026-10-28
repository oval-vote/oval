import { readdirSync, readFileSync, existsSync, statSync } from 'node:fs';
import { join } from 'node:path';
import Ajv from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';

const ajv = new Ajv({ strict: true, allErrors: true });
addFormats(ajv);

let failures = 0;
const fail = (msg) => { failures++; console.log(`✗ ${msg}`); };
const ok = (msg) => console.log(`✓ ${msg}`);

// 1. Schemas compile.
const schemaFiles = [];
for (const major of readdirSync('schema')) {
  const dir = join('schema', major);
  if (!statSync(dir).isDirectory()) continue;
  for (const f of readdirSync(dir).filter((f) => f.endsWith('.json'))) schemaFiles.push(join(dir, f));
}
for (const file of schemaFiles) {
  try {
    const schema = JSON.parse(readFileSync(file, 'utf8'));
    ajv.addSchema(schema);
    ajv.getSchema(schema.$id); // forces compilation
    ok(`${file} compiles (${schema.$id})`);
  } catch (e) {
    fail(`${file}: ${e.message}`);
  }
}

// 2. Documents validate or are rejected as expected.
const docs = (dir) => (existsSync(dir) ? readdirSync(dir).filter((f) => f.endsWith('.json')).map((f) => join(dir, f)) : []);
const validate = (file) => {
  const doc = JSON.parse(readFileSync(file, 'utf8'));
  const check = doc && typeof doc.$schema === 'string' ? ajv.getSchema(doc.$schema) : undefined;
  if (!check) return [`$schema is missing or not one of the schemas in this repo`];
  return check(doc) ? [] : check.errors.map((e) => `${e.instancePath || '/'}: ${e.message}`);
};

for (const file of [...docs('examples'), ...docs('conformance/valid')]) {
  const errors = validate(file);
  if (errors.length) fail(`${file} should be valid:\n    ${errors.join('\n    ')}`);
  else ok(file);
}
for (const file of docs('conformance/invalid')) {
  let errors;
  try { errors = validate(file); } catch (e) { errors = [e.message]; }
  if (!errors.length) fail(`${file} should be invalid`);
  else ok(`${file} rejected: ${errors[0]}`);
}

console.log(failures ? `${failures} failure(s)` : 'all checks passed');
process.exit(failures ? 1 : 0);
