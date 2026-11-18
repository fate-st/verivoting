import json
import os
import pathlib
import subprocess
import sys
import time
import hashlib

ROOT = pathlib.Path('/tmp/vv-pk-profile')
WORKTREE = ROOT / 'worktree'


def save(name, value):
    with (ROOT / name).open('x') as f:
        json.dump(value, f, indent=2)
        f.write('\n')


env = os.environ.copy()
env.update(CARGO_TARGET_DIR=str(ROOT / 'target'), CARGO_BUILD_JOBS='2',
           RAYON_NUM_THREADS='4', RUSTUP_TOOLCHAIN='1.99.1')

if sys.argv[1] == 'compile':
    args = ['cargo', '+1.99.1', 'build', '--release', '--locked', '--offline',
            '--bin', 'stage1_multibatch_pk_profile']
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
profile_path = ROOT / 'batch1_profile.json'
assert not profile_path.exists(), 'do not repeat profiling'
env['VV_PROFILE_OUT_JSON'] = str(profile_path)
args = [str(ROOT / 'target/release/stage1_multibatch_pk_profile'), 'prove-worker',
        'experiments/stage1_multibatch_raw/pk-profile-batch1', 'B', '1', '--approved', 'B']
save('single_execution_intent.json', {'command': args, 'started_at_unix_ns': time.time_ns(),
                                    'max_invocations': 1, 'batch_index': 1, 'gate_c': 'NOT_AUTHORIZED_NOT_STARTED'})
outer = {'command': args, 'cwd': str(WORKTREE), 'outer_wall_semantics': 'spawn_to_observed_exit',
         'nominal_poll_interval_ms': 250, 'polling_overhead_bound_ms': None}
p = None
try:
    with (ROOT / 'batch1.stdout.log').open('xb') as fout, (ROOT / 'batch1.stderr.log').open('xb') as ferr:
        start = time.perf_counter_ns()
        try:
            p = subprocess.Popen(args, cwd=WORKTREE, stdout=fout, stderr=ferr, env=env)
            outer['pid'] = p.pid
            while p.poll() is None:
                time.sleep(0.25)
        finally:
            # Preserve evidence if the parent is interrupted; never retry the child.
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
                         termination_signal=(-p.returncode if p is not None and p.returncode is not None and p.returncode < 0 else None))
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
    assert p is not None and p.returncode == 0, f'Worker failed: {outer.get("returncode")}'
except BaseException as error:
    outer['wrapper_error'] = repr(error)
    raise
finally:
    save('outer_wall.json', outer)

print(json.dumps(outer, indent=2))
