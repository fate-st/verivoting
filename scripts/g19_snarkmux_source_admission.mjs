// G19-A only: archive source-access evidence; never claim cryptographic admission.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {createHash} from 'node:crypto';
import {execFileSync, spawnSync} from 'node:child_process';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const raw = 'experiments/snarkmux_evm';
const decisionPath = 'results/g19/decision.json';
const docPath = 'docs/G19_SNARKMUX_EVM_FEASIBILITY.md';
const sourceUrl = 'https://anonymous.4open.science/r/SnarkMux';
const priorCommit = '6db31eb2f7b6db706589ac1f6e8f27e2e106e1b9';
const sha = bytes => createHash('sha256').update(bytes).digest('hex');
const read = name => fs.readFileSync(path.join(root, name));
const json = name => JSON.parse(read(name));
const git = (...args) => execFileSync('git', args, {cwd: root, encoding: 'utf8'}).trim();
const write = (name, value) => {
  const target = path.join(root, name);
  fs.mkdirSync(path.dirname(target), {recursive: true});
  fs.writeFileSync(target, typeof value === 'string' ? value : `${JSON.stringify(value, null, 2)}\n`, {flag: 'wx'});
};
const pathsUnder = dir => fs.readdirSync(path.join(root, dir), {withFileTypes: true})
  .flatMap(entry => entry.isDirectory() ? pathsUnder(`${dir}/${entry.name}`) : [`${dir}/${entry.name}`]);

if (process.argv[2] === '--check') {
  const sums = read(`${raw}/SHA256SUMS`).toString().trim().split('\n');
  for (const line of sums) {
    const match = /^([a-f0-9]{64})  (.+)$/.exec(line);
    assert.ok(match, `invalid manifest line: ${line}`);
    assert.equal(sha(read(match[2])), match[1], match[2]);
  }
  const history = json(`${raw}/historical_files.json`);
  for (const entry of history.files) assert.equal(sha(read(entry.path)), entry.sha256, entry.path);
  if (history.external_paper) assert.equal(sha(fs.readFileSync(history.external_paper.path)), history.external_paper.sha256);
  const access = json(`${raw}/source_inventory.json`);
  const response = json(`${raw}/evidence/prototype_response.json`);
  assert.equal(sha(Buffer.from(response.body_base64, 'base64')), response.body_sha256);
  assert.equal(sha(Buffer.from(response.headers_base64, 'base64')), response.headers_sha256);
  assert.equal(access.http_status, response.http_status);
  assert.equal(access.source_obtained, false);
  const inventory = json(`${raw}/verifier_inventory.json`);
  assert.equal(inventory.required_relations.length, 6);
  assert.ok(inventory.required_relations.every(r => r.implementation === null && r.test_result === null));
  const results = json(`${raw}/results.json`), decision = json(decisionPath);
  assert.equal(results.stage_status['G19-A'], 'BLOCKED_SOURCE');
  assert.ok(Object.values(results.measurements).every(value => value === null));
  assert.equal(results.proofs_generated, 0);
  assert.equal(results.evm_transactions, 0);
  assert.ok(results.tests.every(test => test.result === null));
  assert.equal(decision.decision, 'INCONCLUSIVE');
  assert.equal(decision.advance_to_g19_b, false);
  console.log(JSON.stringify({archive_check: 'PASS', artifact_hashes_checked: sums.length,
    historical_files_unchanged: history.files.length, six_relations_untested: true,
    cryptographic_or_evm_tests_run: false}, null, 2));
  process.exit(0);
}
assert.equal(process.argv.length, 2, 'usage: node scripts/g19_snarkmux_source_admission.mjs [--check]');
assert.equal(git('branch', '--show-current'), 'experiment/g19-snarkmux-evm-feasibility');
assert.equal(git('rev-parse', 'HEAD'), git('rev-parse', 'experiment/g18-final-architecture-decision'));
assert.equal(git('diff', '--name-only', 'HEAD'), '', 'tracked changes must be reviewed first');
for (const target of [raw, decisionPath, docPath]) assert.ok(!fs.existsSync(path.join(root, target)), `refuse to overwrite ${target}`);

const started = new Date().toISOString();
const tracked = execFileSync('git', ['ls-files', '-z'], {cwd: root, encoding: 'utf8'})
  .split('\0').filter(Boolean).sort().map(name => ({path: name, sha256: sha(read(name))}));
