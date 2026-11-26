"""The mega patch: the community's patches for cutscenes (`patches.toml`)
applied together, in the order of their dependencies.

    python tools/ds/megapatch.py --list
    python tools/ds/megapatch.py <rom> --out <out.nds> [--share] [--bps <out.bps>]
    python tools/ds/megapatch.py --fetch

- `--list`: every patch, on or off, shared or not, tested.
- `<rom> --out`: your ROM with the patches that are on (no film); `--share`
  only those whose licence lets them be shared (`share = "yes"`); `--bps`
  also writes the patch from your ROM to it (patch_rom.py's BPS writer): the
  one to give, with `--share`.
- `--fetch`: the patches without a licence (`share = "no"`) read from their
  source on GitHub (plain text, through `gh api`) into
  `target/ds/megapatch/community/`: they are never in the repository.

`DW_PATCHES` changes the choice: `+Name` adds a patch that is off, `-Name`
takes one away, `only:A,B` keeps only those (and what they need):
`DW_PATCHES="+AddTypes,-SkipQuiz"`.

`ds_cache.py` makes its base with `apply` (the films' cache), so a film
played on DS has them all. The special processes' ids are told, and kept in
the base's manifest (`special_processes`).

Never on the original cartridge dump: the ROM is only read.
"""
import json
import os
import re
import subprocess
import sys
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
LIST = os.path.join(HERE, 'patches.toml')
COMMUNITY = os.path.join(ROOT, 'target', 'ds', 'megapatch', 'community')


def load():
    with open(LIST, 'rb') as file:
        return tomllib.load(file)['patch']


def choice():
    """DW_PATCHES as it is (part of the base's key)."""
    return os.environ.get('DW_PATCHES', '')


def key():
    """What the chosen patches are made of (the base's key): the choice and
    every file of theirs."""
    import hashlib
    digest = hashlib.sha1(choice().encode())
    for one in chosen(load()):
        digest.update(one['name'].encode())
        path = os.path.join(ROOT, one.get('path', ''))
        if not one.get('path') or not os.path.exists(path):
            continue
        files = [path] if os.path.isfile(path) else sorted(os.path.join(base, name) for base, _, names in os.walk(path) for name in names)
        for name in files:
            with open(name, 'rb') as file:
                digest.update(file.read())
    return digest.hexdigest()[:10]


def chosen(patches, share=False):
    """The patches to apply, each after what it needs."""
    by_name = {one['name']: one for one in patches}
    wanted = [one['name'] for one in patches if one.get('default') == 'on']
    asked = choice().strip()
    if asked.startswith('only:'):
        wanted = [name for name in asked[5:].split(',') if name]
    elif asked:
        for word in asked.split(','):
            word = word.strip()
            if word.startswith('+') and word[1:] not in wanted:
                wanted.append(word[1:])
            elif word.startswith('-') and word[1:] in wanted:
                wanted.remove(word[1:])
    for name in wanted:
        if name not in by_name:
            raise SystemExit(f'DW_PATCHES: no patch {name} in patches.toml')
    order = []

    def visit(name, path=()):
        if name in order:
            return
        if name in path:
            raise SystemExit(f'patches.toml: a dependency loop through {name}')
        for need in by_name[name].get('depends', []):
            if need not in by_name:
                raise SystemExit(f'patches.toml: {name} needs {need}, not listed')
            visit(need, path + (name,))
        order.append(name)

    for name in wanted:
        visit(name)
    picked = [by_name[name] for name in order]
    if share:
        refused = [one['name'] for one in picked if one.get('share') != 'yes']
        picked = [one for one in picked if one.get('share') == 'yes'
                  and all(by_name[need].get('share') == 'yes' for need in one.get('depends', []))]
        if refused:
            print(f'  not shared (no licence stated): {", ".join(refused)}')
    return picked


def for_region(source, region):
    """An SP's armips source with its region's lines chosen: the lines under
    `; For EU` (or US, NA, JP) uncommented, the others' commented, and the
    `.create` at its region's address (`; For EU: 0x...`)."""
    names = {'EU': ('EU',), 'US': ('US', 'NA'), 'JP': ('JP',)}[region]
    out, block = [], None
    for line in source.splitlines():
        heading = re.match(r'\s*;\s*For (US|NA|EU|JP)\s*$', line)
        if heading:
            block = heading.group(1)
            out.append(line)
            continue
        body = line.lstrip(';').strip()
        if block and re.match(r'\.(include|definelabel)\b', body):
            # (A typo seen in Adex's EU lines: `Label. 0x...` for `Label, 0x...`.)
            body = re.sub(r'^(\.definelabel\s+\w+)\.\s', r'\1, ', body)
            out.append(body if block in names else ';' + body)
            continue
        block = None
        create = re.match(r'(\s*\.create\s+"[^"]+",\s*)(0x[0-9A-Fa-f]+)(.*)$', line)
        if create:
            other = re.search(rf'For (?:{"|".join(names)}):\s*(0x[0-9A-Fa-f]+)', create.group(3))
            if other:
                line = f'{create.group(1)}{other.group(1)}{create.group(3)}'
        out.append(line)
    return '\n'.join(out) + '\n'


