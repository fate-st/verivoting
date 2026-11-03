import fs from 'node:fs';
import assert from 'node:assert/strict';
import {spawnSync,execFileSync} from 'node:child_process';
const [name,...argv]=process.argv.slice(2);assert.match(name??'',/^[a-z0-9-]+$/);assert.ok(argv.length);
const dir='results/p0p1_execution';fs.mkdirSync(dir,{recursive:true});
const p=dir+'/commands.json';const j=fs.existsSync(p)?JSON.parse(fs.readFileSync(p)):{commands:[]};
const n=j.commands.filter(c=>c.name===name).length+1;const log=name+'-'+n+'.log';
const fd=fs.openSync(dir+'/'+log,'wx');const started=new Date().toISOString(),t=performance.now();
const r=spawnSync(argv[0],argv.slice(1),{stdio:['ignore',fd,fd]});fs.closeSync(fd);
j.commands.push({name,argv,started_utc:started,elapsed_ms:performance.now()-t,exit_code:r.status,signal:r.signal,error:r.error?.message??null,log,
source_base_sha:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),git_dirty:!!execFileSync('git',['status','--porcelain'],{encoding:'utf8'}).trim()});
fs.writeFileSync(p,JSON.stringify(j,null,2)+'\n');process.stdout.write(fs.readFileSync(dir+'/'+log));process.exit(r.status??1);
