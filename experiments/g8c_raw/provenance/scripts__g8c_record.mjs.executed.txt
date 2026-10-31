// Minimal real-command logger for this phase; never rewrites old phase logs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {spawnSync} from 'node:child_process';
const [name,...argv]=process.argv.slice(2);assert.match(name??'',/^[a-z0-9-]+$/);assert.ok(argv.length);
const raw='experiments/g8c_raw';fs.mkdirSync(raw,{recursive:true});
const file=`${raw}/commands.json`,report=fs.existsSync(file)?JSON.parse(fs.readFileSync(file)):{working_directory:process.cwd(),commands:[]};
const attempt=1+report.commands.filter(r=>r.name===name).length,log=`${name}${attempt===1?'':`-attempt-${attempt}`}.log`;
const fd=fs.openSync(`${raw}/${log}`,'wx'),start=performance.now();
const r=spawnSync(argv[0],argv.slice(1),{stdio:['ignore',fd,fd]});fs.closeSync(fd);
report.commands.push({name,attempt,argv,log,exit_code:r.status,signal:r.signal,error:r.error?.message??null,elapsed_ms:performance.now()-start});
fs.writeFileSync(file,`${JSON.stringify(report,null,2)}\n`);process.stdout.write(fs.readFileSync(`${raw}/${log}`));process.exit(r.status??1);
