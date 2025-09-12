from pathlib import Path

def main():
    p = Path('src/bin/pretooluse.rs')
    s = p.read_text(encoding='utf-8')
    old_marker = 'Calls to `{fname}` still pass removed named params'
    if old_marker in s:
        idx = s.index(old_marker)
        # find start of line
        start = s.rfind('\n', 0, idx) + 1
        # find end at the next occurrence of '));' after idx
        end = s.find('));', idx)
        if end != -1:
            end = s.find('\n', end) + 1
        else:
            end = idx
        new_block = (
            '            issues.push(format!(\n'
            '                "Calls to `{}` still pass removed named params: {}",\n'
            '                fname,\n'
            '                seen_named.join(", ")\n'
            '            ));\n'
        )
        new_s = s[:start] + new_block + s[end:]
        backup = p.with_suffix(p.suffix + '.bak_pretooluse')
        backup.write_text(s, encoding='utf-8')
        p.write_text(new_s, encoding='utf-8')
        print('Replaced block in', p)
    else:
        print('Marker not found')

if __name__ == '__main__':
    main()

