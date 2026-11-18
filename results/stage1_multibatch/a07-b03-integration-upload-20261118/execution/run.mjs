import fs from 'node:fs';
import cp from 'node:child_process';
import path from 'node:path';
const task=path.dirname(new URL(import.meta.url).pathname),worktree=task+'/worktree';
const a='experiments/stage1_multibatch_raw/gate-a-20261117-07',b='experiments/stage1_multibatch_raw/gate-b-20261117-03';
const commands={
 ac3:['scripts/stage1_multibatch_audit.mjs','--reuse-schema-self-test','true'],
 a07:['contracts/scripts/stage1_multibatch_compare.mjs','--gate','A','--run',a],
 'a07-audit':['scripts/stage1_multibatch_audit.mjs','--gate','A','--run',a,'--reuse-integration','true'],
 b03:['contracts/scripts/stage1_multibatch_compare.mjs','--gate','B','--approved','B','--materials-only','true','--run',b,'--a-run',a],
 'b03-audit':['scripts/stage1_multibatch_audit.mjs','--gate','B','--run',b,'--reuse-integration','true']
};
const name=process.argv[2];if(!commands[name])throw Error('Unknown authorized stage');
const args=commands[name],env={...process.env,VV_INTEGRATION_ORIGINAL_ROOT:'/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter',VV_INTEGRATION_TARGET_DIR:task+'/target'};
for(const k of Object.keys(env))if(k.startsWith('CARGO_PROFILE_RELEASE_'))delete env[k];
fs.writeFileSync(task+'/'+name+'.intent.json',JSON.stringify({command:[process.execPath,...args],cwd:worktree,started:new Date().toISOString(),one_invocation_only:true},null,2)+'\n',{flag:'wx'});
const out=fs.openSync(task+'/'+name+'.stdout.log','wx'),err=fs.openSync(task+'/'+name+'.stderr.log','wx'),start=performance.now();
const child=cp.spawn(process.execPath,args,{cwd:worktree,env});
child.stdout.on('data',b=>{fs.writeSync(out,b);process.stdout.write(b);});child.stderr.on('data',b=>{fs.writeSync(err,b);process.stderr.write(b);});
child.once('close',(code,signal)=>{fs.closeSync(out);fs.closeSync(err);fs.writeFileSync(task+'/'+name+'.execution.json',JSON.stringify({command:[process.execPath,...args],cwd:worktree,code,signal,wall_ms:performance.now()-start},null,2)+'\n',{flag:'wx'});process.exitCode=code??1;});