const paperPath = path.resolve(root, '../../sn-article-template/sn-article.tex');
const paper = fs.existsSync(paperPath) ? {path: paperPath, sha256: sha(fs.readFileSync(paperPath))} : null;
const baseCommit = git('rev-parse', 'HEAD');
const priorPaperPath = 'experiments/snarkmux_pilot/evidence/paper_source.json';
const priorPaperBytes = execFileSync('git', ['show', `${priorCommit}:${priorPaperPath}`], {cwd: root});
const priorPaper = JSON.parse(priorPaperBytes);
assert.equal(priorPaper.prototype_url, sourceUrl);
write(`${raw}/criteria.json`, {
  frozen_before_source_request_utc: started, minimum_original_proof_count: 2,
  reported_target_curve: 'BN254', reported_recursion_cycle: 'BN254/Grumpkin',
  actual_source_parameters_must_be_confirmed: true, single_transaction_preferred: true,
  full_verification_gas_ceiling: 3000000, minimum_expected_same_condition_saving_fraction: 0.20,
  comparison_requires_same_bn254_circuit_vk_and_public_inputs: true,
  source_unavailable_action: 'STOP_G19_A', unmeasured_values: null,
  no_automatic_commit_push_or_alternate_protocol: true,
});
write(`${raw}/historical_files.json`, {base_commit: baseCommit, files: tracked, external_paper: paper});
write(`${raw}/evidence/prior_paper_source.json`, priorPaperBytes.toString());

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'g19-http-'));
const args = ['--location', '--max-redirs', '5', '--proto', '=https', '--proto-redir', '=https',
  '--connect-timeout', '10', '--max-time', '45', '--silent', '--show-error',
  '--dump-header', path.join(tmp, 'headers'), '--output', path.join(tmp, 'body'),
  '--write-out', '%{json}', sourceUrl];
console.log('G19-A: checking author-linked prototype (one bounded HTTPS request)');
const began = performance.now();
const request = spawnSync('curl', args, {encoding: 'utf8', timeout: 50000, maxBuffer: 8 * 1024 * 1024});
let metadata = {};
try { metadata = JSON.parse(request.stdout || '{}'); } catch { /* raw output retained */ }
const headers = fs.existsSync(path.join(tmp, 'headers')) ? fs.readFileSync(path.join(tmp, 'headers')) : Buffer.alloc(0);
const body = fs.existsSync(path.join(tmp, 'body')) ? fs.readFileSync(path.join(tmp, 'body')) : Buffer.alloc(0);
const status = metadata.http_code || metadata.response_code || null;
const access = {
  requested_url: sourceUrl, effective_url: metadata.url_effective ?? null, http_status: status,
  curl_exit_code: request.status, curl_signal: request.signal, request_wall_ms: performance.now() - began,
  headers_base64: headers.toString('base64'), headers_sha256: sha(headers),
  body_base64: body.toString('base64'), body_sha256: sha(body), body_bytes: body.length,
  content_type: metadata.content_type ?? null, curl_error: request.error?.message ?? null,
  note: 'Raw bytes are base64-encoded to avoid Git line-ending changes. No credentials or access bypass used.',
};
write(`${raw}/evidence/prototype_response.json`, access);
write(`${raw}/logs/source-access.stdout.log`, request.stdout ?? '');
write(`${raw}/logs/source-access.stderr.log`, request.stderr ?? '');
const source = {
  status: 'BLOCKED_SOURCE', checked_at_utc: new Date().toISOString(),
  prototype_url: sourceUrl, effective_url: access.effective_url, http_status: status,
  source_obtained: false, source_commit: null, license: null, dependencies: null,
  final_verifier_entry: null, groth16_folding_entry: null, fold_r1cs_entry: null,
  spartan_variant: null, polynomial_commitment_scheme: null, serialization_entry: null,
  authentic_final_proof_available: false, prototype_build_attempted: false,
  reason: status === 200 ? 'HTTP response alone is not pinned source; manual source retrieval/review required.'
    : 'Author-linked source could not be retrieved in this environment; see raw HTTP evidence.',
  source_link_provenance: {prior_commit: priorCommit, path: priorPaperPath,
    sha256: sha(priorPaperBytes), paper_url: priorPaper.url, paper_pdf_sha256: priorPaper.sha256,
    section: priorPaper.section, newly_reaudited_pdf: false, official_mirror_byte_equality_confirmed: false},
};
write(`${raw}/source_inventory.json`, source);

