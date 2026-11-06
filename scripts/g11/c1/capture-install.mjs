// Copy only public build provenance. No OPAM *.env dumps, credentials or secret keys.
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {ROOT, read, write, sha, sourceProvenance} from './run.mjs';
const [cacheArg, outArg] = process.argv.slice(2);
assert.ok(cacheArg && outArg, 'capture-install.mjs /CACHE /NEW/OUT');
const cache = path.resolve(cacheArg), out = path.resolve(outArg);
fs.mkdirSync(out, {recursive: true});
const source = path.join(cache, 'belenios-git'), release = path.join(cache, 'belenios-3.0');
const config = read(path.join(ROOT, 'configs/g11/c1-belenios.json'));
assert.equal(sha(fs.readFileSync(path.join(cache, 'belenios-3.0.tar.gz'))), config.release_sha256);
const files = execFileSync('git', ['-C', source, 'ls-files', '-z'], {encoding: 'utf8'}).split('\0').filter(Boolean);
const content = p => fs.lstatSync(p).isSymbolicLink() ? Buffer.from(fs.readlinkSync(p)) : fs.readFileSync(p);
for (const p of files) assert.ok(content(path.join(source, p)).equals(content(path.join(release, p))), p);
const logs = fs.readdirSync(cache).filter(p => /^(opam.*|.*-(configure|build|install)|belenios-build.*|official-threshold-demo\.(stdout|stderr))\.log$/.test(p));
for (const p of [...logs, 'dependencies-final.txt', 'dependencies-final.export'])
  fs.copyFileSync(path.join(cache, p), path.join(out, p), fs.constants.COPYFILE_EXCL);
fs.copyFileSync(path.join(source, 'COPYING'), path.join(out, 'COPYING.belenios.txt'), fs.constants.COPYFILE_EXCL);
const demo = fs.readFileSync(path.join(cache, 'official-threshold-demo.stdout.log'), 'utf8');
assert.match(demo, /The simulated election was successful!/);
const uuid = demo.match(/UUID of the election is (\w+)/)[1];
fs.copyFileSync(path.join(source, 'tests/tool/data', uuid, `${uuid}.bel`), path.join(out, 'official-demo.bel'), fs.constants.COPYFILE_EXCL);
const version = (bin, args = []) => execFileSync(bin, args, {encoding: 'utf8'}).trim();
const opam = path.join(cache, 'opam');
write(path.join(out, 'provenance.json'), {
  captured_at: new Date().toISOString(),
  ...sourceProvenance(source, path.join(source, '_build/install/default/bin/belenios-tool'), config),
  release_url: config.release_url, release_sha256: config.release_sha256,
  release_vs_git_identical_files: files.length,
  source_tree_policy: 'All tracked Belenios bytes unchanged; symlinks compared by link text',
  dependencies: {opam: version(opam, ['--version']), ocaml: version(path.join(cache, 'opam-root/4.14.2/bin/ocamlc'), ['-version']),
    dune: version(path.join(cache, 'opam-root/4.14.2/bin/dune'), ['--version']),
    opam_repository: '4093222dece7c1c8f93dc2fd8dd7eec15fa80fed', native: {gmp: '6.3.0', libsodium: '1.0.20', pkgconf: '2.3.0', zlib: '1.3.1'},
    base: {version: 'v0.16.3', adaptation: 'CPU flag probe restricted to x86 targets; not a cryptographic change',
      patch_sha256: sha(fs.readFileSync(path.join(ROOT, 'scripts/g11/c1/base-arm64-clang17.patch')))},
    upstream_bootstrap_difference: 'Belenios pins OCaml 4.14.2 and opam repo date 20240913; same here. Package manager binary 2.6.1 replaces bootstrap opam 2.2.1.',
    versions_file: 'dependencies-final.txt', opam_export: 'dependencies-final.export'},
  compiler: version('cc', ['--version']), macos: version('sw_vers'),
  official_demo: {script: 'tests/tool/demo-threshold.sh', modified: false, status: 'passed',
    command: ['bash', 'tests/tool/demo-threshold.sh'], env: {BELENIOS: source},
    binary_path_alias: '_run/tool-debug/bin/belenios-tool is a symlink to the release CLI; no debug binary used',
    configuration: 'Upstream unmodified 5 voters, 2-of-3 Pedersen plus mandatory server; includes revotes/diff audit', uuid,
    public_archive: 'official-demo.bel', private_material_copied: false},
  setup_issues_resolved: ['raw.githubusercontent.com unreachable: official opam checksum-addressed cache used',
    'nested macOS sandbox-exec was not allowed: dependency builds run with execution approval',
    'pkg-config and zlib missing: built isolated official native dependencies',
    'base -mpopcnt falsely detected on arm64: public compiler-probe-only patch',
    'minimal upstream package declarations omit hex/cohttp-lwt-unix: installed actual source dependencies',
    'one initial opam exec argument-order error corrected; unsuccessful build logs retained'],
  recipe: 'scripts/g11/c1/install-macos-arm64.sh',
  recipe_validation: 'bash syntax checked; equivalent installation steps executed in isolated cache; fresh full recipe was not rerun a second time',
  artifact_sha256: Object.fromEntries(['opam', 'belenios-3.0.tar.gz', 'opam-repository-20240913.tar.gz', 'gmp-6.3.0.tar.xz', 'libsodium-1.0.20.tar.gz',
    'pkgconf-2.3.0.tar.xz', 'zlib-1.3.1.tar.gz'].map(p => [p, sha(fs.readFileSync(path.join(cache, p)))])),
  no_global_package_install: true, private_material_exported: false,
});
console.log(JSON.stringify({status: 'PASS_INSTALL_AND_OFFICIAL_DEMO_PROVENANCE', out, source_files: files.length, logs: logs.length}));
