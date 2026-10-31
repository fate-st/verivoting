// Minimal real-command logger; keeps failed attempts rather than replacing logs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
const [name,...argv]=process.argv.slice(2);
assert.match(name??'',/^[a-z0-9-]+$/); assert.ok(argv.length);
const raw='experiments/g8b_raw';fs.mkdirSync(raw,{recursive:true});
const file=`${raw}/commands.json`,env={CARGO_BUILD_JOBS:'2',CARGO_PROFILE_RELEASE_STRIP:'none'};
const record=fs.existsSync(file)?JSON.parse(fs.readFileSync(file)):{working_directory:process.cwd(),env,commands:[]};
const attempt=1+record.commands.filter(x=>x.name===name).length;
const log=`${name}${attempt===1?'':`-attempt-${attempt}`}.log`;
const fd=fs.openSync(`${raw}/${log}`,'wx'),start=performance.now();
const r=spawnSync(argv[0],argv.slice(1),{env:{...process.env,...env},stdio:['ignore',fd,fd]});fs.closeSync(fd);
record.commands.push({name,attempt,argv,log,exit_code:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:performance.now()-start});
fs.writeFileSync(file,`${JSON.stringify(record,null,2)}\n`);
process.stdout.write(fs.readFileSync(`${raw}/${log}`));process.exit(r.status??1);