const relations = [
  ['R1', 'Instance binding', 'h = h_n', 'Bind the intended ordered original instances.'],
  ['R2', 'Recursive-state binding', 'u_C,n.x = Hash(vk,n,h_n,u*,u*_C,n)', 'Bind the recursive state and final target instance.'],
  ['R3', 'Final R1CS folding', "u\u2032 = FoldR1CS.V(vk_FC,u_C,n,u*_C,n)", 'Compute the exact compressed-proof instance.'],
  ['R4', 'Target SNARK', 'V_tgt(vk_FS,u*,pi*) = 1', 'Verify the final folded target relation.'],
  ['R5', 'Final Spartan proof', "SNARK\u2032.V(vk_SNARK\u2032,u\u2032,pi\u2032) = 1", 'Verify the complete original final proof.'],
  ['R6', 'Non-relaxed instance', 'u_C,n is a valid non-relaxed instance', 'Enforce original encoding and non-relaxed conditions.'],
].map(([id, component, required_relation, purpose]) => ({id, component, required_relation, purpose,
  requirement_source: 'user G19 plan, Figure 2 checklist; not implementation-confirmed',
  implementation: null, source_location: null, test_result: null, status: 'NOT_REVIEWED_SOURCE_BLOCKED'}));
const eip196 = 'https://eips.ethereum.org/EIPS/eip-196';
const eip197 = 'https://eips.ethereum.org/EIPS/eip-197';
const operations = [
  ['BN254 G1 add / scalar multiplication', 'EIP-196 0x06 / 0x07; MSM can compose these operations.', [eip196]],
  ['BN254 G2 add / scalar multiplication / MSM', 'No dedicated operation in EIP-196/197; a matching implementation would need review.', [eip196, eip197]],
  ['BN254 pairing product check', 'EIP-197 0x08 returns a boolean pairing-product check, not a raw GT value.', [eip197]],
  ['GT / Fp12 arithmetic', 'A pairing-product check does not expose raw GT arithmetic; actual need is unknown.', [eip197]],
  ['Grumpkin arithmetic', 'Not supplied by the BN254 operations defined in EIP-196/197; actual need is unknown.', [eip196, eip197]],
  ['Poseidon', 'Original variant, encoding and reusable Solidity implementation must be source-confirmed.', []],
  ['IPA / polynomial commitment opening', 'Do not infer the commitment scheme from the Spartan name.', []],
].map(([operation, general_evm_support, references]) => ({operation, general_evm_support, references,
  original_implementation_uses_operation: null, actual_variant: null, reusable_matching_module: null,
  replacement_plan: null, complexity: null, measured_gas: null}));
write(`${raw}/verifier_inventory.json`, {status: 'INCOMPLETE_SOURCE_BLOCKED', required_relations: relations,
  evm_operations: operations, caveat: 'EIP capability notes are not source-audit findings, execution tests or Prague Gas estimates.'});

const envRead = (program, argv) => {
  const r = spawnSync(program, argv, {encoding: 'utf8', timeout: 10000});
  return {program, argv, exit_code: r.status, stdout: r.stdout?.trim() ?? '', stderr: r.stderr?.trim() ?? ''};
};
const config = json('configs/experiment.G8C-FROZEN-P3-W32.json');
const environment = {node: process.version, platform: os.platform(), release: os.release(), arch: os.arch(),
  cpu_model: os.cpus()[0]?.model ?? null, cpu_count: os.cpus().length, total_memory_bytes: os.totalmem(),
  macos: envRead('sw_vers', []), evm_executed: false,
  existing_evm_configuration_only: {hardfork: 'Prague', source: 'contracts/scripts/g8c_frozen_p3.mjs',
    source_sha256: sha(read('contracts/scripts/g8c_frozen_p3.mjs')), optimizer_runs: 200, viaIR: true,
    transaction_gas_limit: config.transaction_gas_limit, block_gas_limit: config.block_gas_limit,
    dependency_declarations: json('contracts/package.json').dependencies,
    caveat: 'Existing archived driver configuration, not a newly instantiated G19 VM or build.'}};
write(`${raw}/environment.json`, environment);
const tests = ['native_valid', 'native_wrong_proof', 'native_wrong_input', 'native_wrong_binding',
  'evm_valid', 'evm_wrong_proof', 'evm_wrong_input', 'evm_wrong_binding', 'evm_wrong_vk']
  .map(name => ({name, status: 'NOT_RUN_SOURCE_BLOCKED', result: null}));