def region_of(config):
    from skytemple_files.common.ppmdu_config.data import GAME_REGION_EU, GAME_REGION_US, GAME_REGION_JP
    return {GAME_REGION_EU: 'EU', GAME_REGION_US: 'US', GAME_REGION_JP: 'JP'}[config.game_region]


def path_of(one):
    path = os.path.join(ROOT, one['path'])
    if not os.path.exists(path):
        raise SystemExit(f'{one["name"]}: {one["path"]} not found (python tools/ds/megapatch.py --fetch reads it from {one["source"]})')
    return path


def apply(rom, config, patcher=None, tell=print, share=False):
    """The chosen patches applied to `rom`: {'applied': [...],
    'special_processes': {name: id}, 'cot': build folder or None}."""
    import cot_build
    from skytemple_files.patch.patches import Patcher
    patcher = patcher or Patcher(rom, config)
    region = region_of(config)
    picked = chosen(load(), share)
    applied, processes, modules = [], {}, []
    for one in picked:
        name, kind = one['name'], one['kind']
        if kind == 'skypatch':
            # (Applied without asking the package whether it is: the ROM is
            # always the clean one, and marius851000's check reads a RAM
            # address as an offset.)
            if name not in [handler.name for handler in patcher.list()]:
                patcher.add_pkg(staged_package(path_of(one), region), is_zipped=False)
            patcher.apply(name, dict(one['params']) if 'params' in one else None)
            applied.append(name)
        elif kind == 'skytemple':
            if not patcher.is_applied(name):
                patcher.apply(name, dict(one['params']) if 'params' in one else None)
            applied.append(name)
        elif kind == 'sp':
            processes[name] = add_process(rom, path_of(one), region)
            applied.append(name)
        elif kind == 'cot':
            modules.append(path_of(one))
            applied.append(name)
        elif kind == 'left-out':
            raise SystemExit(f'{name} is left out: {one.get("notes", "")}')
        else:
            raise SystemExit(f'patches.toml: {name}: kind {kind}?')
    work = None
    if modules:
        work = cot_build.build(region, modules, tell)
        cot_build.apply(rom, work, tell)
    tell(f'  patches: {", ".join(applied)}')
    for name, number in processes.items():
        tell(f'  special process {number}: {name}')
    return {'applied': applied, 'special_processes': processes, 'cot': work}


def staged_package(path, region):
    """A patch package's folder as SkyTemple reads it. One kept as its
    author's sources (marius851000's: its `.asm` beside `config.xml`, opening
    `arm9.bin` itself) is copied with its `.asm` in `asm_patches/`, without
    its own `.open`/`.close` (SkyTemple's OpenBin opens the binary), and an
    empty header stub for the region."""
    import shutil
    import tempfile
    if os.path.isdir(os.path.join(path, 'asm_patches')):
        return path
    staged = os.path.join(tempfile.mkdtemp(prefix='dw_skypatch_'), os.path.basename(path))
    shutil.copytree(path, staged)
    asm = os.path.join(staged, 'asm_patches')
    os.makedirs(asm)
    for name in os.listdir(staged):
        if not name.endswith('.asm'):
            continue
        with open(os.path.join(staged, name), encoding='utf-8') as file:
            lines = [line for line in file.read().splitlines()
                     if not re.match(r'\s*\.(open|close|nds)\b', line)]
        with open(os.path.join(asm, name), 'w', encoding='utf-8') as file:
            file.write('\n'.join(lines) + '\n')
        os.remove(os.path.join(staged, name))
    stub = {'EU': 'eu', 'US': 'na', 'JP': 'jp'}[region]
    with open(os.path.join(asm, f'header_stub_{stub}.asm'), 'w', encoding='utf-8') as file:
        file.write('.relativeinclude on\n.nds\n.arm\n')
    return staged


