#!/usr/bin/env python3
"""
Fix simple format! macro usages: replace positional placeholders with named placeholders
Examples:
  format!("{}", var) -> format!("{var}")
  format!("{} {}", a, b) -> format!("{a} {b}")
  format!("{:4}", n) -> format!("{n:4}")

This script only transforms calls where all format arguments are simple identifiers
and the format string is a single double-quoted literal (no raw strings). It is
conservative and creates backups with suffix .bak_clippy.
"""
import re
from pathlib import Path


IDENT_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")


def is_simple_ident(s: str) -> bool:
    return bool(IDENT_RE.match(s.strip()))


def process_file(path: Path) -> bool:
    src = path.read_text(encoding='utf-8')
    i = 0
    n = len(src)
    out = []
    changed = False
    while i < n:
        idx = src.find('format!', i)
        if idx == -1:
            out.append(src[i:])
            break
        out.append(src[i:idx])
        j = idx + len('format!')
        # skip whitespace
        while j < n and src[j].isspace():
            j += 1
        if j >= n or src[j] != '(':
            out.append('format!')
            i = j
            continue
        # parse macro content
        pos = j + 1
        depth = 1
        in_str = False
        escaped = False
        mac = []
        while pos < n and depth > 0:
            ch = src[pos]
            mac.append(ch)
            if in_str:
                if not escaped and ch == '"':
                    in_str = False
                escaped = (not escaped) and (ch == '\\')
            else:
                if ch == '"':
                    in_str = True
                    escaped = False
                elif ch == '(':
                    depth += 1
                elif ch == ')':
                    depth -= 1
            pos += 1
        if depth != 0:
            # unmatched
            out.append(src[idx:pos])
            i = pos
            continue
        mac_content = ''.join(mac[:-1]) if mac and mac[-1] == ')' else ''.join(mac)
        # mac_content contains inside of parentheses excluding final )
        # find first double-quoted string literal
        m = re.match(r"\s*\"", mac_content)
        if not m:
            out.append(src[idx:pos])
            i = pos
            continue
        # parse string literal safely
        s_pos = mac_content.find('"')
        p = s_pos + 1
        str_chars = []
        esc = False
        while p < len(mac_content):
            ch = mac_content[p]
            if not esc and ch == '"':
                break
            str_chars.append(ch)
            if not esc and ch == '\\':
                esc = True
            else:
                esc = False
            p += 1
        if p >= len(mac_content):
            out.append(src[idx:pos])
            i = pos
            continue
        fmt_str = ''.join(str_chars)
        after = mac_content[p+1:]
        # parse args from after if starts with comma
        args = []
        after_strip = after.lstrip()
        if after_strip.startswith(','):
            args_list = after_strip[1:]
            # split by commas at top level
            cur = ''
            lev = 0
            for ch in args_list:
                if ch == '(':
                    lev += 1
                elif ch == ')':
                    if lev > 0:
                        lev -= 1
                if ch == ',' and lev == 0:
                    args.append(cur.strip())
                    cur = ''
                else:
                    cur += ch
            if cur.strip():
                # remove trailing parens from last
                args.append(cur.strip().rstrip(') ').strip())
        # check all args are simple idents
        if len(args) == 0 or not all(is_simple_ident(a) for a in args):
            out.append(src[idx:pos])
            i = pos
            continue
        # replace positional placeholders
        new_fmt = []
        q = 0
        arg_i = 0
        while q < len(fmt_str):
            ch = fmt_str[q]
            if ch == '{':
                # skip escaped '{{'
                if q+1 < len(fmt_str) and fmt_str[q+1] == '{':
                    new_fmt.append('{{')
                    q += 2
                    continue
                # find closing }
                r = q+1
                while r < len(fmt_str) and fmt_str[r] != '}':
                    r += 1
                if r >= len(fmt_str):
                    new_fmt.append(ch); q += 1; continue
                inner = fmt_str[q+1:r]
                if inner == '' or inner.startswith(':'):
                    if arg_i >= len(args):
                        # not enough args
                        new_fmt.append(fmt_str[q:r+1])
                    else:
                        name = args[arg_i]
                        arg_i += 1
                        if inner == '':
                            new_fmt.append('{' + name + '}')
                        else:
                            new_fmt.append('{' + name + inner + '}')
                else:
                    new_fmt.append('{' + inner + '}')
                q = r+1
            else:
                new_fmt.append(ch); q += 1
        # reconstruct macro: remove the consumed args from `after`
        remaining = after
        if args:
            # locate start of args in `after` (first comma) and end (after last arg occurrence)
            comma_pos = after.find(',')
            if comma_pos != -1:
                last_arg = args[-1].strip()
                idx_last = after.find(last_arg, comma_pos)
                if idx_last != -1:
                    pos_after_last = idx_last + len(last_arg)
                    remaining = after[pos_after_last:]

        new_macro = 'format!("' + ''.join(new_fmt).replace('"', '\\"') + '"' + remaining + ')'
        out.append(new_macro)
        changed = True
        i = pos
    new_src = ''.join(out)
    if changed and new_src != src:
        backup = path.with_suffix(path.suffix + '.bak_clippy')
        path.write_text(src, encoding='utf-8')
        backup.write_text(src, encoding='utf-8')
        path.write_text(new_src, encoding='utf-8')
        print(f"Modified {path}; backup at {backup}")
        return True
    return False


def main():
    root = Path('src')
    modified = []
    for p in root.rglob('*.rs'):
        if process_file(p):
            modified.append(str(p))
    print('Done. Modified files:', modified)


if __name__ == '__main__':
    main()
