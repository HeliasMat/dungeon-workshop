"""Lay the shots taken by emu.py side by side, in frame order, with the frame
number above each one.

    python tools/ds/sheet.py <out.png> <shot prefix> [columns] [first-last]

With `first-last`, only the shots of those frames.

The shots are `<prefix>_<frame>.png` (both DS screens, top above bottom).
The sheet is made from a capture of the user's own game: never commit it.
"""
import glob
import re
import sys

from PIL import Image, ImageDraw


def main():
    if len(sys.argv) < 3:
        print(__doc__)
        return 2
    out, prefix = sys.argv[1], sys.argv[2]
    columns = int(sys.argv[3]) if len(sys.argv) > 3 else 6
    shots = []
    for path in glob.glob(prefix + '_*.png'):
        found = re.search(r'_(\d+)\.png$', path)
        if found:
            shots.append((int(found.group(1)), path))
    shots.sort()
    if len(sys.argv) > 4:
        first, last = (int(part) for part in sys.argv[4].split("-"))
        shots = [(frame, path) for frame, path in shots if first <= frame <= last]
    if not shots:
        print('no shots')
        return 1
    first = Image.open(shots[0][1])
    width, height = first.size
    label = 12
    rows = (len(shots) + columns - 1) // columns
    sheet = Image.new('RGB', (columns * width, rows * (height + label)), (0, 0, 0))
    draw = ImageDraw.Draw(sheet)
    for index, (frame, path) in enumerate(shots):
        x, y = (index % columns) * width, (index // columns) * (height + label)
        draw.text((x + 2, y), str(frame), fill=(255, 255, 0))
        sheet.paste(Image.open(path).convert('RGB'), (x, y + label))
    sheet.save(out)
    print(out, len(shots), 'shots')
    return 0


if __name__ == '__main__':
    sys.exit(main())
