// Ground sprite room: LoadWanTableEntryFromPack's MemAlloc(len, flags),
// with the flags GsrSpriteAllocFlags picks. MemAlloc is entered by a
// branch, lr untouched: MemLocateSet's caller is still
// LoadWanTableEntryFromPack, which FixMemorySoftlock checks on a failure.
// Copyright (C) 2026 Dungeon Workshop. GPL-3.0-or-later.

.text
.align 4
.global GsrSpriteAllocTrampoline
GsrSpriteAllocTrampoline:
  push {r0, lr}
  bl GsrSpriteAllocFlags
  mov r1, r0
  pop {r0, lr}
  b MemAlloc
