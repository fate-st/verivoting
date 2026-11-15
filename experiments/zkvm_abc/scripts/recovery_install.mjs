// Install ONLY the two pinned official archives after exact published SHA checks.
// Uses rzup's documented/source-mapped directory layout, without global shims.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
const sha=b=>createHash('sha256').update(b).digest('hex');
const run=process.argv[2];assert.ok(run,'NEW_RUN_DIRECTORY required');
const target=path.resolve('target/g17-tools/risc0');
const specs=[
 {file:'target/g17-tools/cargo-risczero-3.1.0-verified.tgz',sha:'3b50b0393b8e4158e7129add4cfe0cbc536e994b5abc6a3a435e1e6b841c0990',bytes:51568850,dir:'extensions/v3.1.0-cargo-risczero-aarch64-apple-darwin',binaries:['cargo-risczero','r0vm']},
 {file:'target/g17-tools/rust-toolchain-1.97.0-verified.tar.gz',sha:'85070d82aebbe8e1027e75f2cd0c781f5c82c0ae7e06697218be9d8b71dec15d',bytes:442966921,dir:'toolchains/v1.97.0-rust-aarch64-apple-darwin',binaries:['bin/rustc','bin/cargo']},
];
const verified=JSON.parse(fs.readFileSync(path.join(run,'download_verified.json')));
assert.equal(verified.status,'PASS_OFFICIAL_ASSET_SHA256');
const records=[];
for(const s of specs){
 const b=fs.readFileSync(s.file);assert.equal(b.length,s.bytes);assert.equal(sha(b),s.sha);
 const dir=path.join(target,s.dir);assert.ok(!fs.existsSync(dir),'Refuse overwriting an installed component: '+dir);
 const members=execFileSync('tar',['-tzf',s.file],{encoding:'utf8',maxBuffer:8*1024*1024}).trim().split('\n');
 assert.ok(members.every(p=>!p.startsWith('/')&&!p.split('/').includes('..')));
 fs.mkdirSync(dir,{recursive:true});execFileSync('tar',['-xzf',s.file,'-C',dir]);
 records.push({archive:s.file,archive_sha256:s.sha,installed_directory:dir,
  binaries:s.binaries.map(p=>{const file=path.join(dir,p);return {path:file,sha256:sha(fs.readFileSync(file)),bytes:fs.statSync(file).size}})});
}
const record={status:'PASS_PINNED_TOOLCHAIN_EXTRACTED',method:'Verified official tar archives into rzup-compatible version directories',
 global_rust_default_changed:false,global_cargo_shims_changed:false,shell_profile_changed:false,records};
fs.writeFileSync(path.join(run,'installation.json'),JSON.stringify(record,null,2)+'\n',{flag:'wx'});
console.log(JSON.stringify(record,null,2));
