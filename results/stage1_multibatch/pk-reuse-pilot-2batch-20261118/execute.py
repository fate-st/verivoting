import datetime
import json
import os
import pathlib
import subprocess
import sys
import time

ROOT = pathlib.Path('/tmp/vv-pk-reuse-pilot')
WORKTREE = ROOT / 'worktree'
HISTORICAL = pathlib.Path('/Users/lijian/Documents/02学习/论文/latex模版/experiment/verivoting-vscode-starter/experiments/stage1_multibatch_raw/gate-b-20261117-02/materials')


def save(name, value):
    with (ROOT / name).open('x') as f:
        json.dump(value, f, indent=2)
        f.write('\n')


env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(ROOT / 'target'), CARGO_BUILD_JOBS='2',
           RAYON_NUM_THREADS='4', RUSTUP_TOOLCHAIN='1.99.1')
if sys.argv[1] == 'compile':
    args = ['cargo', '+1.99.1', 'build', '--release', '--locked', '--offline',
            '--bin', 'stage1_multibatch_pk_reuse_pilot']
    with (ROOT / 'compile.stdout.log').open('xb') as out, (ROOT / 'compile.stderr.log').open('xb') as err:
        start = time.perf_counter_ns()
        result = subprocess.run(args, cwd=WORKTREE, stdout=out, stderr=err, env=env)
        elapsed = time.perf_counter_ns() - start
    save('compile.json', {'command': args, 'cwd': str(WORKTREE), 'returncode': result.returncode,
                         'compile_wall_ns': elapsed, 'compile_wall_ms': elapsed / 1e6,
                         'environment': {k: env[k] for k in ['CARGO_TARGET_DIR', 'CARGO_BUILD_JOBS', 'RAYON_NUM_THREADS', 'RUSTUP_TOOLCHAIN']},
                         'release_opt_level': 1})
    sys.exit(result.returncode)

assert sys.argv[1] == 'run'
assert json.loads((ROOT / 'compile.json').read_text())['returncode'] == 0
profile_path = ROOT / 'reuse_pilot_profile.json'
assert not profile_path.exists(), 'do not repeat pilot'
env['VV_REUSE_PILOT_OUT_JSON'] = str(profile_path)
env['VV_REUSE_HISTORICAL_REQUEST_PATH'] = str(HISTORICAL / 'request.json')
physical_bytes = int(subprocess.check_output(['sysctl', '-n', 'hw.memsize']).decode().strip())
swap_before = subprocess.check_output(['sysctl', '-n', 'vm.swapusage']).decode().strip()
vm_before = subprocess.check_output(['vm_stat']).decode()
args = [str(ROOT / 'target/release/stage1_multibatch_pk_reuse_pilot'), 'prove-batch-range',
        'experiments/stage1_multibatch_raw/pk-reuse-pilot-2batch', 'B', '1', '2', '--approved', 'B']
save('single_execution_intent.json', {'command': args, 'started_at_unix_ns': time.time_ns(),
                                    'max_invocations': 1, 'batches': [1, 2],
                                    'historical_request_path': env['VV_REUSE_HISTORICAL_REQUEST_PATH'],
                                    'gate_c': 'NOT_AUTHORIZED_NOT_STARTED'})
outer = {'command': args, 'cwd': str(WORKTREE), 'outer_wall_semantics': 'spawn_to_observed_exit',
         'nominal_poll_interval_ms': 250, 'polling_overhead_bound_ms': None,
         'host_physical_mem_bytes': physical_bytes, 'host_physical_mem_mib': physical_bytes / 1048576,
         'swap_before': swap_before, 'vm_stat_before': vm_before,
         'swap_scope': 'host-global before/after; changes cannot be attributed exclusively to pilot'}
p = None
peak = 0
peak_ts = None
samples = 0
sample_errors = 0
phase_metrics = {}
try:
    with (ROOT / 'pilot.stdout.log').open('xb') as fout, (ROOT / 'pilot.stderr.log').open('xb') as ferr, (ROOT / 'rss_samples.jsonl').open('x') as trace:
        start = time.perf_counter_ns()
        try:
            p = subprocess.Popen(args, cwd=WORKTREE, stdout=fout, stderr=ferr, env=env)
            outer['pid'] = p.pid
            while p.poll() is None:
                try:
                    phase = 'spawn'
                    for line in (ROOT / 'pilot.stdout.log').read_text().splitlines():
                        try:
                            phase = json.loads(line).get('phase', phase)
                        except json.JSONDecodeError:
                            pass
                    rss_text = subprocess.check_output(['ps', '-o', 'rss=', '-p', str(p.pid)], stderr=subprocess.DEVNULL).decode().strip()
                    if rss_text:
                        rss_kib = int(rss_text.split()[0])
                        stamp = datetime.datetime.now(datetime.timezone.utc).isoformat().replace('+00:00', 'Z')
                        sample = {'pid': p.pid, 'timestamp_utc': stamp, 'outer_offset_ns': time.perf_counter_ns() - start,
                                  'phase': phase, 'rss_kib': rss_kib, 'rss_mib': rss_kib / 1024}
                        trace.write(json.dumps(sample) + '\n')
                        trace.flush()
                        samples += 1
                        if rss_kib > peak:
                            peak = rss_kib
                            peak_ts = stamp
                        values = phase_metrics.setdefault(phase, {'sample_count': 0, 'sampled_peak_rss_mib': 0, 'first_rss_mib': sample['rss_mib']})
                        values['sample_count'] += 1
                        values['last_rss_mib'] = sample['rss_mib']
                        values['sampled_peak_rss_mib'] = max(values['sampled_peak_rss_mib'], sample['rss_mib'])
                except (subprocess.SubprocessError, OSError, ValueError):
                    sample_errors += 1
                time.sleep(0.25)
        finally:
            if p is not None and p.poll() is None:
                p.terminate()
                try:
                    p.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    p.kill()
                    p.wait()
            elapsed = time.perf_counter_ns() - start
            outer.update(outer_wall_ns=elapsed, outer_wall_ms=elapsed / 1e6,
                         returncode=None if p is None else p.returncode,
                         termination_signal=(-p.returncode if p is not None and p.returncode is not None and p.returncode < 0 else None),
                         sampled_peak_rss_mib=peak / 1024 if peak > 0 else None,
                         sampled_peak_time_utc=peak_ts, rss_sample_count=samples,
                         rss_sampling_errors=sample_errors, per_phase_rss=phase_metrics)
    outer['swap_after'] = subprocess.check_output(['sysctl', '-n', 'vm.swapusage']).decode().strip()
    outer['vm_stat_after'] = subprocess.check_output(['vm_stat']).decode()
    if profile_path.exists():
        prof = json.loads(profile_path.read_text())
        config_ns = prof.get('process_config_ns')
        worker_ns = prof.get('worker_total_ns')
        if config_ns is not None and worker_ns is not None:
            residual = elapsed - int(config_ns) - int(worker_ns)
            if residual < 0:
                raise RuntimeError('Invalid timing hierarchy')
            outer.update(process_config_ns=config_ns, process_config_ms=config_ns / 1e6,
                         worker_total_ns=worker_ns, worker_total_ms=worker_ns / 1e6,
                         outer_residual_ns=residual, outer_residual_ms=residual / 1e6)
    assert p is not None and p.returncode == 0, f'Reuse pilot failed: {outer.get("returncode")}'
except BaseException as error:
    outer['wrapper_error'] = repr(error)
    raise
finally:
    save('outer_wall_rss.json', outer)
print(json.dumps(outer, indent=2))
