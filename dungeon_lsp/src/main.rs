//! `dungeon_lsp`: the language server, over stdin and stdout.

fn main() {
    let (stdin, stdout) = (std::io::stdin(), std::io::stdout());
    let code = dungeon_lsp::run(&mut stdin.lock(), &mut stdout.lock()).unwrap_or_else(|why| {
        eprintln!("dungeon_lsp: {why}");
        1
    });
    std::process::exit(code);
}
