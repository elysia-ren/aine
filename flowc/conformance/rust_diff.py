import os, subprocess, sys

NL = chr(10)
DIR = 'examples'
BUILD = 'build/_rd'
os.makedirs(BUILD, exist_ok=True)

AINE = './target/release/aine.exe'
# 已知非差分语料: 故意报错/需要交互/UI/网络/时钟/多文件模块
SKIP = {
    'bad.aine', 'w51_probe.aine', 'w48_min.aine', 'px_probe.aine', 'px2_probe_none.aine',
    'account_book.aine', 'todo_app.aine', 'counter.aine', 'clock.aine', 'weather.aine',
    'snake.aine', 'paint.aine', 'notepad.aine', 'calc.aine', 'chat.aine', 'ui_demo.aine',
}
# 输出含时间/随机/网络 → 字节级 diff 必然失败,只查编译
NONDET = {'bench', 'benchpush', 'bigstr', 'http', 'json', 'clock'}

def run(cmd, timeout=30):
    try:
        p = subprocess.run(cmd, capture_output=True, timeout=timeout, shell=False)
        return p.returncode, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return 124, b'', b'TIMEOUT'

results = []
names = sorted(f for f in os.listdir(DIR) if f.endswith('.aine'))
for name in names:
    if name in SKIP:
        continue
    base = name[:-5]
    path = os.path.join(DIR, name)
    rc_h, out_h, err_h = run([AINE, 'run', path])
    if rc_h != 0:
        results.append((name, 'HOST_SKIP', ''))
        continue
    rc_e, out_e, _ = run([AINE, 'rust', path])
    if rc_e != 0:
        results.append((name, 'EMIT_FAIL', ''))
        continue
    gen = os.path.join(BUILD, base + '_gen.rs')
    exe = os.path.join(BUILD, base + '_r.exe')
    with open(gen, 'w', encoding='utf-8', newline=NL) as f:
        f.write(out_e.decode('utf-8', 'replace'))
    rc_c, _, err_c = run(['rustc', '--edition', '2021', '-o', exe, gen], timeout=60)
    if rc_c != 0:
        first = err_c.decode('utf-8', 'replace').split(NL)
        msg = NL.join(first[:2])
        results.append((name, 'RUSTC_FAIL', msg))
        continue
    rc_r, out_r, _ = run([exe])
    if rc_r != 0:
        results.append((name, 'RUN_FAIL', ''))
        continue
    if out_h == out_r:
        results.append((name, 'DIFF_PASS', ''))
    else:
        results.append((name, 'DIFF_FAIL', ''))

print('== Rust 后端全量差分 ==')
np = sum(1 for r in results if r[1] == 'DIFF_PASS')
print('PASS: %d / %d (host-runnable)' % (np, len(results)))
for n, s, m in results:
    if s != 'DIFF_PASS':
        print('%-24s %s %s' % (n, s, m.replace(NL, ' | ')[:110]))
print('PASS list:', ', '.join(n for n, s, _ in results if s == 'DIFF_PASS'))
