// Ground sprite room: more room for the sprites of a cutscene's Pokémon.
//
// Copyright (C) 2026 Dungeon Workshop. GPL-3.0-or-later.
// A c-of-time module (c-of-time: SkyTemple, MIT); the game's memory
// allocator as pmdsky-debug documents it (GPL-3), read in pmd-sky's
// decompiled ground code (GROUND_MEMORY_ARENA_2 made in GroundMainLoop,
// LoadWanTableEntryFromPack's two MemAlloc calls, GetFreeArenaGround).
//
// In ground mode each species' sprite is loaded once, by
// LoadWanTableEntryFromPack, into GROUND_MEMORY_ARENA_2 (allocation flags
// 14): a sub-arena of 0xB0000 bytes (0xAFCE4 usable, 32 blocks) that
// GroundMainLoop takes from the default arena. A scene with ~17 species of
// Sky's (about 10 of SpriteCollab's, larger) fills it; the next sprite
// finds no room in MemLocateSet (FixMemorySoftlock then draws a default
// sprite; without it the game stops).
//
// A sprite that does not fit in arena 2 goes to GROUND_MEMORY_ARENA_1
// (flags 15: 0x63B04 bytes, where the game already puts its ground effects
// and some actors' sprites, nearly empty in a cutscene) when
// GSR_ARENA1_RESERVE stays free in one block after it; else to arena 2 as
// the game does (FixMemorySoftlock's default sprite). Freed as before:
// MemFree asks GetFreeArenaGround, which finds arena 2 or arena 1 by the
// pointer. Arena 1 is made again each time ground mode starts.
//
// Measured, not used: the default arena has about 0x28000 free in a scene
// (it holds arena 2); growing arena 2 when GroundMainLoop makes it hung the
// title screen, a ground mode too, out of default-arena room.

#include <pmdsky.h>
#include <cot.h>

// What arena 1 keeps for its own users (its effects and others: 0xB300
// seen at most in a scene with moves).
#define GSR_ARENA1_RESERVE 0x20000
// User flags: GetAllocArenaGround gives arena 1 for 15.
#define GSR_ARENA1_FLAGS 15
// The allocator flags MemLocateSet passes FindAvailableMemBlock for a
// plain allocation (best fit).
#define GSR_FIND_FLAGS 1

static bool Fits(struct mem_arena* arena, uint32_t len, uint32_t reserve) {
  // (A split adds a block: past max_blocks is the game's fatal error.)
  return arena->n_blocks + 1 < arena->max_blocks &&
         FindAvailableMemBlock(arena, GSR_FIND_FLAGS, ((len + 3) & ~3) + reserve) >= 0;
}

// LoadWanTableEntryFromPack's MemAlloc(len, flags), through the trampoline:
// the flags to allocate with.
__attribute__((used)) uint32_t GsrSpriteAllocFlags(uint32_t len, uint32_t flags) {
  if ((flags & 0xFF) != 14 || MEMORY_ALLOCATION_ARENA_GETTERS.get_alloc_arena != GetAllocArenaGround) {
    return flags;
  }
  struct mem_arena* arena2 = GetAllocArenaGround(NULL, 14);
  struct mem_arena* arena1 = GetAllocArenaGround(NULL, GSR_ARENA1_FLAGS);
  if (arena2 == NULL || Fits(arena2, len, 0)) {
    return flags;
  }
  if (arena1 != NULL && arena1 != arena2 && Fits(arena1, len, GSR_ARENA1_RESERVE)) {
    return (flags & ~0xFF) | GSR_ARENA1_FLAGS;
  }
  return flags;
}
