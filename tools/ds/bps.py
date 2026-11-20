"""BPS patches (beat's format): a writer of our own and an applier to check it.

A ROM rebuilt by ndspy has its files moved by a few blocks: unchanged
stretches are found where they were, or 512-byte blocks further.
"""
import zlib


def number(value):
    out = bytearray()
    while True:
        part = value & 0x7F
        value >>= 7
        if value == 0:
            out.append(0x80 | part)
            return bytes(out)
        out.append(part)
        value -= 1


def same_run(source, target, s, t, limit):
    """How many bytes match from source[s] and target[t]."""
    run, step = 0, 4096
    while run < limit:
        size = min(step, limit - run)
        if source[s + run:s + run + size] == target[t + run:t + run + size]:
            run += size
            continue
        if size == 1:
            break
        step = max(1, size // 2)
    return run


def bps(source, target):
    source, target = memoryview(source), memoryview(target)
    index = {}
    for at in range(0, len(source) - 64, 512):
        index.setdefault(bytes(source[at:at + 64]), at)
    out = bytearray(b'BPS1' + number(len(source)) + number(len(target)) + number(0))
    t, literal_from, source_at, copy_at = 0, None, 0, 0

    def flush_literal(end):
        nonlocal literal_from
        if literal_from is not None:
            out.extend(number(((end - literal_from) - 1) << 2 | 1))
            out.extend(target[literal_from:end])
            literal_from = None

    while t < len(target):
        best, where = 0, None
        # Where it was, as the last copy went on, or at the same place.
        for s in (source_at, t):
            if 0 <= s < len(source):
                limit = min(len(source) - s, len(target) - t)
                run = same_run(source, target, s, t, limit)
                if run > best:
                    best, where = run, s
        if best < 32 and t % 512 == 0 and t + 64 <= len(target):
            s = index.get(bytes(target[t:t + 64]))
            if s is not None:
                run = same_run(source, target, s, t, min(len(source) - s, len(target) - t))
                if run > best:
                    best, where = run, s
        if best >= 32:
            flush_literal(t)
            if where == t:
                out.extend(number((best - 1) << 2 | 0))  # SourceRead
            else:
                relative = where - copy_at
                out.extend(number((best - 1) << 2 | 2))  # SourceCopy
                out.extend(number((abs(relative) << 1) | (1 if relative < 0 else 0)))
                copy_at = where + best
            source_at = where + best
            t += best
            continue
        if literal_from is None:
            literal_from = t
        # On to the next block boundary, where a moved file may start.
        t = min(len(target), (t // 512 + 1) * 512) if best == 0 else t + max(best, 1)
    flush_literal(len(target))
    out.extend(zlib.crc32(source).to_bytes(4, 'little'))
    out.extend(zlib.crc32(target).to_bytes(4, 'little'))
    out.extend(zlib.crc32(out).to_bytes(4, 'little'))
    return bytes(out)


def apply_bps(source, patch):
    """The patched file: to check what `bps` writes."""
    at = 4

    def read_number():
        nonlocal at
        value, shift = 0, 1
        while True:
            byte = patch[at]
            at += 1
            value += (byte & 0x7F) * shift
            if byte & 0x80:
                return value
            shift <<= 7
            value += shift

    read_number()
    target = bytearray(read_number())
    metadata = read_number()
    at += metadata
    out, source_at, target_at = 0, 0, 0
    while at < len(patch) - 12:
        data = read_number()
        kind, length = data & 3, (data >> 2) + 1
        if kind == 0:
            target[out:out + length] = source[out:out + length]
        elif kind == 1:
            target[out:out + length] = patch[at:at + length]
            at += length
        elif kind == 2:
            offset = read_number()
            source_at += -(offset >> 1) if offset & 1 else offset >> 1
            target[out:out + length] = source[source_at:source_at + length]
            source_at += length
        else:
            offset = read_number()
            target_at += -(offset >> 1) if offset & 1 else offset >> 1
            for i in range(length):
                target[out + i] = target[target_at + i]
            target_at += length
        out += length
    return bytes(target)


