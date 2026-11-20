# Credits

Dungeon Workshop is free software, under the **GPL-3.0 or later** license (see [LICENSE](LICENSE)). It was written building on the open work of the community that studies the games of the genre. Thank you to them.

## Code adapted from other projects

| What is here | Based on | License |
|---|---|---|
| The floor generator (`rules/src/dungeon/generator/`) | [dungeon-eos](https://github.com/SkyTemple/dungeon-eos), the Python port of the generator, by SkyTemple | GPL-3.0 |
| The readers for the cartridge's file formats (`rom_formats/src/`: portraits, fixed rooms, backgrounds, tiles, menus, items, sprites, scenes) | [skytemple-files](https://github.com/SkyTemple/skytemple-files) and [skytemple-rust](https://github.com/SkyTemple/skytemple-rust), by SkyTemple | GPL-3.0 |
| Dungeon tilesets of one's own drawn on a template of 47 cells (`media/src/template.rs`) | [skytemple-dtef](https://github.com/SkyTemple/skytemple-dtef), the Dungeon Tile Exchange Format, by SkyTemple | GPL-3.0 |
| Converting a picture to the console's rules, tiles of 8 with 16 colours each (`media/src/quant.rs`, the idea, written anew) | [tilequant](https://github.com/SkyTemple/tilequant), by SkyTemple | GPL-3.0 |
| A Pokémon's sheets written out and read back in SpriteBot's layout, the 5×4 sheet of faces (`spritecollab/src/lib.rs`) | [skytemple-files](https://github.com/SkyTemple/skytemple-files)' `kao/sprite_bot_sheet.py`, by SkyTemple | GPL-3.0 |
| The addresses, numbers and data layout of the cartridge (`rom_formats/src/sym.rs`, written from its symbols by `rom_formats/examples/rom_symbols.rs`) | [pmdsky-debug](https://github.com/UsernameFodder/pmdsky-debug) | GPL-3.0 |
| The guests' tables of the EditGuestPokemon patch and the symbols SkyTemple's patches add (`rom_formats/src/guests.rs`, `rom_symbols.rs`) | [skytemple-files](https://github.com/SkyTemple/skytemple-files), by SkyTemple | GPL-3.0 |
| Reading the scripts (jumps, branches, variables) | [ExplorerScript](https://github.com/SkyTemple/ExplorerScript), by SkyTemple | MIT |
| Notes on the audio formats | [ppmdu](https://github.com/PsyCommando/ppmdu), by PsyCommando | public domain |

The rules of fights and dungeons (damage, accuracy, statuses, abilities, weather, AI, spawns) are written by this project. They are inspired by the known behaviour of the games of the genre, without taking anyone's code.

## What is not here

This repository contains **no game data**: no pictures, sounds, music, dialogue or scripts from any published game. The tool reads them from **your own copy** of the cartridge, after the extraction done on your own machine (see the [README](README.md)). The pictures, sounds and music in the `library` folder are made by the project's own tools.

The only words of the game found here are **names**: those of species, moves, items, places and music tracks. Films and games name what they use, as one names a character, and the tests' small lists (`tests/fixture_data/`) stand in for the extracted data so that they run without a cartridge. These names are used only to refer to what is read from your own copy.

## Rights holders

If you hold rights to something in this repository and consider that it should not be here, please [open an issue](https://github.com/HeliasMat/dungeon-workshop/issues). It will be looked at promptly and, where it is founded, the element will be removed.

*Pokémon* and *Pokémon Mystery Dungeon* are trademarks of Nintendo, The Pokémon Company, Game Freak, Creatures and Spike Chunsoft. This project is neither affiliated with nor endorsed by these companies.
