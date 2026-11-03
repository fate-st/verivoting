// Limited acceptance of evidence synchronization and the blocked design inventory.
// This does not execute proofs, contracts or the full SnarkPack verifier.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
process.chdir(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const run=(exe,args,opts={})=>{const b=execFileSync(exe,args,{encoding:'utf8',...opts});if(b)process.stdout.write(b);};
for(const p of ['scripts/p0_paper_evidence.mjs','scripts/g9p1_design.mjs','scripts/p0p1_record.mjs','scripts/p0p1_check.mjs'])run('node',['--check',p]);
run('python3',['-c','import ast,sys; ast.parse(sys.stdin.read()); print("summarizer AST: PASS")'],{input:fs.readFileSync('scripts/summarize.py','utf8')});
run('git',['diff','--check']);
const permitted=['README.md','docs/IMPLEMENTATION_STATUS.md','docs/NEXT_STEPS.md','scripts/summarize.py','experiments/aggregate/summary.json','experiments/aggregate/summary.md'];
const changed=execFileSync('git',['diff','--name-only','a5204043bdcec5a788f79de2ad79225320845e64'],{encoding:'utf8'}).trim().split('\n').filter(Boolean);
// New P0/P1 artifacts may be untracked or committed in a later reproduction.
const newPath=p=>['docs/G9_P1_STAGED_EXACT_DESIGN.md','docs/P0_PAPER_EVIDENCE_SYNC.md','scripts/g9p1_design.mjs','scripts/p0_paper_evidence.mjs','scripts/p0p1_record.mjs','scripts/p0p1_check.mjs'].includes(p)||['results/p0_paper_sync/','results/g9_p1/','results/p0p1_execution/'].some(prefix=>p.startsWith(prefix));
for(const p of changed)assert.ok(permitted.includes(p)||newPath(p),p);
const untracked=execFileSync('git',['ls-files','--others','--exclude-standard'],{encoding:'utf8'}).trim().split('\n').filter(Boolean);
for(const p of untracked)assert.ok(newPath(p),p);
const snapshot=fs.readFileSync('results/p0_paper_sync/sn-article.synced.tex');
const summary=JSON.parse(fs.readFileSync('experiments/aggregate/summary.json'));
assert.equal(summary.p1.constraints,3521);assert.equal(summary.p1.verified,true);
assert.equal(summary.gates.P0_paper_sync,'PASS_PAPER_EVIDENCE_SYNC');
assert.equal(summary.gates.P1_staged_exact_design,'BLOCKED_P1_RESOURCE_AND_EQUIVALENCE_GATES');
run('node',['scripts/p0_paper_evidence.mjs','--check']);
run('node',['scripts/g9p1_design.mjs','--check']);
run('node',['scripts/g9_route_review.mjs','--check']);
run('node',['scripts/g8e_accounting.mjs','--check']);
const [paperPath,originalPath]=process.argv.slice(2);
if(paperPath){
  const paper=fs.readFileSync(paperPath);assert.equal(sha(paper),sha(snapshot));
  run('node',['scripts/p0_paper_evidence.mjs','--check','--paper',paperPath]);
  if(originalPath){
    const original=fs.readFileSync(originalPath,'utf8'),tex=paper.toString();
    const intro=s=>s.slice(s.indexOf('\\section{Introduction}'),s.indexOf('\\section{Preliminaries'));
    assert.ok(intro(original).length>1000);assert.equal(intro(tex),intro(original));
    const formal=s=>[...s.matchAll(/\\begin\{(definition|theorem|lemma|proposition|corollary)\}[\s\S]*?\\end\{\1\}/g)].map(m=>m[0]);
    assert.equal(formal(original).length,15);assert.deepEqual(formal(tex),formal(original));
    console.log('Original Introduction and all 15 formal environments unchanged: PASS');
  }
}
console.log(JSON.stringify({scope:'P0 evidence and P1 design only',paper_sha256:sha(snapshot),historical_crypto_and_raw_tracked_bytes_unchanged:true,historical_P1_constraints:summary.p1.constraints,git_diff_check:'PASS',new_evm:0,new_setup:0,new_prove:0,P2_started:false}));
