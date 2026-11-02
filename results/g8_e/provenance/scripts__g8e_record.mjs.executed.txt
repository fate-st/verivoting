// Command evidence for derived accounting only; never modify old phase logs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
const [name, ...argv] = process.argv.slice(2);
assert.match(name ?? '', /^[a-z0-9-]+$/); assert.ok(argv.length);
const dir = 'results/g8_e/execution'; fs.mkdirSync(dir, {recursive: true});
const file = `${dir}/commands.json`;
const report = fs.existsSync(file) ? JSON.parse(fs.readFileSync(file)) : {working_directory: process.cwd(), commands: []};
const attempt = 1 + report.commands.filter(c => c.name === name).length;
const log = `${name}${attempt === 1 ? '' : `-attempt-${attempt}`}.log`;
const fd = fs.openSync(`${dir}/${log}`, 'wx'), start = performance.now();
const r = spawnSync(argv[0], argv.slice(1), {stdio: ['ignore', fd, fd]}); fs.closeSync(fd);
report.commands.push({name, attempt, argv, log, exit_code: r.status, signal: r.signal,
  error: r.error?.message ?? null, elapsed_ms: performance.now() - start});
fs.writeFileSync(file, JSON.stringify(report, null, 2) + '\n');
process.stdout.write(fs.readFileSync(`${dir}/${log}`)); process.exit(r.status ?? 1);