write(`${raw}/results.json`, {
  gate: 'G19', status: 'BLOCKED_SOURCE', stage_status: {'G19-A': 'BLOCKED_SOURCE',
    'G19-B': 'NOT_STARTED', 'G19-C': 'NOT_STARTED', 'G19-D': 'NOT_EVALUATED', 'G19-E': 'NOT_STARTED'},
  intended_original_instance_count: 2, actual_original_instance_count: 0,
  proofs_generated: 0, evm_transactions: 0, solidity_created: false, tests,
  measurements: {deploy_gas: null, full_verify_gas: null, binding_gas: null, calldata_bytes: null,
    creation_bytecode_bytes: null, runtime_bytecode_bytes: null, native_verification_ms: null,
    proof_generation_ms: null, prover_peak_rss_bytes: null, bn254_individual_baseline_gas: null,
    saving_fraction: null},
  no_mock_no_host_success_no_old_bls_baseline: true,
});
const unchanged = tracked.every(entry => sha(read(entry.path)) === entry.sha256);
assert.equal(unchanged, true);
if (paper) assert.equal(sha(fs.readFileSync(paper.path)), paper.sha256);
write(decisionPath, {
  experiment: 'G19-SnarkMux-EVM-Feasibility', decision: 'INCONCLUSIVE',
  detail: 'INCONCLUSIVE_SOURCE_OR_ENVIRONMENT', stopped_at: 'G19-A', gate_status: 'BLOCKED_SOURCE',
  source_inventory: `${raw}/source_inventory.json`, raw_response: `${raw}/evidence/prototype_response.json`,
  source_http_status: status, original_verifier_implementation_review_complete: false,
  actual_deployment: false, cryptographic_positive_or_negative_results: null, gas_evidence: null,
  same_condition_comparison: null, historical_files_unchanged: unchanged,
  advance_to_g19_b: false, approve_g19_e_or_g20: false,
  reason: 'The author-linked original source is unavailable/unpinned; there is no authentic final proof or complete source audit.',
  limitations: ['Not evidence of mathematical or engineering impossibility on EVM.',
    'No original Spartan/PCS/Grumpkin implementation was inspected.',
    'No cost or performance superiority conclusion is supported.'],
  next_required_evidence: 'An accessible author-linked source archive/repository, pinned revision, license and matching final-proof vector.',
});

