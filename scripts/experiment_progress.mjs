// Read-only terminal viewer. Never starts, stops or reruns an experiment.
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const ROOT = fileURLToPath(new URL('../', import.meta.url));
export const PLAN = [
  ['audit-inventory', '证据清单核对'], ['audit-b1', '四路线历史报告'],
  ['audit-b2', '历史成本报告'], ['audit-connected-chain', '链上记录核对'],
  ['audit-connected-report', '完整流程报告'], ['audit-belenios', 'Belenios旧记录'],
  ['audit-g9-boundary', 'G9边界核对'], ['build', 'Release编译'],
  ['fmt', '格式检查'], ['clippy', '严格Clippy'], ['tests', '相关测试'],
  ['ballot-setup', '选票Setup'], ['ballot-prove', '8票证明生成'],
  ['state-setup', '状态Setup'], ['state-prove', '2份状态证明生成'],
  ['aggregate-receive', '聚合与四路线接收'],
];
const SCHEMA = 'VV/STAGE2/EXECUTION/v1';
const EVM_SCHEMA = 'VV/STAGE2/EVM-FOLD-EXECUTION/v1';
export const EVM_PLAN = [
 ['build-rust-reference','Rust对照编译'], ['fmt','格式检查'], ['clippy-reference','严格Clippy'],
 ['rust-reference','真实密文Rust累计'], ['compile','Solidity编译'], ['point-conformance','Rust/EVM点运算对照'],
 ['direct-fold-vm','Direct Fold接收与冻结'], ['state-proof-vm','State Proof接收与冻结'],
];
const supported = schema => [SCHEMA,EVM_SCHEMA].includes(schema);
function readJson(file) { return JSON.parse(fs.readFileSync(file, 'utf8')); }
function tryJson(file) { try { return readJson(file); } catch { return null; } }
function safeLog(run, relative) {
  const p = path.resolve(run, relative);
  return p.startsWith(path.resolve(run) + path.sep) ? p : null;
}
export function tail(file, lines = 6) {
  if (!file) return '';
  let fd;
  try {
    fd = fs.openSync(file, 'r');
    const size = fs.fstatSync(fd).size;
    const start = Math.max(0, size - 8192);
    const b = Buffer.alloc(size - start);
    fs.readSync(fd, b, 0, b.length, start);
    let text = b.toString('utf8');
    if (start) text = text.slice(text.indexOf('\n') + 1);
    return text.trimEnd().split('\n').slice(-lines).join('\n');
  } catch { return ''; }
  finally { if (fd !== undefined) fs.closeSync(fd); }
}
export function findLatest(base) {
  if (Array.isArray(base)) {
    return base.map(findLatest).filter(Boolean).sort((a,b) => b.started-a.started || b.dir.localeCompare(a.dir))[0] ?? null;
  }
  if (!fs.existsSync(base)) return null;
  const runs = [];
  for (const entry of fs.readdirSync(base, {withFileTypes: true})) {
    if (!entry.isDirectory()) continue;
    const dir = path.join(base, entry.name);
    const manifest = tryJson(path.join(dir, 'execution.json'));
    if (!supported(manifest?.schema)) continue;
    runs.push({dir, manifest, started: Date.parse(manifest.started_utc) || 0});
  }
  // The experiment start timestamp, not a copied directory's mtime, defines latest.
  runs.sort((a, b) => b.started - a.started || b.dir.localeCompare(a.dir));
  return runs[0] ?? null;
}
export function snapshot({base, run}) {
  const selected = run ? {dir: path.resolve(run), manifest: readJson(path.join(run, 'execution.json'))} : findLatest(base);
  if (!selected) return null;
  const {dir, manifest: m} = selected;
  if (!supported(m.schema) || !Array.isArray(m.commands)) throw new Error('该目录不是当前阶段二成本实验的execution.json');
  const commands = new Map(m.commands.map(c => [c.name, c]));
  const plan = m.schema === EVM_SCHEMA ? EVM_PLAN : PLAN;
  const names = [...plan.map(([name]) => name), ...m.commands.map(c => c.name).filter(n => !plan.some(([p]) => p === n))];
  const running = m.status === 'RUNNING';
  const pendingLogs = names.filter(n => !commands.has(n) && fs.existsSync(path.join(dir, 'logs', n + '.log')));
  const active = running ? m.running_command?.name ?? pendingLogs.at(-1) : null;
  const steps = names.map(name => {
    const c = commands.get(name);
    return {name, label: plan.find(([n]) => n === name)?.[1] ?? name,
      state: c ? (c.code === 0 ? 'passed' : c.signal ? 'interrupted' : 'failed') : name === active ? 'pending_receipt' : 'waiting',
      elapsed_ms: c?.elapsed_ms ?? null};
  });
  const latest = active ? 'logs/' + active + '.log' : m.commands.at(-1)?.log;
  const summary = tryJson(path.join(dir, 'cost_summary.json'));
  const direct = m.schema === EVM_SCHEMA ? tryJson(path.join(dir,'direct_fold.json')) : null;
  const state = m.schema === EVM_SCHEMA ? tryJson(path.join(dir,'state_proof.json')) : null;
  const evm = direct?.status?.startsWith('PASS') && state?.status?.startsWith('PASS') ?
    {direct_gas:direct.accepted_batch_gas,state_gas:state.accepted_batch_gas,negative_tests:direct.negatives.length+state.negatives.length} : null;
  const counts = {};
  for (const [phase, kind, total] of [['ballot-prove', 'ballot', 8], ['state-prove', 'state', 2]]) {
    const proof = tryJson(path.join(dir, phase + '.json'));
    if (Array.isArray(proof?.proofs)) counts[kind] = {done: proof.proofs.filter(p => p.verified === true).length, total};
    else {
      const text = tail(path.join(dir, 'logs', phase + '.log'), 20);
      const matches = [...text.matchAll(new RegExp('fresh ' + kind + ' (\\d+)/(\\d+)', 'g'))];
      if (matches.length) counts[kind] = {done: Number(matches.at(-1)[1]), total: Number(matches.at(-1)[2])};
    }
  }
  return {dir, status: m.status, running, steps, counts,
    passed: steps.filter(s => s.state === 'passed').length,
    failed: steps.filter(s => ['failed', 'interrupted'].includes(s.state)).length,
    started_utc: m.started_utc, finished_utc: m.finished_utc ?? null,
    latest_log: latest ?? null, log_tail: latest ? tail(safeLog(dir, latest)) : '',
    error: m.error ?? null, summary, evm};
}
export function render(view, {watch = false, interval = 2} = {}) {
  if (!view) return 'VeriVoting 实验进度（只读）\n尚未找到阶段二成本实验记录；等待新运行目录。\n';
  const lines = ['VeriVoting 实验进度（只读）', '目录：' + view.dir];
  const title = view.status.startsWith('PASS') ? '✓ 本轮通过' : view.running ? '◐ 清单记录为运行中（未核实进程存活）' : '✗ 本轮失败或中止';
  lines.push('状态：' + title + '  [' + view.status + ']');
  const filled = Math.floor(view.passed / view.steps.length * 20);
  lines.push('步骤：[' + '#'.repeat(filled) + '-'.repeat(20 - filled) + '] ' + view.passed + '/' + view.steps.length + '通过，失败/中止' + view.failed);
  const marks = {passed: '✓', failed: '✗', interrupted: '!', pending_receipt: '◐', waiting: '·'};
  for (const s of view.steps) {
    const elapsed = Number.isFinite(s.elapsed_ms) ? '  ' + (s.elapsed_ms / 1000).toFixed(2) + ' s' : '';
    const state = s.state === 'pending_receipt' ? '  日志已创建，待完成回执' : s.state === 'waiting' ? '  尚无完成记录' : s.state === 'interrupted' ? '  已中止' : '';
    lines.push('  ' + marks[s.state] + ' ' + s.name.padEnd(25) + s.label + elapsed + state);
  }
  const progress = Object.entries(view.counts).map(([k, c]) => (k === 'ballot' ? '选票证明' : '状态证明') + ' ' + c.done + '/' + c.total);
  if (progress.length) lines.push('\n已生成且验证的证明记录：' + progress.join('；'));
  if (view.summary) {
    const s = view.summary;
    lines.push('纯计算组件：选票Prove ' + (s.ballot_proving_ms / 1000).toFixed(3) + ' s；状态Prove ' + (s.state_proving_ms / 1000).toFixed(3) + ' s；聚合 ' + s.aggregation_ms.toFixed(3) + ' ms');
  }
  if (view.evm) lines.push('EVM两批接收：Direct Fold ' + view.evm.direct_gas.toLocaleString('en-US') + ' Gas；State Proof ' + view.evm.state_gas.toLocaleString('en-US') + ' Gas；必要拒绝测试 ' + view.evm.negative_tests + ' 项。', '此处仅为接收交易，不含部署、认证和链下证明成本；见完整对比报告。');
  if (view.latest_log) lines.push('\n最新步骤日志：' + view.latest_log, view.log_tail || '（暂时没有日志输出，不据此判断卡住）');
  if (view.error) lines.push('\n错误记录：' + view.error);
  lines.push('\n步骤进度不是算力百分比或剩余时间估计；步骤耗时包含加载等开销。');
  lines.push('本轮状态不代表整个项目完成；后续任务见 docs/NEXT_STEPS.md。');
  if (watch) lines.push('每' + interval + '秒自动刷新；自动跟随最新目录（--run指定时固定目录）。Ctrl+C只退出查看器。');
  return lines.join('\n') + '\n';
}
const HELP = `用法：node scripts/experiment_progress.mjs [选项]
  --watch          每2秒刷新，自动跟随最新运行目录
  --run DIR        只查看指定运行目录
  --base DIR       修改自动查找的父目录
  --interval SEC   刷新间隔，默认2秒
  --json           输出一次结构化进度
  --help           显示帮助
默认只显示一次。支持当前阶段二原生成本和EVM累计对比实验；只读、不启动实验。\n`;
export async function main(argv = process.argv.slice(2)) {
  const options = {base: [path.join(ROOT, 'experiments/stage2_matched_cost_raw'),path.join(ROOT, 'experiments/stage2_evm_fold_raw')], watch: false, interval: 2};
  let json = false;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--watch') options.watch = true;
    else if (arg === '--json') json = true;
    else if (arg === '--help' || arg === '-h') { process.stdout.write(HELP); return; }
    else if (['--run', '--base', '--interval'].includes(arg)) {
      const value = argv[++i];
      if (!value || value.startsWith('--')) throw new Error(arg + '缺少参数');
      if (arg === '--interval') options.interval = Number(value);
      else options[arg.slice(2)] = path.resolve(value);
    } else throw new Error('未知参数：' + arg);
  }
  if (!Number.isFinite(options.interval) || options.interval < 0.05 || options.interval > 60) throw new Error('刷新间隔应为0.05至60秒');
  if (json && options.watch) throw new Error('--json与--watch不能同时使用');
  let previous = '';
  const draw = () => {
    let text;
    try { const view = snapshot(options); text = json ? JSON.stringify(view, null, 2) + '\n' : render(view, options); }
    catch (e) {
      if (!options.watch) throw e;
      text = '进度文件暂时不可读（可能正在写入），稍后自动重试：' + e.message + '\n';
    }
    if (options.watch && process.stdout.isTTY) process.stdout.write('\x1b[2J\x1b[H' + text);
    else if (text !== previous) process.stdout.write(text);
    previous = text;
  };
  draw();
  if (options.watch) {
    const timer = setInterval(draw, options.interval * 1000);
    const stop = () => { clearInterval(timer); process.stdout.write('\n已退出进度查看器；没有停止或重新运行实验。\n'); };
    process.once('SIGINT', stop);
    process.once('SIGTERM', stop);
  }
}
if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(e => { console.error('查看进度失败：' + e.message); process.exitCode = 1; });
}
