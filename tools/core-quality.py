"""Runs the quality gates of the xemnas core and prints one summary.

The core is capture -> memory (graph, observations, review) -> context that
reaches the agent. Every gate measures assertiveness (precision, recall,
contamination) or performance (latency, cost) on a labelled corpus and fails
when a floor or ceiling is crossed. See docs/arquitetura/qualidade-do-nucleo.md.

    python tools/core-quality.py

Smart App Control (Windows) blocks freshly linked test binaries whose hash it
has not seen; each gate binary is rebuilt with a different codegen-units value
until one runs (the behaviour is the same, only the hash changes).
"""
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
ENV = dict(os.environ)
ENV.setdefault('CARGO_TARGET_DIR', os.path.join(ROOT, 'target'))

# (label, package, target, filter, ignored?)
GATES = [
    ('context selection', 'storage-sqlite', 'test:context_corpus', 'context_quality_gate', False),
    ('context selection, sealed v4', 'storage-sqlite', 'test:context_corpus', 'sealed_v4_quality_gate', False),
    ('context selection, calibration v5', 'storage-sqlite', 'test:context_corpus', 'calibration_v5_quality_gate', False),
    ('context report', 'storage-sqlite', 'test:context_corpus', 'report_context_corpus', True),
    ('mention links', 'application', 'lib', 'graph::mention', False),
    ('ai link proposals', 'storage-sqlite', 'test:link_corpus', 'link_quality_gate', False),
    ('automatic review', 'storage-sqlite', 'test:auto_approval', '', False),
    ('automatic triage', 'application', 'lib', 'auto_approval', False),
    ('observations latency', 'storage-sqlite', 'test:observations_evaluation', 'scoped_latency_report', True),
    ('observation query latency', 'storage-sqlite', 'test:observations_router_corpus', 'observation_query_latency', True),
]

# Gates that need a fixture generated once by a live model (never in CI):
# skipped, not failed, until the file exists. The gate test is #[ignore]d until
# then; when the fixture is committed, remove that attribute and the `True`
# of the gate above so `cargo test` runs it too.
NEEDS_FIXTURE = {
    'ai link proposals': 'crates/storage-sqlite/tests/fixtures/link_corpus_answers.json',
}

KEEP = re.compile(r'(?:^|\.\.\. )((?:gate |summary |mention |latency|refresh_|build_pack |render |observation_query ).*)')


def package_dir(name):
    meta = json.loads(subprocess.run(
        ['cargo', 'metadata', '--no-deps', '--format-version', '1'],
        cwd=ROOT, capture_output=True, text=True, env=ENV).stdout)
    return next(os.path.dirname(p['manifest_path']) for p in meta['packages'] if p['name'] == name)


def build(package, target, units):
    selector = ['--lib'] if target == 'lib' else ['--test', target.split(':', 1)[1]]
    result = subprocess.run(
        ['cargo', 'rustc', '-j4', '--locked', '-p', package, *selector, '--profile', 'test',
         '--message-format=json', '--', '-C', f'codegen-units={units}'],
        cwd=ROOT, capture_output=True, text=True, encoding='utf-8', errors='replace', env=ENV)
    for line in reversed(result.stdout.splitlines()):
        try:
            message = json.loads(line)
        except ValueError:
            continue
        if message.get('reason') == 'compiler-artifact' and message.get('executable'):
            return message['executable']
    raise SystemExit(f'build failed for {package} {target}:\n{result.stderr[-2000:]}')


def run_gate(label, package, target, test_filter, ignored):
    cwd = package_dir(package)
    for units in range(2, 40):
        exe = build(package, target, units)
        args = [exe, '--nocapture', '--test-threads=1']
        if test_filter:
            args.insert(1, test_filter)
        if ignored:
            args.append('--ignored')
        try:
            run = subprocess.run(args, cwd=cwd, capture_output=True, text=True,
                                 encoding='utf-8', errors='replace', env=ENV, timeout=1800)
        except OSError:
            continue  # blocked by Smart App Control: try another hash
        output = run.stdout + run.stderr
        if 'code: 4551' in output:
            continue
        passed = run.returncode == 0
        lines = [m.group(1) for m in map(KEEP.search, output.splitlines()) if m]
        return passed, lines, output
    return False, ['every rebuilt binary was blocked by Smart App Control'], ''


def main():
    failed = []
    for gate in GATES:
        fixture = NEEDS_FIXTURE.get(gate[0])
        if fixture and not os.path.exists(os.path.join(ROOT, fixture)):
            print(f'skip {gate[0]}')
            print(f'     fixture not generated yet: {fixture}')
            continue
        passed, lines, output = run_gate(*gate)
        print(f"{'ok  ' if passed else 'FAIL'} {gate[0]}")
        for line in lines:
            print(f'     {line}')
        if not passed:
            failed.append(gate[0])
            tail = [line for line in output.splitlines() if 'panicked' in line or 'assert' in line]
            for line in tail[:6]:
                print(f'     {line}')
    print('core quality:', 'all gates pass' if not failed else f'failed: {", ".join(failed)}')
    sys.exit(1 if failed else 0)


if __name__ == '__main__':
    main()
