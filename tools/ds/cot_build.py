"""One c-of-time build for a ROM, without make: its C code (and its
modules') compiled with the ARM GNU toolchain, linked into overlay 36,
then its hooks assembled with armips. What c-of-time's Makefile and
`scripts/patch.py` do (c-of-time: SkyTemple, MIT; its build scripts GPL-3),
done from Python, on the ROM in memory.

    python tools/ds/cot_build.py <rom.nds> <out.nds> [module folder ...]

Only one c-of-time build may be in a ROM (its code is the last 32 KB of
overlay 36): `patches.toml`'s c-of-time modules are merged into this one
build. Here the code starts `SKIP` bytes into overlay 36's common area
(0x30F70), so the small skypatches that put a few bytes at its start
(Chesyon's, from 0x30F70 + 0x4C) keep theirs.

Where things are (environment variables, else these defaults):
- `DW_COT`: c-of-time's source (`../pmd_explorer_of_sky/reference/c-of-time`
  beside this repository);
- `DW_PMDSKY_DEBUG`: pmdsky-debug (its `symbols/` and `headers/`): c-of-time's
  own submodule if it holds them, else `pmdsky-debug` beside c-of-time;
- `DW_ARM_TOOLCHAIN`: the ARM GNU toolchain's `bin` (`arm-none-eabi-gcc`);
  `../outils/arm-gnu-toolchain/bin` beside this repository by default;
- `SKYTEMPLE_ARMIPS_EXEC`: armips.

The build is kept under `target/ds/cot/<hash>/` (the hash of every source
file, the flags, the region): made once.
"""
import glob
import hashlib
import os
import shutil
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
PARENT = os.path.dirname(ROOT)
OUT = os.path.join(ROOT, 'target', 'ds', 'cot')

# Overlay 36: loaded at 0x23A7080, c-of-time's area from 0x30F70 to its end.
OVERLAY_36 = 0x23A7080
COMMON = 0x30F70
COMMON_SIZE = 0x8010
SKIP = 0x100
OPT = '-O2'
RELEASE = '-DDEBUG'


def cot_root():
    return os.environ.get('DW_COT') or os.path.join(PARENT, 'pmd_explorer_of_sky', 'reference', 'c-of-time')


def pmdsky_debug(cot):
    if os.environ.get('DW_PMDSKY_DEBUG'):
        return os.environ['DW_PMDSKY_DEBUG']
    own = os.path.join(cot, 'pmdsky-debug')
    if os.path.isdir(os.path.join(own, 'symbols')):
        return own
    return os.path.join(os.path.dirname(cot), 'pmdsky-debug')


def toolchain():
    return os.environ.get('DW_ARM_TOOLCHAIN') or os.path.join(PARENT, 'outils', 'arm-gnu-toolchain', 'bin')


def tool(name):
    path = os.path.join(toolchain(), f'arm-none-eabi-{name}')
    if os.name == 'nt':
        path += '.exe'
    if not os.path.isfile(path):
        raise SystemExit(f'c-of-time: {path} not found (set DW_ARM_TOOLCHAIN to the ARM GNU toolchain\'s bin)')
    return path


def run(command, cwd=None):
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f'c-of-time: {os.path.basename(command[0])} failed:\n{result.stdout}\n{result.stderr}')
    return result.stdout


def linker_symbols(debug, region):
    """pmdsky-debug's symbols for a region as a linker script (c-of-time's
    `generate_linkerscript.py`)."""
    import yaml
    lines, seen = ['/* Generated from pmdsky-debug by cot_build.py. */'], set()
    for path in sorted(glob.glob(os.path.join(debug, 'symbols', '**', '*.yml'), recursive=True)):
        with open(path, encoding='utf-8') as file:
            data = yaml.load(file, Loader=getattr(yaml, 'CSafeLoader', yaml.SafeLoader))
        for name, contents in data.items():
            address = contents.get('address', {})
            if region in address and f'{name.upper()}_LOAD_ADDR' not in seen:
                lines.append(f'{name.upper()}_LOAD_ADDR = {hex(address[region])};')
                seen.add(f'{name.upper()}_LOAD_ADDR')
            for symbol in contents.get('functions', []) + contents.get('data', []):
                addresses = symbol['address']
                at = addresses.get(f'{region}-ITCM', addresses.get(region))
                if isinstance(at, list):
                    if not at:
                        continue
                    at = at[0]
                if at is None:
                    continue
                for alias in [symbol['name']] + symbol.get('aliases', []):
                    if alias not in seen:
                        lines.append(f'{alias} = {hex(at)};')
                        seen.add(alias)
    return '\n'.join(lines) + '\n'


