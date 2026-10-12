//! How complete each translation is: `cargo run -p tkmm_core --example i18n_status`.
//! Includes drop-in folders in %APPDATA%\TKModManager\locales. Pass `-v` to list missing keys.

fn main() {
    let verbose = std::env::args().any(|a| a == "-v" || a == "--verbose");
    let langs = tkmm_core::i18n::coverage();
    if langs.is_empty() {
        println!("Only English so far. Copy crates/core/locales/en to start a translation (see TRANSLATING.md).");
        return;
    }
    for c in langs {
        let pct = if c.total == 0 { 100.0 } else { c.translated as f64 * 100.0 / c.total as f64 };
        println!("{:<8} {:<24} {:>5.1}%  {}/{} messages", c.code, c.name, pct, c.translated, c.total);
        if !c.unknown.is_empty() {
            println!("         not in English (renamed or removed, delete them): {}", c.unknown.join(", "));
        }
        if verbose {
            for k in &c.missing {
                println!("         missing: {k}");
            }
        } else if !c.missing.is_empty() {
            println!("         {} missing (run with -v to list them)", c.missing.len());
        }
    }
}
