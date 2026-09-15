// Liest tauri.conf.json und capabilities/, erzeugt die Berechtigungsschemata
// in gen/schemas und bettet das Symbol ein. Ohne diese Datei kompiliert
// `tauri::generate_context!` nicht.
fn main() {
    tauri_build::build();
}