def sources_key(cot, debug, modules, region):
    digest = hashlib.sha1(f'{region}|{SKIP}|{OPT}|{RELEASE}|2'.encode())
    with open(__file__, 'rb') as file:
        digest.update(file.read())
    for folder in [os.path.join(cot, part) for part in ('src', 'include', 'patches', 'symbols', 'linker.ld')] + list(modules) + [os.path.join(debug, 'symbols')]:
        if os.path.isfile(folder):
            with open(folder, 'rb') as file:
                digest.update(file.read())
            continue
        for base, dirs, files in sorted(os.walk(folder)):
            dirs.sort()
            for name in sorted(files):
                digest.update(name.encode())
                with open(os.path.join(base, name), 'rb') as file:
                    digest.update(file.read())
    return digest.hexdigest()[:12]


def build(region, modules=(), tell=print):
    """The build's folder (`out.elf`, its patches), made once."""
    cot = cot_root()
    debug = pmdsky_debug(cot)
    for need in (os.path.join(cot, 'src'), os.path.join(debug, 'headers'), os.path.join(debug, 'symbols')):
        if not os.path.isdir(need):
            raise SystemExit(f'c-of-time: {need} not found (DW_COT, DW_PMDSKY_DEBUG)')
    key = sources_key(cot, debug, modules, region)
    work = os.path.join(OUT, key)
    elf = os.path.join(work, 'out.elf')
    if os.path.isfile(elf):
        return work
    if os.path.isdir(work):
        shutil.rmtree(work)
    os.makedirs(os.path.join(work, 'build'))
    for part in ('src', 'include', 'patches', 'symbols'):
        shutil.copytree(os.path.join(cot, part), os.path.join(work, part))
    for module in modules:
        shutil.copytree(module, os.path.join(work, 'modules', os.path.basename(module.rstrip('/\\'))))
    with open(os.path.join(cot, 'linker.ld'), encoding='utf-8') as file:
        linker = file.read()
    origin = f'ORIGIN = {hex(OVERLAY_36 + COMMON).upper().replace("X", "x")}, LENGTH = {hex(COMMON_SIZE)}'
    if 'ORIGIN = 0x23D7FF0, LENGTH = 0x8010' not in linker:
        raise SystemExit(f'c-of-time: linker.ld\'s main area is not where it was ({origin})')
    linker = linker.replace('ORIGIN = 0x23D7FF0, LENGTH = 0x8010',
                            f'ORIGIN = {hex(OVERLAY_36 + COMMON + SKIP)}, LENGTH = {hex(COMMON_SIZE - SKIP)}')
    with open(os.path.join(work, 'linker.ld'), 'w', encoding='utf-8') as file:
        file.write(linker)
    with open(os.path.join(work, 'symbols', f'generated_{region}.ld'), 'w', encoding='utf-8') as file:
        file.write(linker_symbols(debug, region))
    module_dirs = sorted(glob.glob(os.path.join(work, 'modules', '*')))
    sources = [os.path.join(work, 'src'), os.path.join(work, 'src', 'cot')] + [os.path.join(m, 'src') for m in module_dirs]
    includes = [os.path.join(work, 'include'), os.path.join(debug, 'headers')] + [os.path.join(m, 'include') for m in module_dirs]
    arch = ['-marm', '-mno-thumb-interwork']
    cflags = ['-g', '-Wall', OPT, RELEASE, '-march=armv5te', '-mtune=arm946e-s', '-fomit-frame-pointer', '-fno-short-enums',
              '-ffast-math', '-fno-builtin'] + arch + [f'-I{one}' for one in includes if os.path.isdir(one)] + ['-DARM9', '-flto', f'-DREGION_{region}']
    gcc = tool('gcc')
    objects = []
    for folder in sources:
        for path in sorted(glob.glob(os.path.join(folder, '*.c')) + glob.glob(os.path.join(folder, '*.s'))):
            obj = os.path.join(work, 'build', os.path.splitext(os.path.basename(path))[0] + '.o')
            if path.endswith('.s'):
                run([gcc, '-x', 'assembler-with-cpp', '-g'] + arch + [f'-DREGION_{region}', '-c', path, '-o', obj])
            else:
                run([gcc] + cflags + ['-c', path, '-o', obj])
            objects.append(obj)
    customs = sorted(glob.glob(os.path.join(work, 'symbols', f'custom_{region}.ld')) + glob.glob(os.path.join(work, 'symbols', f'custom_{region}_*.ld')))
    for module in module_dirs:
        customs += sorted(glob.glob(os.path.join(module, 'symbols', f'custom_{region}.ld')) + glob.glob(os.path.join(module, 'symbols', f'custom_{region}_*.ld')))
    ldflags = ['-T', os.path.join(work, 'symbols', f'generated_{region}.ld')]
    for one in customs:
        ldflags += ['-T', one]
    ldflags += ['-T', os.path.join(work, 'linker.ld'), '-g'] + arch + [f'-Wl,-Map,{os.path.join(work, "out.map")}', '-Xlinker', '-no-enum-size-warning',
                                                                        '-nostdlib', '-Xlinker', '--no-check-sections']
    run([gcc] + ldflags + objects + ['-o', elf + '.tmp'])
    os.replace(elf + '.tmp', elf)
    size = run([tool('size'), elf]).splitlines()[-1].split()[0]
    tell(f'  c-of-time: built ({len(objects)} files, {len(module_dirs)} modules, {size} bytes of code) in {work}')
    return work


