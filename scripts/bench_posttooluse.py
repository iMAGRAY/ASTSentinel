#!/usr/bin/env python3
import json, os, time, subprocess, tempfile, shutil, re, sys
from pathlib import Path

BIN = Path('target/release/posttooluse')

CASES = [
  {
    'name': 'py_hardcoded_password',
    'file': 'app.py',
    'before': "print('start')\n",
    'after':  "print('start')\npassword='x'\n",
  },
  {
    'name': 'js_api_contract_break',
    'file': 'api.js',
    'before': "function greet(name){ return 'hi '+name }\n",
    'after':  "function greet(){ return 'hi' }\n",
  },
  {
    'name': 'py_command_injection',
    'file': 'run.py',
    'before': "def run(cmd):\n    return True\n",
    'after':  "def run(cmd):\n    import os\n    os.system('bash -c '+cmd)\n    return True\n",
  },
  {
    'name': 'ts_destructure_param_drop',
    'file': 'api.ts',
    'before': "function makeUser({name, age}:{name:string; age:number}){ return name+' '+age }\n",
    'after':  "function makeUser({name}:{name:string}){ return name }\n",
  },
  {
    'name': 'py_subprocess_shell',
    'file': 'run2.py',
    'before': "import subprocess\nsubprocess.run(cmd)\n",
    'after':  "import subprocess\nsubprocess.run(cmd, shell=True)\n",
  },
  {
    'name': 'js_arrow_default_drop',
    'file': 'greet.js',
    'before': "const greet=(name='u')=>`hi ${name}`\n",
    'after':  "const greet=()=>`hi`\n",
  },
]

def build_input(edit_file: Path, before: str, after: str, cwd: Path) -> str:
    payload = {
        'tool_name': 'Edit',
        'tool_input': {
            'file_path': str(edit_file),
            'old_string': before,
            'new_string': after,
        },
        'cwd': str(cwd),
        'hook_event_name': 'PostToolUse',
    }
    return json.dumps(payload, ensure_ascii=False)

def run_case(env_file: Path, case: dict) -> dict:
    tmp = Path(tempfile.mkdtemp(prefix='posttool-bench-'))
    try:
        (tmp / case['file']).write_text(case['before'], encoding='utf-8')
        input_json = build_input(tmp / case['file'], case['before'], case['after'], tmp)
        t0 = time.perf_counter()
        p = subprocess.run([str(BIN)], input=input_json.encode('utf-8'), stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        dt = time.perf_counter() - t0
        out_text = p.stdout.decode('utf-8','ignore')
        err_text = p.stderr.decode('utf-8','ignore')
        add_ctx = ''
        ok = False
        try:
            js = json.loads(out_text)
            add_ctx = js.get('hookSpecificOutput',{}).get('additionalContext','')
            ok = True
        except Exception:
            pass
        agent_json = False
        agent_block = ''
        m = re.search(r'AGENT_JSON_START\n(.*?)\nAGENT_JSON_END', add_ctx, re.S)
        if m:
            agent_json = True
            agent_block = m.group(1)
        metrics = {
            'name': case['name'],
            'ok': ok,
            'exit_code': p.returncode,
            'duration_ms': int(dt*1000),
            'add_ctx_len': len(add_ctx),
            'agent_json': agent_json,
            'risk_report': ('=== RISK REPORT ===' in add_ctx),
            'next_steps': ('=== NEXT STEPS ===' in add_ctx),
            'detected_password': ('password' in add_ctx.lower() or 'хардкод' in add_ctx.lower()),
            'stderr_head': err_text.splitlines()[:4],
            'agent_head': agent_block[:400],
        }
        return metrics
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

def write_env(dir_path: Path, provider: str, model: str, key_env: str, key_value: str):
    env_path = dir_path / '.env'
    env = []
    env.append(f'POSTTOOL_PROVIDER={provider}')
    env.append(f'POSTTOOL_MODEL={model}')
    if key_env and key_value:
        env.append(f'{key_env}={key_value}')
    env_path.write_text('\n'.join(env)+"\n", encoding='utf-8')
    return env_path

def main():
    if not BIN.exists():
        print('Build release first: cargo build --release', file=sys.stderr)
        sys.exit(1)
    tests = []
    # XAI
    xai_key = os.environ.get('XAI_BENCH_KEY','')
    if xai_key:
        env_path = write_env(BIN.parent, 'xai', 'grok-code-fast-1', 'XAI_API_KEY', xai_key)
        for c in CASES:
            tests.append(('xai', run_case(env_path, c)))
        try:
            env_path.unlink()
        except: pass
    # OpenAI
    oai_key = os.environ.get('OPENAI_BENCH_KEY','')
    if oai_key:
        env_path = write_env(BIN.parent, 'openai', 'gpt-5-nano', 'OPENAI_API_KEY', oai_key)
        for c in CASES:
            tests.append(('openai', run_case(env_path, c)))
        try:
            env_path.unlink()
        except: pass
    # Print summary
    print('provider,name,ok,exit_code,duration_ms,add_ctx_len,agent_json,risk_report,next_steps,detected_password')
    for prov, m in tests:
        print(f"{prov},{m['name']},{m['ok']},{m['exit_code']},{m['duration_ms']},{m['add_ctx_len']},{m['agent_json']},{m['risk_report']},{m['next_steps']},{m['detected_password']}")
    # Agent JSON heads (short)
    for prov, m in tests:
        if m['agent_json']:
            print(f"\n[{prov}:{m['name']}] agent_json_head:\n{m['agent_head']}")

if __name__ == '__main__':
    main()
