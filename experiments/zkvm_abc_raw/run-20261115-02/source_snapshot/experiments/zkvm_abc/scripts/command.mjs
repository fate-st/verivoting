// One bounded, logged command. No proving or network service is selected here.
import fs from 'node:fs';
import path from 'node:path';
import {spawn, execFileSync} from 'node:child_process';
import {performance} from 'node:perf_hooks';
import assert from 'node:assert/strict';
const [dir, name, seconds, program, ...args] = process.argv.slice(2);
assert.ok(dir && /^[a-z0-9_-]+$/.test(name) && Number(seconds) > 0 && program);
fs.mkdirSync(path.join(dir, 'logs'), {recursive:true});
const output = path.join(dir, `${name}.command.json`);
assert.ok(!fs.existsSync(output), 'Use a new run/command name; do not overwrite evidence.');
const started = performance.now(), startedUtc = new Date().toISOString();
const out = fs.openSync(path.join(dir,'logs',name+'.stdout.log'),'wx');
const err = fs.openSync(path.join(dir,'logs',name+'.stderr.log'),'wx');
const repo = process.cwd();
const env = {...process.env, CARGO_BUILD_JOBS:'2', RISC0_DEV_MODE:'0',
  RISC0_HOME:path.join(repo,'target/g17-tools/risc0')};
// No Bonsai/network prover credentials are read or forwarded to experiments.
for(const key of Object.keys(env)) if (/^(BONSAI_|RISC0_PROVER|RISC0_EXECUTOR)/.test(key)) delete env[key];
const c = spawn(program,args,{env,detached:true,stdio:['ignore',out,err]});
let maximumProcessKiB=0, maximumTreeKiB=0, samples=0, termination=null, spawnError=null;
const stop=reason=>{if(termination)return;termination=reason;try{process.kill(-c.pid,'SIGTERM')}catch{}setTimeout(()=>{try{process.kill(-c.pid,'SIGKILL')}catch{}},1500).unref()};
const poll=setInterval(()=>{
  if((performance.now()-started)/1000>Number(seconds)) stop('TIME_BUDGET');
  try {
    const ps=execFileSync('/bin/ps',['-axo','pid=,ppid=,rss='],{encoding:'utf8'}).trim().split('\n')
      .map(l=>l.trim().split(/\s+/).map(Number));
    const ids=new Set([c.pid]);let changed=true;
    while(changed){changed=false;for(const [pid,ppid]of ps)if(ids.has(ppid)&&!ids.has(pid)){ids.add(pid);changed=true}}
    const rss=ps.filter(([pid])=>ids.has(pid)).map(r=>r[2]);
    if(rss.length){samples++;maximumProcessKiB=Math.max(maximumProcessKiB,...rss);maximumTreeKiB=Math.max(maximumTreeKiB,rss.reduce((a,b)=>a+b,0));
      if(Math.max(...rss)>16*1024*1024)stop('PROCESS_RSS_BUDGET');}
  }catch{}
},1000);
const heart=setInterval(()=>console.log(`${name}: running ${Math.round((performance.now()-started)/1000)}s, sampled process/tree peak ${(maximumProcessKiB/1024).toFixed(1)}/${(maximumTreeKiB/1024).toFixed(1)} MiB`),20000);
c.on('error',e=>{spawnError=e.message});
c.on('close',(code,signal)=>{
  clearInterval(poll);clearInterval(heart);fs.closeSync(out);fs.closeSync(err);
  const j={name,program,args,cwd:repo,started_utc:startedUtc,finished_utc:new Date().toISOString(),
    exit_code:code,signal,spawn_error:spawnError,termination,wall_ms:performance.now()-started,
    timeout_seconds:Number(seconds),single_process_rss_limit_mib:16384,
    sampled_max_single_process_rss_mib:samples?maximumProcessKiB/1024:null,
    sampled_max_process_tree_rss_mib:samples?maximumTreeKiB/1024:null,rss_sample_count:samples,
    memory_scope:'Polled RSS, not exact high-water mark or guest memory. Null if no process sample.',
    env_overrides:{CARGO_BUILD_JOBS:'2',RISC0_DEV_MODE:'0',RISC0_HOME:env.RISC0_HOME},
    stdout:`logs/${name}.stdout.log`,stderr:`logs/${name}.stderr.log`};
  fs.writeFileSync(output,JSON.stringify(j,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(j,null,2));
  process.exitCode=code===0&&!termination&&!spawnError?0:1;
});
