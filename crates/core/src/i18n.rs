//! UI translations. Every language is a folder of Fluent files, `crates/core/locales/<code>/`,
//! compiled in by build.rs; `en` is the source of truth and the fallback for any missing message.
//! Translators can also drop a folder into `%APPDATA%\TKModManager\locales\<code>\` to try it
//! without building: those files add to (and override) the compiled-in ones.
//!
//! Call `init` once at startup, before any text is made; `t!` looks messages up afterwards.
//! See TRANSLATING.md.

use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::FluentResource;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

pub use fluent_bundle::{FluentArgs, FluentValue};

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

/// The source language: always complete, used when a message is missing elsewhere.
pub const SOURCE: &str = "en";

type Bundle = FluentBundle<FluentResource>;

struct State {
    code: String,
    chosen: Option<Bundle>,
    en: Bundle,
}

static STATE: RwLock<Option<Arc<State>>> = RwLock::new(None);

/// `%APPDATA%\TKModManager\locales`: drop-in translations, read at startup.
pub fn dropin_dir() -> PathBuf {
    crate::paths::app_data_dir().join("locales")
}

/// (file name, Fluent source) of every file of a language: compiled in first, then drop-ins.
fn sources(code: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = LOCALES
        .iter()
        .filter(|(c, _)| c.eq_ignore_ascii_case(code))
        .flat_map(|(_, files)| files.iter().map(|(n, s)| (n.to_string(), s.to_string())))
        .collect();
    let dir = dropin_dir().join(code);
    if let Ok(rd) = std::fs::read_dir(&dir) {
        let mut files: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("ftl")))
            .collect();
        files.sort();
        for f in files {
            if let Ok(text) = std::fs::read_to_string(&f) {
                let name = f.file_name().unwrap_or_default().to_string_lossy().into_owned();
                out.push((name, text.trim_start_matches('\u{feff}').to_string()));
            }
        }
    }
    out
}

fn bundle(code: &str) -> Option<Bundle> {
    let files = sources(code);
    if files.is_empty() {
        return None;
    }
    let lang = code.parse().unwrap_or_else(|_| SOURCE.parse().unwrap());
    let mut b = FluentBundle::new_concurrent(vec![lang]);
    // Bidi isolation marks around placeables show up as boxes in Qt labels.
    b.set_use_isolating(false);
    for (_, text) in files {
        // A file with mistakes still loads the messages that parsed.
        let res = FluentResource::try_new(text).unwrap_or_else(|(res, _)| res);
        b.add_resource_overriding(res);
    }
    Some(b)
}

/// Load the language to show (a code from `available`, e.g. "de" or "pt-BR"). Unknown codes and
/// "" fall back to English. Called again, it replaces the previous choice.
pub fn init(code: &str) {
    let en = bundle(SOURCE).expect("the English translation is compiled in");
    let chosen = if code.is_empty() || code.eq_ignore_ascii_case(SOURCE) { None } else { bundle(code) };
    let code = if chosen.is_some() { code.to_string() } else { SOURCE.to_string() };
    *STATE.write().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(State { code, chosen, en }));
}

/// The language in use ("en" until `init` picks another).
pub fn current() -> String {
    state().code.clone()
}

fn state() -> Arc<State> {
    if let Some(s) = STATE.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
        return s.clone();
    }
    init(SOURCE);
    STATE.read().unwrap_or_else(|e| e.into_inner()).clone().expect("initialised above")
}

/// The text of `key` in the chosen language, then English, then the key itself.
pub fn tr(key: &str, args: Option<&FluentArgs>) -> String {
    let st = state();
    for b in st.chosen.iter().chain(std::iter::once(&st.en)) {
        if let Some(pattern) = b.get_message(key).and_then(|m| m.value()) {
            let mut errors = Vec::new();
            return b.format_pattern(pattern, args, &mut errors).into_owned();
        }
    }
    key.to_string()
}

/// Values that `t!` accepts as message arguments.
pub trait Arg {
    fn into_value(self) -> FluentValue<'static>;
}

impl Arg for String {
    fn into_value(self) -> FluentValue<'static> {
        FluentValue::from(self)
    }
}
impl Arg for &String {
    fn into_value(self) -> FluentValue<'static> {
        FluentValue::from(self.clone())
    }
}
impl Arg for &str {
    fn into_value(self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}
impl Arg for &&str {
    fn into_value(self) -> FluentValue<'static> {
        FluentValue::from(self.to_string())
    }
}
macro_rules! num_arg {
    ($($t:ty),*) => {$(
        impl Arg for $t {
            fn into_value(self) -> FluentValue<'static> {
                FluentValue::from(self)
            }
        }
        impl Arg for &$t {
            fn into_value(self) -> FluentValue<'static> {
                FluentValue::from(*self)
            }
        }
    )*};
}
num_arg!(i32, i64, u32, u64, usize, f64);

