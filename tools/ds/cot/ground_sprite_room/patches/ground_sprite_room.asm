// Ground sprite room's hooks (EU). Copyright (C) 2026 Dungeon Workshop. GPL-3.0-or-later.
// Addresses: symbols/custom_EU_gsr.ld (pmdsky-debug's LoadWanTableEntryFromPack;
// its two calls after pmd-sky's decompiled code, checked in the EU ROM).

.nds
.include "symbols.asm"

.open "arm9.bin", arm9_start
  // LoadWanTableEntryFromPack: the decompressed sprite's MemAlloc, then the raw one's.
  .org GsrWanAllocCompressedCall
  .area 0x4
    bl GsrSpriteAllocTrampoline
  .endarea

  .org GsrWanAllocRawCall
  .area 0x4
    bl GsrSpriteAllocTrampoline
  .endarea
.close
