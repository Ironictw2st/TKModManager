# Translating TK Mod Manager

All of the app's text lives in plain-text files, one folder per language:

```
crates/core/locales/
  en/          English: the original, always complete
    main.ftl  settings.ftl  mods.ftl  window.ftl  tasks.ftl  core-*.ftl
  de/          a translation: the same files, translated
```

You need no programming knowledge and you don't need to build the app. Any text editor works,
as long as it saves files as UTF-8 (Notepad, VS Code and Notepad++ all do).

## Start a new language

1. Copy the whole `en` folder and name the copy after your language's code: `de` (German),
   `fr` (French), `ru` (Russian), `pl` (Polish), `es` (Spanish), `zh-CN` (Simplified Chinese),
   `zh-TW` (Traditional Chinese), `pt-BR` (Brazilian Portuguese), `ja`, `ko`, `tr` and so on.
   Use [BCP 47 codes](https://en.wikipedia.org/wiki/IETF_language_tag), with a hyphen.
2. In `main.ftl`, change `language-name = English` to your language's name written in that
   language, for example `language-name = Deutsch`. The language pickers show this name.
3. Translate the text to the right of each `=`. Leave everything to the left alone.

A message you haven't translated yet (or delete) shows in English. That means you can
translate a few files, try them, and do the rest later.

## The file format (Fluent)

The files use [Fluent](https://projectfluent.org/fluent/guide/). In short:

```ftl
# A comment: notes for translators. You don't need to translate comments.
settings-language = Language

# Variables are in { $braces }. Keep their names exactly; you may move them around.
status-up-to-date = Up to date (updated { $date })

# Plurals. The `*` marks the default. Use the plural categories your language has:
# zero, one, two, few, many, other (see the CLDR plural rules for your language).
mods-count = { $count ->
    [one] { $count } mod
   *[other] { $count } mods
}
```

The same message in Polish, which has more plural forms than English:

```ftl
mods-count = { $count ->
    [one] { $count } mod
    [few] { $count } mody
   *[many] { $count } modów
}
```

Rules:

- **Don't rename or translate the key** (the part before `=`), or the app can't find the text.
- **Don't rename `{ $variables }`.** You may leave one out if your sentence doesn't need it.
- **Long text** can continue on the next lines, as long as those lines are indented:
  ```ftl
  some-message =
      First line of a long message
      that continues here.
  ```
- **A line break inside the text** is a real line break (used in some dialogs). Keep blank lines
  where the English has them.
- **Special characters:** to write a literal `{` use `{"{"}`. Keep a message's leading space
  with `{" "}`. Lines that start with `[`, `*` or `.` must be written as `{"["}`, and so on.
- **Product and game names** (TK Mod Manager, Total War: THREE KINGDOMS, Steam, Nexus Mods,
  Workshop, script extender, RPFM) may stay as they are, or follow what your language's
  community usually says.
- Text like `data/`, `tkmm_mods.txt`, `--profile` or `script_extender.cfg` is a folder, a file or
  a command: keep it as it is.

## Try your translation without building

1. Install the latest TK Mod Manager.
2. Put your language folder in `%APPDATA%\TKModManager\locales\`, so that you have
   `%APPDATA%\TKModManager\locales\de\main.ftl` and so on. (Press Win+R, type
   `%APPDATA%\TKModManager` and press Enter to open the folder.)
3. Start the app. Under **Settings > Behaviour > Language**, pick your language and restart.

Files in that folder are read at every start, and they take priority over the translations built
into the app. Edit, restart, check. When you're done, delete the folder so your copy picks up
later updates of the built-in translation.

If a message shows its key (such as `mods-count`) instead of text, the line above it probably
has a mistake, such as a missing `}` or an unindented second line.

## Send it in

Open a pull request on [GitHub](https://github.com/Ironictw2st/TKModManager) that adds your
folder under `crates/core/locales/`. If you don't use GitHub, open an issue there with your folder
attached as a zip. Once it is merged, the next
release includes it, and players can pick it on first start or in Settings.

The automatic checks on a pull request make sure that every file parses, that `language-name`
is set, that no key is unknown (renamed or misspelt), and that no message uses a variable
English doesn't have.

## Keeping a translation up to date

New versions add and change English messages. To see what's missing in your language (this needs
[Rust](https://rustup.rs)):

```
cargo run -p tkmm_core --example i18n_status -- -v
```

The command prints how complete each language is and lists the keys that are missing or no
longer exist. Your drop-in folder in `%APPDATA%` is included. Without Rust, compare your folder
with `en` on GitHub.

## For developers

- Write user-facing text with `t!("key")` or `t!("key", count = n, name = text)`. The macro is in
  `tkmm_core` (`crates/core/src/i18n.rs`) and is available everywhere in the app crate. Add the
  English message to the matching `en/*.ftl` file.
- Write whole sentences with variables. Don't glue fragments together, and don't make plurals by
  hand (`if n == 1`): use a Fluent selector.
- Never translate anything that is parsed or written for the game: profile exports, list-file
  directives, settings values, `script_extender.cfg`, log markers, file names.
- `cargo test -p tkmm_core` fails on a `t!` key missing from `en`, on an `en` message no code
  uses, and on the translation errors listed above.
- Keys built at run time (`month-1`…`month-12`) are listed in `DYNAMIC_PREFIXES` in that
  test.
- A new language folder is picked up at build time (`crates/core/build.rs`). `release.ps1`
  also ships Qt's own `qtbase_<code>.qm`, so that standard buttons like Yes/No/Cancel are
  translated as well.