/// `t!("key")` or `t!("key", count = n, name = text)`: the translated text as a `String`.
/// Argument names are the Fluent variables (`{ $count }`, `{ $name }`).
#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::tr($key, None)
    };
    ($key:expr, $($name:ident = $value:expr),+ $(,)?) => {{
        let mut args = $crate::i18n::FluentArgs::new();
        $( args.set(stringify!($name), $crate::i18n::Arg::into_value($value)); )+
        $crate::i18n::tr($key, Some(&args))
    }};
}

/// Every language there is (compiled in or dropped in), as (code, name in that language),
/// English first and the rest by name.
pub fn available() -> Vec<(String, String)> {
    let mut codes: Vec<String> = LOCALES.iter().map(|(c, _)| c.to_string()).collect();
    if let Ok(rd) = std::fs::read_dir(dropin_dir()) {
        for e in rd.flatten() {
            let code = e.file_name().to_string_lossy().into_owned();
            if e.path().is_dir() && !codes.iter().any(|c| c.eq_ignore_ascii_case(&code)) {
                codes.push(code);
            }
        }
    }
    let mut out: Vec<(String, String)> = codes
        .into_iter()
        .filter_map(|code| {
            let b = bundle(&code)?;
            let name = b
                .get_message("language-name")
                .and_then(|m| m.value())
                .map(|p| b.format_pattern(p, None, &mut Vec::new()).into_owned())
                .unwrap_or_else(|| code.clone());
            Some((code, name))
        })
        .collect();
    out.sort_by(|a, b| (a.0 != SOURCE).cmp(&(b.0 != SOURCE)).then_with(|| a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    out
}

/// The available language that best matches Windows' display language ("de-AT" → "de").
pub fn system_default() -> String {
    let langs: Vec<String> = available().into_iter().map(|(c, _)| c).collect();
    best_match(&system_locale(), &langs).unwrap_or_else(|| SOURCE.to_string())
}

fn best_match(locale: &str, langs: &[String]) -> Option<String> {
    let locale = locale.replace('_', "-");
    if let Some(c) = langs.iter().find(|c| c.eq_ignore_ascii_case(&locale)) {
        return Some(c.clone());
    }
    let primary = locale.split('-').next().unwrap_or("");
    langs
        .iter()
        .find(|c| c.eq_ignore_ascii_case(primary))
        .or_else(|| langs.iter().find(|c| c.split('-').next().is_some_and(|p| p.eq_ignore_ascii_case(primary))))
        .cloned()
}

#[cfg(windows)]
fn system_locale() -> String {
    #[link(name = "kernel32")]
    extern "system" {
        fn GetUserDefaultLocaleName(name: *mut u16, len: i32) -> i32;
    }
    let mut buf = [0u16; 85];
    // SAFETY: the buffer is LOCALE_NAME_MAX_LENGTH (85) wide characters, as the API requires.
    let n = unsafe { GetUserDefaultLocaleName(buf.as_mut_ptr(), buf.len() as i32) };
    if n <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..(n - 1) as usize])
}

#[cfg(not(windows))]
fn system_locale() -> String {
    std::env::var("LANG").unwrap_or_default().split('.').next().unwrap_or("").to_string()
}

/// How complete a language is: (messages translated, messages in English, missing keys).
pub struct Coverage {
    pub code: String,
    pub name: String,
    pub translated: usize,
    pub total: usize,
    pub missing: Vec<String>,
    /// Keys the language has that English doesn't (renamed or removed messages).
    pub unknown: Vec<String>,
}

/// Message ids (not terms) defined by a set of Fluent files.
fn message_ids(files: &[(String, String)]) -> Vec<String> {
    let mut ids = Vec::new();
    for (_, text) in files {
        let res = match fluent_syntax::parser::parse(text.as_str()) {
            Ok(r) => r,
            Err((r, _)) => r,
        };
        for entry in res.body {
            if let fluent_syntax::ast::Entry::Message(m) = entry {
                ids.push(m.id.name.to_string());
            }
        }
    }
    ids
}

