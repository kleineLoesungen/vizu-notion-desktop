//! Der Notion-Token.
//!
//! Wo er herkommt, in dieser Reihenfolge:
//!
//! 1. `VIZU_NOTION_TOKEN` — für Skripte, CI und zum Ausprobieren.
//! 2. Der Speicher der Anwendung ([`SecretStore`]): der Schlüsselbund des
//!    Systems, oder eine Datei, wenn `VIZU_NOTION_TOKEN_FILE` gesetzt ist
//!    (Rechner ohne Schlüsselbund, Tests).
//!
//! Der Token wird nie ausgegeben und nie protokolliert. Angezeigt wird höchstens
//! ein [`hint`]: Anfang und die letzten vier Zeichen.

use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use ts_rs::TS;

use crate::error::{Error, Result, Validator};
use crate::paths::APP_NAME;

pub const ENV_TOKEN: &str = "VIZU_NOTION_TOKEN";
pub const ENV_TOKEN_FILE: &str = "VIZU_NOTION_TOKEN_FILE";

/// Unter diesem Konto steht der Token im Schlüsselbund (Dienst: `vizu-notion`).
const KEYCHAIN_ACCOUNT: &str = "notion-token";

/// Kürzer ist kein Notion-Token. Fängt vor allem versehentlich eingefügte
/// Wörter ab.
const TOKEN_MIN: usize = 20;

pub trait SecretStore: Send + Sync {
    fn load(&self) -> Result<Option<String>>;
    fn save(&self, token: &str) -> Result<()>;
    /// `true`, wenn etwas gelöscht wurde.
    fn delete(&self) -> Result<bool>;
    /// Für Menschen: „Schlüsselbund", „Datei …".
    fn describe(&self) -> String;
}

/// Der passende Speicher für diesen Rechner.
pub fn default_store() -> Box<dyn SecretStore> {
    match std::env::var_os(ENV_TOKEN_FILE) {
        Some(path) => Box::new(FileStore::new(PathBuf::from(path))),
        None => Box::new(Keychain),
    }
}

/// Keychain auf macOS, Secret Service auf Linux.
///
/// Auf macOS fragt das System beim ersten Lesen durch ein anderes Programm
/// nach — liest die Desktop-Anwendung einen Token, den die Kommandozeile
/// gespeichert hat. „Immer erlauben" beantwortet das dauerhaft.
#[derive(Debug, Default)]
pub struct Keychain;

impl Keychain {
    fn entry() -> Result<keyring::Entry> {
        keyring::Entry::new(APP_NAME, KEYCHAIN_ACCOUNT).map_err(store_error)
    }
}

impl SecretStore for Keychain {
    fn load(&self) -> Result<Option<String>> {
        match Self::entry()?.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(store_error(e)),
        }
    }

    fn save(&self, token: &str) -> Result<()> {
        Self::entry()?.set_password(token).map_err(store_error)
    }

    fn delete(&self) -> Result<bool> {
        match Self::entry()?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(store_error(e)),
        }
    }

    fn describe(&self) -> String {
        "Schlüsselbund".to_string()
    }
}

fn store_error(e: keyring::Error) -> Error {
    Error::SecretStore(e.to_string())
}

/// Eine Datei mit nichts als dem Token, nur für den Besitzer lesbar.
#[derive(Debug)]
pub struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn io(&self, source: std::io::Error) -> Error {
        Error::Io {
            path: self.path.clone(),
            source,
        }
    }
}

impl SecretStore for FileStore {
    fn load(&self) -> Result<Option<String>> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ok(Some(text.trim().to_string()).filter(|t| !t.is_empty())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(self.io(e)),
        }
    }

    fn save(&self, token: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| self.io(e))?;
        }
        write_private(&self.path, token).map_err(|e| self.io(e))
    }

    fn delete(&self) -> Result<bool> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(self.io(e)),
        }
    }

    fn describe(&self) -> String {
        format!("Datei {}", self.path.display())
    }
}

#[cfg(unix)]
fn write_private(path: &std::path::Path, token: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(token.as_bytes())
}

#[cfg(not(unix))]
fn write_private(path: &std::path::Path, token: &str) -> std::io::Result<()> {
    std::fs::write(path, token)
}

/// Nur im Arbeitsspeicher. Für Tests und [`crate::App::in_memory`].
#[derive(Debug, Default)]
pub struct MemoryStore(Mutex<Option<String>>);

impl SecretStore for MemoryStore {
    fn load(&self) -> Result<Option<String>> {
        Ok(self.0.lock().unwrap_or_else(|e| e.into_inner()).clone())
    }

