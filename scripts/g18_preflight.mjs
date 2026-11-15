// Read-only probes; no Docker installation, pulls, backend fallback or proving.
import fs from 'node:fs';
import os from 'node:os';
import assert from 'node:assert/strict';
import path from 'node:path';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const run=process.argv[2],config=JSON.parse(fs.readFileSync('configs/experiment.G18-DECISION.json'));
const sha=b=>createHash('sha256').update(b).digest('hex');
const probes=[];
const probe=(name,program,args)=>{
 const j=spawnSync(program,args,{encoding:'utf8',timeout:20000});
 const p={name,program,args,exit_code:j.status,signal:j.signal,error:j.error?.message??null,
  stdout:j.stdout??'',stderr:j.stderr??''};probes.push(p);return p;
};
const sdk=path.resolve('target/g17-tools/risc0/extensions/v3.1.0-cargo-risczero-aarch64-apple-darwin');
probe('r0vm-version',path.join(sdk,'r0vm'),['--version']);
probe('r0vm-capabilities',path.join(sdk,'r0vm'),['--help']);
const docker=probe('docker-version','docker',['--version']);
const cuda=probe('cuda-device','nvidia-smi',[]);
probe('metal-compiler','xcrun',['--find','metal']);
probe('developer-tools','xcode-select',['-p']);
const system=probe('system-version','sw_vers',[]);
const r0vmBytes=fs.readFileSync(path.join(sdk,'r0vm'));
const expected=JSON.parse(fs.readFileSync('experiments/zkvm_abc_raw/run-20261115-03/installation.json'))
 .records.flatMap(r=>r.binaries).find(b=>b.path.endsWith('/r0vm')).sha256;
assert.equal(sha(r0vmBytes),expected);
assert.equal(os.arch(),config.environment.arch);assert.equal(os.platform(),config.environment.platform);
const record={schema:'VV/G18/ENVIRONMENT-PREFLIGHT/v1',platform:os.platform(),arch:os.arch(),cpu:os.cpus()[0].model,
 memory_bytes:os.totalmem(),system_version:system.stdout,probes,r0vm_sha256:sha(r0vmBytes),
 original_official_r0vm_unchanged:true,local_prover_entrypoint_present:true,
 local_stark_proving_available:'UNKNOWN_UNTIL_OFFICIAL_SMOKE',docker_cli_available:docker.exit_code===0,
 cuda_available:cuda.exit_code===0,metal_runtime_proving_available:'UNKNOWN_NOT_INFERRED_FROM_XCRUN',
 required_groth16_wrapper_available:false,required_groth16_wrapper_status:'BLOCKED_APPLE_SILICON_UNSUPPORTED',
 wrapper_reasons:['Current fixed platform is darwin arm64/Apple M4; official 3.1 local-proving documentation excludes Apple Silicon for Groth16 shrink-wrap',
  ...(docker.exit_code===0?[]:['Docker CLI not installed']),...(cuda.exit_code===0?[]:['CUDA/NVIDIA backend not installed'])],
 official_documentation:config.official_sources,official_installed_source:config.official_smoke.official_files.at(-1),
 environment_admitted:false,b1_ballot_prove_admitted:false,
 smoke_authorization:'Only the one predeclared official Composite proof smoke; does not admit B1/A/B/Gas',
 automatic_platform_or_backend_switch:false,dev_mode:false,remote_service_used:false};
fs.writeFileSync(path.join(run,'environment_preflight.json'),JSON.stringify(record,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(record,null,2));