const rows = relations.map(r => `| ${r.id} ${r.component} | \`${r.required_relation}\` | 未取得源码，未审查/未执行 |`).join('\n');
const report = `# G19：SnarkMux 最终验证器 EVM 部署可行性\n\n` +
  `结论：**INCONCLUSIVE**；G19-A 状态：**BLOCKED_SOURCE**。\n\n` +
  `本次对作者原型地址进行了一次真实 HTTPS 访问，HTTP 状态为 ${status ?? 'N/A'}，curl 退出码 ${request.status ?? 'N/A'}。\n` +
  `有效地址：${access.effective_url ?? 'N/A'}。原始响应头、响应体（base64）和 curl 日志均已保存；不是重用历史 HTTP 结果。\n\n` +
  `没有取得原型源码，无法锁定 commit、许可证、Spartan 变体、承诺方案或真实最终证明。按方案 4.1 停止后续部署测试。\n` +
  `HTTP 阻塞只描述本环境本次访问，不能写成“SnarkMux 不支持 EVM”，也不构成 NO_GO_WITHIN_BUDGET。\n\n` +
  `## 已完成与未完成\n\n` +
  `- 基于 G18 提交 \`${baseCommit}\` 建立独立 G19 分支。\n` +
  `- 新保存来源清单、六项关系清单、通用 EVM 运算支持说明及本次访问证据。\n` +
  `- 检查 ${tracked.length} 个原有 Git 跟踪文件和外部论文正文 SHA-256，均未改动。\n` +
  `- 未构建原型、未生成证明、未写 Solidity、未发送 EVM 交易；所有密码学正反例均未运行，Gas/大小/时间指标为 null。\n` +
  `- 不使用历史 BLS12-381 证明充当本轮 BN254 测试，不复跑 G10/G17/G18，不开始 MicroSpartan 适配。\n\n` +
  `## 完整关系清单（要求，不是源码审计结论）\n\n| 必需关系 | 检查式 | 本轮状态 |\n|---|---|---|\n${rows}\n\n` +
  `## EVM 支持边界\n\n` +
  `[EIP-196](${eip196}) 提供 BN254 G1 加法与标量乘法；[EIP-197](${eip197}) 提供配对乘积布尔检查，不暴露原始 GT 运算结果。\n` +
  `这只能用于通用依赖矩阵，不能证明原始 SnarkMux 的全部关系已经可部署。原型实际是否需要 G2/GT/Grumpkin/MSM/IPA 等及其规模均未知。\n` +
  `不能仅凭 Spartan 名称推定其 PCS。未给出未经源码支持的替代实现、复杂度或 Gas 估算。\n\n` +
  `## 固定研究门槛与来源\n\n` +
  `criteria.json 在本次源站请求前写入：原型最小 2 份证明、优先单交易、完整验证 ≤3,000,000 Gas、同条件 BN254 基线期望节省 ≥20%。本轮未评估是否满足。\n` +
  `环境见 environment.json；Prague/optimizer/viaIR/20M 交易和 30M 区块限额来自既有配置，不表示本轮运行了 VM。\n` +
  `原型 URL 的论文依据取自先前提交 \`${priorCommit}\` 的归档（§6.2，印刷页26），本次没有重新审阅 PDF。该归档记录的是镜像 PDF，未确认与官方 PDF 逐字节相同。\n\n` +
  `## 复现与核对命令\n\n` +
  `\`node --check scripts/g19_snarkmux_source_admission.mjs\`\n\n` +
  `\`node scripts/g19_snarkmux_source_admission.mjs\`：本次真实执行入口；输出存在时拒绝覆盖。\n\n` +
  `\`node scripts/g19_snarkmux_source_admission.mjs --check\`：只读检查归档 SHA-256、旧文件及状态一致性；不会重新请求或运行证明。\n\n` +
  `## 继续所需证据\n\n` +
  `需要作者关联的可访问源码归档或仓库、可固定版本、许可证及匹配的最小最终证明向量。取得后另建新运行记录，先审查六项关系，再决定 G19-B；不要覆盖本次失败记录。\n` +
  `本轮到此停止；未自动提交或推送。\n`;
write(`${raw}/report.md`, report);
write(docPath, '# G19 SnarkMux EVM 可行性审查\n\n' +
  `当前结论：INCONCLUSIVE / BLOCKED_SOURCE，作者原型本次 HTTP ${status ?? 'N/A'}。\n\n` +
  'G19-A 未通过，未开始 Solidity、证明生成、EVM 测试或替代协议适配。\n\n' +
  '[完整报告](../experiments/snarkmux_evm/report.md) · [来源清单](../experiments/snarkmux_evm/source_inventory.json) · ' +
  '[关系与依赖矩阵](../experiments/snarkmux_evm/verifier_inventory.json) · [最终决策](../results/g19/decision.json)\n\n' +
  '只读验收：`node scripts/g19_snarkmux_source_admission.mjs --check`。\n\n' +
  '源码不可获取不等于 EVM 部署不可行。取得原作者源码及匹配证明后才能继续。\n');
write(`${raw}/execution.json`, {started_at_utc: started, finished_at_utc: new Date().toISOString(),
  base_commit: baseCommit, branch: git('branch', '--show-current'),
  command: 'node scripts/g19_snarkmux_source_admission.mjs',
  script_path: 'scripts/g19_snarkmux_source_admission.mjs',
  script_sha256: sha(read('scripts/g19_snarkmux_source_admission.mjs')),
  curl: {program: 'curl', argv: args, exit_code: request.status, signal: request.signal},
  historical_tracked_files_checked: tracked.length, historical_files_unchanged: unchanged,
  cryptographic_test_count: 0, evm_transaction_count: 0, automatically_committed: false, automatically_pushed: false});
const artifacts = [...pathsUnder(raw), decisionPath, docPath, 'scripts/g19_snarkmux_source_admission.mjs'].sort();
write(`${raw}/SHA256SUMS`, `${artifacts.map(name => `${sha(read(name))}  ${name}`).join('\n')}\n`);
console.log(JSON.stringify({status: 'BLOCKED_SOURCE', http_status: status, decision: 'INCONCLUSIVE',
  historical_files_unchanged: unchanged, report: `${raw}/report.md`, next_stages_started: false}, null, 2));