    fn save(&self, token: &str) -> Result<()> {
        *self.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(token.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<bool> {
        Ok(self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .is_some())
    }

    fn describe(&self) -> String {
        "Arbeitsspeicher".to_string()
    }
}

/// Ein geteilter Speicher ist selbst einer.
///
/// Damit kann ein Test denselben Speicher behalten, den er der Anwendung
/// mitgibt, und hineinsehen.
impl<T: SecretStore + ?Sized> SecretStore for std::sync::Arc<T> {
    fn load(&self) -> Result<Option<String>> {
        self.as_ref().load()
    }
    fn save(&self, token: &str) -> Result<()> {
        self.as_ref().save(token)
    }
    fn delete(&self) -> Result<bool> {
        self.as_ref().delete()
    }
    fn describe(&self) -> String {
        self.as_ref().describe()
    }
}

// --- Fachliche Funktionen ----------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TokenOrigin {
    Environment,
    Store,
}

/// Was sich über den Token sagen lässt, ohne ihn zu zeigen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct TokenStatus {
    /// `None`: kein Token.
    pub origin: Option<TokenOrigin>,
    /// Z. B. `ntn_…a1b2`.
    pub hint: Option<String>,
    /// Wo die Anwendung speichert.
    pub store: String,
    /// Der Speicher ließ sich nicht lesen.
    pub store_error: Option<String>,
}

/// Räumt auf und prüft. Gibt den bereinigten Token zurück.
pub fn clean_token(raw: &str) -> Result<String> {
    let token = raw.trim().to_string();
    let mut v = Validator::new();
    if token.is_empty() {
        v.add("token", "darf nicht leer sein");
    } else if token.chars().any(char::is_whitespace) {
        v.add("token", "enthält Leerzeichen — nur den Token einfügen");
    } else if token.chars().count() < TOKEN_MIN {
        v.add(
            "token",
            "zu kurz für einen Notion-Token (beginnt meist mit ntn_ oder secret_)",
        );
    }
    v.finish()?;
    Ok(token)
}

pub fn set(store: &dyn SecretStore, raw: &str) -> Result<TokenStatus> {
    let token = clean_token(raw)?;
    store.save(&token)?;
    Ok(status_with(store, None))
}

/// Löscht den gespeicherten Token. `VIZU_NOTION_TOKEN` bleibt davon unberührt.
pub fn clear(store: &dyn SecretStore) -> Result<bool> {
    store.delete()
}

pub fn status(store: &dyn SecretStore) -> TokenStatus {
    status_with(store, env_token())
}

/// Der Token für einen Abruf.
pub fn resolve(store: &dyn SecretStore) -> Result<String> {
    resolve_with(store, env_token())
}

/// Wie [`resolve`], mit ausdrücklich übergebener Umgebung — für Tests, die
/// keine Prozessvariable setzen dürfen.
pub fn resolve_with(store: &dyn SecretStore, env: Option<String>) -> Result<String> {
    if let Some(token) = env {
        return Ok(token);
    }
    match store.load() {
        Ok(Some(token)) => Ok(token),
        Ok(None) => Err(Error::NoToken(None)),
        Err(e) => Err(Error::NoToken(Some(e.to_string()))),
    }
}

pub fn status_with(store: &dyn SecretStore, env: Option<String>) -> TokenStatus {
    let describe = store.describe();
    if let Some(token) = env {
        return TokenStatus {
            origin: Some(TokenOrigin::Environment),
            hint: Some(hint(&token)),
            store: describe,
            store_error: None,
        };
    }
    match store.load() {
        Ok(token) => TokenStatus {
            origin: token.as_ref().map(|_| TokenOrigin::Store),
            hint: token.as_deref().map(hint),
            store: describe,
            store_error: None,
        },
        Err(e) => TokenStatus {
            origin: None,
            hint: None,
            store: describe,
            store_error: Some(e.to_string()),
        },
    }
}

/// `VIZU_NOTION_TOKEN`, falls gesetzt. Gelesen wird das nur beim Start —
/// siehe [`crate::App::token`].
pub fn env_token() -> Option<String> {
    std::env::var(ENV_TOKEN)
        .ok()
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Anfang bis zum ersten `_` (höchstens sieben Zeichen) und die letzten vier.
pub fn hint(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    if chars.len() < 12 {
        return "…".to_string();
    }
    let prefix: String = match token.find('_') {
        Some(i) if i < 7 => token[..=i].to_string(),
        _ => String::new(),
    };
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{prefix}…{tail}")
}
