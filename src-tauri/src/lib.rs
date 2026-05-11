use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use tauri::{AppHandle, Manager};

const DEFAULT_NOTE_CONTENT: &str = "# Welcome to Lithe\n\nStart with plain Markdown. Stay local. Add power only when you ask for it.\n\n- Fast, quiet editing\n- Portable text\n- Future plugin hooks\n";

#[derive(Serialize)]
struct SpaceInfo {
    name: String,
    path: String,
}

#[derive(Serialize, Clone)]
struct NoteEntry {
    name: String,
    path: String,
}

#[derive(Serialize)]
struct SpaceSnapshot {
    space: SpaceInfo,
    notes: Vec<NoteEntry>,
    active_note: NoteEntry,
    content: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct CaptureMetadataRecord {
    id: String,
    path: String,
    start_line: usize,
    end_line: usize,
    created_at: String,
    year: i32,
    month: u32,
    day: u32,
    date: String,
    weekday: String,
    time: String,
}

#[derive(Serialize, Deserialize)]
struct CaptureMetadataStore {
    version: u8,
    records: Vec<CaptureMetadataRecord>,
}

fn default_space_path(app: &AppHandle) -> Result<PathBuf, String> {
    let documents_dir = app
        .path()
        .document_dir()
        .map_err(|error| error.to_string())?;

    Ok(documents_dir.join("Lithe"))
}

fn app_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join("current-space.txt"))
}

fn current_space_path(app: &AppHandle) -> Result<PathBuf, String> {
    let state_path = app_state_path(app)?;

    if state_path.exists() {
        let path = fs::read_to_string(state_path).map_err(|error| error.to_string())?;
        let trimmed_path = path.trim();

        if !trimmed_path.is_empty() {
            return Ok(PathBuf::from(trimmed_path));
        }
    }

    default_space_path(app)
}

fn default_note_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(current_space_path(app)?.join("default.md"))
}

fn capture_metadata_path(space_path: &Path) -> PathBuf {
    space_path.join(".lithe").join("captures.json")
}

fn ensure_space_path(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path.join(".lithe")).map_err(|error| error.to_string())
}

fn ensure_current_space(app: &AppHandle) -> Result<PathBuf, String> {
    let space_path = current_space_path(app)?;
    ensure_space_path(&space_path)?;

    Ok(space_path)
}

fn space_info(path: &Path) -> SpaceInfo {
    SpaceInfo {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Lithe")
            .to_string(),
        path: path.to_string_lossy().to_string(),
    }
}

fn note_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

fn relative_path(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|error| error.to_string())?;

    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/"))
}

fn collect_notes(root: &Path, dir: &Path, notes: &mut Vec<NoteEntry>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        if file_name == ".lithe" {
            continue;
        }

        if path.is_dir() {
            collect_notes(root, &path, notes)?;
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }

        notes.push(NoteEntry {
            name: note_name_from_path(&path),
            path: relative_path(root, &path)?,
        });
    }

    Ok(())
}

fn list_notes_in_space(space_path: &Path) -> Result<Vec<NoteEntry>, String> {
    let mut notes = Vec::new();

    collect_notes(space_path, space_path, &mut notes)?;
    notes.sort_by(|first, second| first.path.cmp(&second.path));

    Ok(notes)
}

fn ensure_default_note(space_path: &Path) -> Result<NoteEntry, String> {
    let mut notes = list_notes_in_space(space_path)?;

    if let Some(note) = notes.pop() {
        return Ok(note);
    }

    let default_path = space_path.join("default.md");
    fs::write(&default_path, DEFAULT_NOTE_CONTENT).map_err(|error| error.to_string())?;

    Ok(NoteEntry {
        name: "default".to_string(),
        path: "default.md".to_string(),
    })
}

fn resolve_note_path(space_path: &Path, relative_note_path: &str) -> Result<PathBuf, String> {
    let relative_path = Path::new(relative_note_path);

    if relative_path.is_absolute() {
        return Err("Note path must be relative to the current Space.".to_string());
    }

    for component in relative_path.components() {
        if !matches!(component, Component::Normal(_)) {
            return Err("Note path contains unsupported path components.".to_string());
        }
    }

    if relative_path.extension().and_then(|extension| extension.to_str()) != Some("md") {
        return Err("Note path must point to a Markdown file.".to_string());
    }

    Ok(space_path.join(relative_path))
}

fn safe_note_file_name(name: &str) -> String {
    let stem = name
        .trim()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ' ') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");

    if stem.is_empty() {
        "Untitled".to_string()
    } else {
        stem
    }
}

