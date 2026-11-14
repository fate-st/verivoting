// Index archived evidence; does not rerun a prover, relabel historical commits or rank systems.
import fs from 'node:fs';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const sha=b=>createHash('sha256').update(b).digest('hex');
const git=(...a)=>execFileSync('git',a,{encoding:'utf8'}).trim();
const files=git('ls-files','-z').split('\0').filter(Boolean);
const selected=files.filter(p=>/^experiments\/g[^/]+_raw\/[^/]+\.json$/.test(p)||/^results\/g[^/]+\/(?:summary|cost_ledger|functional_summary|lifecycle_costs)\.json$/.test(p)||p==='experiments/g11/baselines/belenios/run-20261106-01/environment.json'||p==='experiments/g11/baselines/belenios/run-20261106-01/run.json'||p==='experiments/g11/literature/literature_summary.json');
const groups=new Map();
for(const p of selected){
 const id=p.split('/')[1].replace(/_raw$/,'');
 const x=JSON.parse(fs.readFileSync(p));
 const scalars=Object.fromEntries(Object.entries(x).filter(([k,v])=>
  (v===null||['string','number','boolean'].includes(typeof v))&&
  /status|^git_commit$|^git_sha$|^git_head$|^git_dirty$|source_base_sha|source_version_sha256|binary_sha256|build|thread|hardware|^curve$|suite|layout|circuit|vk.*sha256|srs.*sha256|manifest.*sha256|config.*sha256|setup_count|proof_count|reuse|batch_size|registered|candidate|^n$|^tau$|^t$|^protocol|^schema$/.test(k)&&
  !k.includes('secret')&&!k.includes('private_tally')));
 const metrics=Object.entries(x).filter(([k,v])=>/ms$|gas$|_bytes$|rss.*mib|constraints/.test(k)&&['number','boolean'].includes(typeof v)).map(([field,value])=>({field,value}));
 const command_files=files.filter(q=>q.startsWith(p.slice(0,p.lastIndexOf('/')+1))&&/\/(commands|manifest|execution)\.json$/.test(q));
 const entry={path:p,sha256:sha(fs.readFileSync(p)),metadata:scalars,metrics,command_evidence:command_files,
  source_commit: x.source_base_sha??x.git_commit??x.git_sha??x.git_head??x.base_commit??null};
 if(!groups.has(id))groups.set(id,[]);groups.get(id).push(entry);
}
const fixed=['configs/protocol_manifest.JUBJUB-EVAL-D5-W32.json','crates/vv-friendly-dkg/src/registration.rs',
 'crates/vv-friendly-crypto/src/ballot_identity_gadget.rs','crates/vv-friendly-crypto/src/state_gadget.rs',
 'contracts/src/auth/VeriVotingCertificateGateV2.sol'];
const result={schema:'VV/STAGE2/ARCHIVED-EXPERIMENT-INVENTORY/v1',inventory_base_commit:'c160a430c0aa971250626e520209621c7673d389',
 inventory_head_commit:git('rev-parse','HEAD'),
 experiment_families:[...groups].map(([family,evidence])=>({family,evidence})),
 frozen_files:fixed.map(p=>({path:p,sha256:sha(fs.readFileSync(p))})),
 limitations:[
  'This is an index of archived metadata and measured fields, not a re-execution of every historic experiment.',
  'Historical dirty-source status, missing commit fields (null) and differing suites remain visible; current HEAD is not assigned to old data.',
  'Command evidence points to original logs/manifests. A per-experiment manifest may be incomplete; missing information is not fabricated.',
  'This stage runs only six pinned archive/report checks plus the new bounded pilot, not all historic check wrappers.',
  'Persistent RegisterVoter/global ledger, partial trees, general multi-region service, independent DKG, exact SnarkPack EVM and T4/T5 remain open.'
 ]};
if(process.argv[2]==='--check') {
 const old=JSON.parse(fs.readFileSync(process.argv[3]));assert.deepEqual(result,old);
 console.log(JSON.stringify({status:'PASS_ARCHIVED_INVENTORY_HASHES_AND_METADATA',families:result.experiment_families.length,evidence_files:selected.length}));
} else {
 assert.ok(process.argv[2],'usage: node scripts/stage2_inventory.mjs NEW_JSON | --check EXISTING_JSON');
 fs.writeFileSync(process.argv[2],JSON.stringify(result,null,2)+'\n',{flag:'wx'});
 console.log(JSON.stringify({status:'PASS_INVENTORY_INDEX_CREATED',families:result.experiment_families.length,evidence_files:selected.length}));
}