def apply(rom, work, tell=print):
    """The build's code put in overlay 36 (and arm9 if it has a section
    there), then its hooks assembled (c-of-time's `scripts/patch.py`)."""
    import ndspy.code
    elf = os.path.join(work, 'out.elf')
    overlays = rom.loadArm9Overlays()
    if 36 not in overlays:
        raise SystemExit('c-of-time: no overlay 36: ExtraSpace first')
    symbols = {}
    for line in run([tool('nm'), elf]).splitlines():
        parts = line.split()
        if len(parts) >= 3:
            symbols[parts[2]] = int(parts[0], 16)
    sections = []
    lines = run([tool('objdump'), '-h', elf]).splitlines()
    for at, line in enumerate(lines):
        parts = line.split()
        if len(parts) >= 7 and parts[0].isdigit() and parts[1].startswith('.text'):
            sections.append((parts[1], int(parts[2], 16), int(parts[3], 16)))
    with tempfile.TemporaryDirectory() as tmp:
        for name, size, vma in sections:
            out = os.path.join(tmp, 'section.bin')
            run([tool('objcopy'), '-j', name, '-O', 'binary', elf, out])
            with open(out, 'rb') as file:
                code = file.read()
            if len(code) != size:
                raise SystemExit(f'c-of-time: {name}: {len(code)} bytes, {size} said')
            kind, number = name.split('.')[2], int(name.split('.')[3])
            if kind == 'ov':
                overlay = overlays[number]
                data, base = bytearray(rom.files[overlay.fileID]), overlay.ramAddress
            elif kind == 'arm' and number == 9:
                data, base = bytearray(rom.arm9), rom.arm9RamAddress
            else:
                raise SystemExit(f'c-of-time: section {name} not supported')
            start = vma - base
            if len(data) < start + size:
                data.extend(bytes(start + size - len(data)))
            data[start:start + size] = code
            if kind == 'ov':
                overlay.data = bytes(data)
                overlay.save()
                rom.files[overlay.fileID] = bytes(data)
                rom.arm9OverlayTable = ndspy.code.saveOverlayTable(overlays)
            else:
                rom.arm9 = bytes(data)
            tell(f'  c-of-time: {name} at {hex(vma)}-{hex(vma + size)}')
        # The hooks: armips on the binaries, with the build's symbols.
        binaries = os.path.join(tmp, 'binaries')
        os.makedirs(binaries)
        with open(os.path.join(binaries, 'symbols.asm'), 'w', encoding='utf-8') as file:
            for name, at in symbols.items():
                file.write(f'.definelabel {name},{hex(at)}\n')
            file.write(f'arm9_start equ {hex(rom.arm9RamAddress)}\narm9_end equ {hex(rom.arm9RamAddress + len(rom.arm9))}\n')
            file.write(f'arm7_start equ {hex(rom.arm7RamAddress)}\narm7_end equ {hex(rom.arm7RamAddress + len(rom.arm7))}\n')
            for index, overlay in overlays.items():
                file.write(f'overlay{index}_start equ {hex(overlay.ramAddress)}\noverlay{index}_end equ {hex(overlay.ramAddress + overlay.ramSize)}\n')
        with open(os.path.join(binaries, 'arm9.bin'), 'wb') as file:
            file.write(rom.arm9)
        with open(os.path.join(binaries, 'arm7.bin'), 'wb') as file:
            file.write(rom.arm7)
        for index, overlay in overlays.items():
            with open(os.path.join(binaries, f'overlay{index}.bin'), 'wb') as file:
                file.write(rom.files[overlay.fileID])
        armips = os.environ.get('SKYTEMPLE_ARMIPS_EXEC') or 'armips'
        for patch in sorted(glob.glob(os.path.join(work, 'patches', '*.asm'))) + sorted(glob.glob(os.path.join(work, 'modules', '*', 'patches', '*.asm'))):
            run([armips, patch, '-root', binaries])
            tell(f'  c-of-time: hooks of {os.path.relpath(patch, work)}')
        with open(os.path.join(binaries, 'arm9.bin'), 'rb') as file:
            rom.arm9 = file.read()
        with open(os.path.join(binaries, 'arm7.bin'), 'rb') as file:
            rom.arm7 = file.read()
        for index, overlay in overlays.items():
            with open(os.path.join(binaries, f'overlay{index}.bin'), 'rb') as file:
                rom.files[overlay.fileID] = file.read()


def main():
    args = sys.argv[1:]
    if len(args) < 2:
        print(__doc__)
        return 2
    from ndspy.rom import NintendoDSRom
    rom = NintendoDSRom.fromFile(args[0])
    work = build('EU', args[2:])
    apply(rom, work)
    rom.saveToFile(args[1])
    return 0


if __name__ == '__main__':
    sys.exit(main())
