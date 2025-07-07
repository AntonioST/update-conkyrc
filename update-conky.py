#! /usr/bin/env python

import argparse
import re
import sys
from pathlib import Path

AP = argparse.ArgumentParser(description="update conky hwmon path")
AP.add_argument('-i', '--inplace', action='store_true', help='inplace conky rc file')
AP.add_argument('-c', '--color', metavar='COLOR', default=None, help='change primary color')
AP.add_argument('--no-coretemp', action='store_true', help='do not update coretemp path')
AP.add_argument('--no-nvme', action='store_true', help='do not update nvme (disk) path')
AP.add_argument(metavar='RC', nargs='?', default=None, dest='RC', help='rc file')

OPT = AP.parse_args()

RC = OPT.RC
if RC is None:
    RC = Path.home() / '.conkyrc'

RC = Path(RC)
if not RC.exists():
    raise FileNotFoundError(f'{RC} file not exists')

CONTENT = RC.read_text()

def find_hwmon_path(cate: str) -> str | None:
    i = 0
    while (p := Path(f'/sys/class/hwmon/hwmon{i}/name')).exists():
        name = p.read_text()
        if name == cate:
            return str(i)
        i += 1
    return None

def update_hwmon_path(line: str, num: str | None) -> str:
    if num is None:
        return line
    return re.sub(r'hwmon \d+', f'hwmon {num}', line)

OUTPUT = []

in_config = False

for line in CONTENT.split('\n'):
    if line.startswith('--'):
        OUTPUT.append(line)
    elif line.startswith('conky.config'):
        in_config = True
        OUTPUT.append(line)
    elif in_config and line == '}':
        in_config = False
        OUTPUT.append(line)
    elif in_config and '=' in line and (color := OPT.color) is not None:
        eqi = line.index('=')
        config_key = line[:eqi].strip()
        if config_key in ('color0', 'default_color', 'default_outline_color', 'default_shade_color'):
            OUTPUT.append(f"{line[:eqi]}= '{color}',")
        else:
            OUTPUT.append(line)
    elif 'hwmon' not in line:
        OUTPUT.append(line)
    elif line.startswith('== CPU ==') and not OPT.no_coretemp:
        OUTPUT.append(update_hwmon_path(line, find_hwmon_path('coretemp')))
    elif line.startswith('== Disk IO ==') and not OPT.no_nvme:
        OUTPUT.append(update_hwmon_path(line, find_hwmon_path('nvme')))
    else:
        OUTPUT.append(line)

CONTENT = '\n'.join(OUTPUT)

if OPT.inplace:
    with RC.open('w') as out:
        print(CONTENT, file=out)
else:
    print(CONTENT)
