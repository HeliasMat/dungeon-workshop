# dungeon_lsp

A language server (LSP over stdin/stdout) for films (`.film`), games (`.game`) and decors (`.decor`): the Workshop's help in any editor.

```sh
cargo build --release -p dungeon_lsp    # target/release/dungeon_lsp(.exe)
```

Open the project's root folder in the editor (the server looks for `library/films` from there, as the Workshop does).

| | film | game | decor |
|---|---|---|---|
| Problems (errors, remarks, details) | the film service, with line and column | the language's errors; opened or saved, the whole game checked (`game::project_check`, as `scene_player check-game`) | the decor reader's first error, on its line |
| Fixes (code actions) | yes | the language's | |
| Completion | words, recipes, blocks, values, what a word wants there; places and Pokémon on `scene:`, faces in `who (…)` | | |
| Hover | a word's forms and what it does, a recipe, a value | | |
| Go to definition | a recipe (`define`, a shared file, the library), a value, a mark; a `uses` / `include` file | | |
| Formatting, outline, folding | yes | | |

A `.game` opened or saved is checked whole: its films, places, dungeons and fights, the names, the flags, the reach of its chapters. What is wrong is told on the line of the file it is in, for the `.game` itself and for the files open in the editor (open a place or a film to see its own); it is checked again when the `.game` is saved. It reads the game's data from `data/` (the items and the Pokémon are only checked with it).

## VS Code

With a generic LSP client extension (for example "Generic LSP Client", `zsol.vscode-glspc`), in `settings.json`:

```json
{
  "files.associations": { "*.film": "film", "*.game": "game", "*.decor": "decor" },
  "glspc.server.command": "C:/path/to/target/release/dungeon_lsp.exe",
  "glspc.server.languageId": ["film", "game", "decor"]
}
```

## Neovim

```lua
vim.filetype.add({ extension = { film = "film", game = "game", decor = "decor" } })
vim.api.nvim_create_autocmd("FileType", {
  pattern = { "film", "game", "decor" },
  callback = function(args)
    vim.lsp.start({
      name = "dungeon_lsp",
      cmd = { "/path/to/target/release/dungeon_lsp" },
      root_dir = vim.fs.root(args.buf, { "tests" }),
    })
  end,
})
```

## Helix

In `languages.toml`:

```toml
[language-server.dungeon_lsp]
command = "/path/to/target/release/dungeon_lsp"

[[language]]
name = "film"
scope = "source.film"
file-types = ["film"]
language-servers = ["dungeon_lsp"]
```