def add_process(rom, path, region):
    """An SP's code added to BALANCE/process.bin: its id."""
    from skytemple_files.data.data_cd.armips_importer import ArmipsImporter
    from skytemple_files.data.data_cd.handler import DataCDHandler
    with open(path, encoding='utf-8') as file:
        source = for_region(file.read(), region)
    code = ArmipsImporter().assemble(source)
    processes = DataCDHandler.deserialize(rom.getFileByName('BALANCE/process.bin'))
    processes.add_effect_code(code)
    processes.add_item_effect_id(processes.nb_effects() - 1)
    rom.setFileByName('BALANCE/process.bin', DataCDHandler.serialize(processes))
    return processes.nb_items() - 1


def github_parts(url):
    """owner/repo, ref, path of a github.com/<owner>/<repo>[/tree|blob/<ref>/<path>] link."""
    found = re.match(r'https://github\.com/([^/]+/[^/]+)(?:/(?:tree|blob)/([^/]+)/(.*))?$', url)
    if not found:
        raise SystemExit(f'not a GitHub link: {url}')
    return found.group(1), found.group(2), found.group(3) or ''


SKIP = ('.skypatch', '.bin', '.zip', '.7z', '.nds', '.xdelta', '.bps', '.ips', '.png', '.gif', '.jpg', '.o', '.elf')


def fetch_folder(repo, ref, path, dest):
    """A GitHub file or folder's text files read into `dest` (gh api)."""
    suffix = f'?ref={ref}' if ref and ref != 'HEAD' else ''
    listing = json.loads(subprocess.run(['gh', 'api', f'repos/{repo}/contents/{path}{suffix}'], capture_output=True, check=True).stdout)
    single = isinstance(listing, dict)
    for entry in [listing] if single else listing:
        if entry['type'] == 'dir':
            fetch_folder(repo, ref, entry['path'], os.path.join(dest, entry['name']))
        elif entry['type'] == 'file' and not entry['name'].lower().endswith(SKIP):
            target = dest if single else os.path.join(dest, entry['name'])
            os.makedirs(os.path.dirname(target), exist_ok=True)
            data = subprocess.run(['gh', 'api', f'repos/{repo}/contents/{entry["path"]}{suffix}', '-H', 'Accept: application/vnd.github.raw'],
                                  capture_output=True, check=True).stdout
            with open(target, 'wb') as file:
                file.write(data)


def fetch():
    for one in load():
        if one.get('share') == 'yes' or not one.get('path', '').startswith('target/'):
            continue
        dest = os.path.join(ROOT, one['path'])
        if os.path.exists(dest):
            print(f'{one["name"]}: already in {one["path"]}')
            continue
        repo, ref, path = github_parts(one['source'])
        fetch_folder(repo, ref, path, dest)
        print(f'{one["name"]}: read from {one["source"]} into {one["path"]}')


def make(rom_path, out, share=False, bps_out=None):
    for name in ('matplotlib', 'plotly'):
        sys.modules.setdefault(name, None)
    from ndspy.rom import NintendoDSRom
    from skytemple_files.common.util import get_ppmdu_config_for_rom
    with open(rom_path, 'rb') as file:
        original = file.read()
    rom = NintendoDSRom(original)
    config = get_ppmdu_config_for_rom(rom)
    done = apply(rom, config, share=share)
    data = rom.save(updateDeviceCapacity=True)
    with open(out, 'wb') as file:
        file.write(data)
    print(f'{out}: {len(done["applied"])} patches, {len(data)} bytes')
    if bps_out:
        from bps import apply_bps, bps
        made = bps(original, data)
        if apply_bps(original, made) != data:
            raise SystemExit('the BPS patch does not give the ROM back')
        with open(bps_out, 'wb') as file:
            file.write(made)
        print(f'{bps_out}: {len(made)} bytes')
    with open(out + '.json', 'w', encoding='utf-8') as file:
        json.dump({'applied': done['applied'], 'special_processes': done['special_processes'], 'share': share}, file, indent=1)


def show():
    picked = {one['name'] for one in chosen(load())}
    for one in load():
        mark = 'on ' if one['name'] in picked else 'off'
        print(f'{mark} {one["name"]:28} {one["kind"]:9} share {one.get("share", "?"):3}  {one.get("tested", "")}')


def main():
    args = sys.argv[1:]
    if args == ['--list']:
        show()
        return 0
    if args == ['--fetch']:
        fetch()
        return 0
    if len(args) >= 3 and args[1] == '--out':
        share = '--share' in args
        bps_out = args[args.index('--bps') + 1] if '--bps' in args else None
        make(args[0], args[2], share, bps_out)
        return 0
    print(__doc__)
    return 2


if __name__ == '__main__':
    sys.path.insert(0, HERE)
    sys.exit(main())