fn unique_note_path(space_path: &Path, name: &str) -> PathBuf {
    let stem = safe_note_file_name(name);
    let mut candidate = space_path.join(format!("{stem}.md"));
    let mut counter = 2;

    while candidate.exists() {
        candidate = space_path.join(format!("{stem}-{counter}.md"));
        counter += 1;
    }

    candidate
}

fn snapshot_for_note(app: &AppHandle, note: NoteEntry) -> Result<SpaceSnapshot, String> {
    let space_path = ensure_current_space(app)?;
    let note_path = resolve_note_path(&space_path, &note.path)?;
    let content = fs::read_to_string(note_path).map_err(|error| error.to_string())?;

    Ok(SpaceSnapshot {
        space: space_info(&space_path),
        notes: list_notes_in_space(&space_path)?,
        active_note: note,
        content,
    })
}

fn read_capture_metadata(path: &Path) -> Result<CaptureMetadataStore, String> {
    if !path.exists() {
        return Ok(CaptureMetadataStore {
            version: 1,
            records: Vec::new(),
        });
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_json::from_str(&content).map_err(|error| error.to_string())
}

#[tauri::command]
fn load_space(app: AppHandle) -> Result<SpaceSnapshot, String> {
    let space_path = ensure_current_space(&app)?;
    let note = ensure_default_note(&space_path)?;

    snapshot_for_note(&app, note)
}

#[tauri::command]
fn open_space(app: AppHandle, path: String) -> Result<SpaceSnapshot, String> {
    let space_path = PathBuf::from(path);
    ensure_space_path(&space_path)?;

    let state_path = app_state_path(&app)?;
    if let Some(parent) = state_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(state_path, space_path.to_string_lossy().to_string()).map_err(|error| error.to_string())?;

    let note = ensure_default_note(&space_path)?;
    snapshot_for_note(&app, note)
}

#[tauri::command]
fn create_note(app: AppHandle, name: String) -> Result<SpaceSnapshot, String> {
    let space_path = ensure_current_space(&app)?;
    let note_path = unique_note_path(&space_path, &name);
    fs::write(&note_path, "").map_err(|error| error.to_string())?;

    let note = NoteEntry {
        name: note_name_from_path(&note_path),
        path: relative_path(&space_path, &note_path)?,
    };

    snapshot_for_note(&app, note)
}

#[tauri::command]
fn read_note(app: AppHandle, path: String) -> Result<SpaceSnapshot, String> {
    let space_path = ensure_current_space(&app)?;
    let note_path = resolve_note_path(&space_path, &path)?;
    let note = NoteEntry {
        name: note_name_from_path(&note_path),
        path: relative_path(&space_path, &note_path)?,
    };

    snapshot_for_note(&app, note)
}

#[tauri::command]
fn write_note(app: AppHandle, path: String, content: String) -> Result<(), String> {
    let space_path = ensure_current_space(&app)?;
    let note_path = resolve_note_path(&space_path, &path)?;

    if let Some(parent) = note_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    fs::write(note_path, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn record_capture_metadata(app: AppHandle, record: CaptureMetadataRecord) -> Result<(), String> {
    let space_path = ensure_current_space(&app)?;
    let metadata_path = capture_metadata_path(&space_path);
    let mut metadata = read_capture_metadata(&metadata_path)?;

    metadata.records.push(record);

    if let Some(parent) = metadata_path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    let content = serde_json::to_string_pretty(&metadata).map_err(|error| error.to_string())?;
    fs::write(metadata_path, content).map_err(|error| error.to_string())
}

#[tauri::command]
fn delete_note(app: AppHandle, path: String) -> Result<SpaceSnapshot, String> {
    let space_path = ensure_current_space(&app)?;
    let note_path = resolve_note_path(&space_path, &path)?;

    if note_path.exists() {
        fs::remove_file(note_path).map_err(|error| error.to_string())?;
    }

    let note = ensure_default_note(&space_path)?;
    snapshot_for_note(&app, note)
}

#[tauri::command]
fn load_default_document(app: AppHandle) -> Result<Option<String>, String> {
    ensure_current_space(&app)?;
    let path = default_note_path(&app)?;

    if !path.exists() {
        return Ok(None);
    }

    fs::read_to_string(path).map(Some).map_err(|error| error.to_string())
}

#[tauri::command]
fn save_default_document(app: AppHandle, content: String) -> Result<(), String> {
    ensure_current_space(&app)?;
    let path = default_note_path(&app)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Default note path does not have a parent directory.".to_string())?;

    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, content).map_err(|error| error.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            load_space,
            open_space,
            create_note,
            read_note,
            write_note,
            record_capture_metadata,
            delete_note,
            load_default_document,
            save_default_document
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lithe");
}
