// Per-command environment only. Does not mutate rustup defaults or shell profiles.
import path from 'node:path';
import fs from 'node:fs';
import {execFileSync, spawnSync} from 'node:child_process';
const [kind,...args]=process.argv.slice(2);
const repo=process.cwd(),cargo=execFileSync('rustup',['which','--toolchain','1.99.1','cargo'],{encoding:'utf8'}).trim();
const sdk=path.join(repo,'target/g17-tools/risc0/extensions/v3.1.0-cargo-risczero-aarch64-apple-darwin');
const env={...process.env,PATH:path.dirname(cargo)+path.delimiter+process.env.PATH,
 RISC0_HOME:path.join(repo,'target/g17-tools/risc0'),RISC0_SERVER_PATH:path.join(sdk,'r0vm'),
 RISC0_EXECUTOR:'ipc',RISC0_DEV_MODE:'0',CARGO_BUILD_JOBS:'2',
 // Host rust-objcopy aborts in dyld on this machine. Keep debug sections instead;
 // risc0-build sanitizes CARGO_* for the guest, so its ELF is unchanged.
 CARGO_PROFILE_RELEASE_STRIP:'none',
 HTTPS_PROXY:'http://127.0.0.1:7890',HTTP_PROXY:'http://127.0.0.1:7890'};
for(const key of Object.keys(env))if(key.startsWith('BONSAI_')||key.startsWith('RISC0_PROVER'))delete env[key];
delete env.RISC0_SKIP_BUILD;
if(fs.existsSync('experiments/zkvm_abc/guest_b_groth16/Cargo.lock'))env.RISC0_BUILD_LOCKED='1';
else delete env.RISC0_BUILD_LOCKED;
const binaries={cargo,host:path.join(repo,'experiments/zkvm_abc/target/release/g17-host'),
 sdk:path.join(sdk,'cargo-risczero'),r0vm:path.join(sdk,'r0vm'),
 rust:path.join(env.RISC0_HOME,'toolchains/v1.97.0-rust-aarch64-apple-darwin/bin/rustc'),
 rzup:path.join(repo,'target/g17-tools/rzup')};
if(!binaries[kind])throw Error('expected cargo|host|sdk|r0vm|rust|rzup');
console.log(JSON.stringify({recovery_environment:{host_cargo:cargo,guest_toolchain_version:'1.97.0',sdk:'3.1.0',
 RISC0_HOME:env.RISC0_HOME,RISC0_SERVER_PATH:env.RISC0_SERVER_PATH,RISC0_EXECUTOR:env.RISC0_EXECUTOR,
 RISC0_DEV_MODE:'0',RISC0_BUILD_LOCKED:env.RISC0_BUILD_LOCKED??null,
 host_release_strip:env.CARGO_PROFILE_RELEASE_STRIP,
 proxy:'Existing macOS system proxy used explicitly for official downloads',
 global_default_or_profile_changed:false,prover_invoked:false}}));
const result=spawnSync(binaries[kind],args,{env,stdio:'inherit'});
if(result.error)throw result.error;
if(result.signal){process.kill(process.pid,result.signal)}else process.exitCode=result.status??1;