/// Coverage of every non-English language, for `examples/i18n_status.rs` and the language list.
pub fn coverage() -> Vec<Coverage> {
    let en: Vec<String> = message_ids(&sources(SOURCE));
    available()
        .into_iter()
        .filter(|(c, _)| c != SOURCE)
        .map(|(code, name)| {
            let have: BTreeMap<String, ()> = message_ids(&sources(&code)).into_iter().map(|k| (k, ())).collect();
            let missing: Vec<String> = en.iter().filter(|k| !have.contains_key(*k)).cloned().collect();
            let unknown: Vec<String> = have.keys().filter(|k| !en.contains(k)).cloned().collect();
            Coverage { code, name, translated: en.len() - missing.len(), total: en.len(), missing, unknown }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use fluent_syntax::ast;
    use std::collections::{BTreeSet, HashMap};

    /// Messages looked up with a key built at run time; the test can't see them in the source.
    const DYNAMIC_PREFIXES: &[&str] = &["language-name", "month-"];

    fn parse(code: &str) -> HashMap<String, ast::Message<String>> {
        let mut out = HashMap::new();
        for (name, text) in LOCALES.iter().filter(|(c, _)| *c == code).flat_map(|(_, f)| f.iter()) {
            let res = fluent_syntax::parser::parse(text.to_string())
                .unwrap_or_else(|(_, errs)| panic!("{code}/{name} has Fluent syntax errors: {errs:?}"));
            for e in res.body {
                if let ast::Entry::Message(m) = e {
                    let id = m.id.name.clone();
                    assert!(out.insert(id.clone(), m).is_none(), "{code}/{name}: message {id} is defined twice");
                }
            }
        }
        out
    }

    fn vars_in_pattern(p: &ast::Pattern<String>, out: &mut BTreeSet<String>) {
        for el in &p.elements {
            if let ast::PatternElement::Placeable { expression } = el {
                vars_in_expr(expression, out);
            }
        }
    }

    fn vars_in_expr(e: &ast::Expression<String>, out: &mut BTreeSet<String>) {
        match e {
            ast::Expression::Select { selector, variants } => {
                vars_in_inline(selector, out);
                for v in variants {
                    vars_in_pattern(&v.value, out);
                }
            }
            ast::Expression::Inline(i) => vars_in_inline(i, out),
        }
    }

    fn vars_in_inline(i: &ast::InlineExpression<String>, out: &mut BTreeSet<String>) {
        match i {
            ast::InlineExpression::VariableReference { id } => {
                out.insert(id.name.clone());
            }
            ast::InlineExpression::FunctionReference { arguments, .. } => {
                for a in &arguments.positional {
                    vars_in_inline(a, out);
                }
            }
            ast::InlineExpression::Placeable { expression } => vars_in_expr(expression, out),
            _ => {}
        }
    }

    fn vars(m: &ast::Message<String>) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        if let Some(p) = &m.value {
            vars_in_pattern(p, &mut out);
        }
        out
    }

    #[test]
    fn every_language_parses_and_matches_english() {
        let en = parse(SOURCE);
        assert!(en.contains_key("language-name"));
        for (code, _) in LOCALES.iter().filter(|(c, _)| *c != SOURCE) {
            let lang = parse(code);
            assert!(lang.contains_key("language-name"), "{code} has no language-name");
            for (id, m) in &lang {
                let base = en.get(id).unwrap_or_else(|| panic!("{code}: {id} is not an English message (renamed or removed?)"));
                let extra: Vec<_> = vars(m).difference(&vars(base)).cloned().collect();
                assert!(extra.is_empty(), "{code}: {id} uses variables English doesn't have: {extra:?}");
            }
        }
    }

    /// Keys of every `t!("…")` in the workspace sources.
    fn keys_in_source() -> BTreeSet<String> {
        let crates = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut stack = vec![crates];
        let mut keys = BTreeSet::new();
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    if p.file_name().is_some_and(|n| n != "target" && n != "locales") {
                        stack.push(p);
                    }
                } else if p.extension().is_some_and(|x| x == "rs") {
                    let file = std::fs::read_to_string(&p).unwrap();
                    // Comments may show `t!("key")` as an example.
                    let code: Vec<&str> = file.lines().filter(|l| !l.trim_start().starts_with("//")).collect();
                    let text = code.join("
");
                    for (i, _) in text.match_indices("t!(") {
                        // Not the end of another macro name (`format!(`, `print!(`).
                        if text[..i].chars().last().is_some_and(|c| c.is_alphanumeric() || c == '_') {
                            continue;
                        }
                        // rustfmt may put the key on the next line.
                        let Some(rest) = text[i + 3..].trim_start().strip_prefix('"') else { continue };
                        let key = &rest[..rest.find('"').unwrap_or(0)];
                        // A Fluent id; anything else is this scanner's own string literal.
                        if key.starts_with(|c: char| c.is_ascii_alphabetic()) && key.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                            keys.insert(key.to_string());
                        }
                    }
                }
            }
        }
        keys
    }

    #[test]
    fn source_and_english_agree() {
        let en = parse(SOURCE);
        let used = keys_in_source();
        let missing: Vec<_> = used.iter().filter(|k| !en.contains_key(*k)).collect();
        assert!(missing.is_empty(), "t!() keys missing from locales/en: {missing:?}");
        let unused: Vec<_> = en
            .keys()
            .filter(|k| !used.contains(*k) && !DYNAMIC_PREFIXES.iter().any(|p| k.starts_with(p)))
            .collect();
        assert!(unused.is_empty(), "locales/en messages no code uses: {unused:?}");
    }

    #[test]
    fn lookups_fall_back_and_format() {
        assert_eq!(tr("no-such-message", None), "no-such-message");
        assert_eq!(tr("language-name", None), "English");
    }

    #[test]
    fn locale_matching() {
        let langs = vec!["en".to_string(), "de".to_string(), "pt-BR".to_string()];
        assert_eq!(best_match("de-AT", &langs).as_deref(), Some("de"));
        assert_eq!(best_match("pt-PT", &langs).as_deref(), Some("pt-BR"));
        assert_eq!(best_match("pt_BR", &langs).as_deref(), Some("pt-BR"));
        assert_eq!(best_match("fr-FR", &langs), None);
    }
}
