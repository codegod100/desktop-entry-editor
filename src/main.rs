use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use slint::{Model, ModelRc, SharedString, VecModel};

slint::include_modules!();

const STANDARD_PATHS: &[&str] = &[
    "/usr/share/applications",
    "/usr/local/share/applications",
    "/var/lib/flatpak/exports/share/applications",
];

fn home_applications() -> PathBuf {
    dirs_home().join(".local/share/applications")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

/// Parsed .desktop file
#[derive(Clone, Debug)]
struct DesktopEntryData {
    path: PathBuf,
    keys: Vec<String>,
    values: Vec<String>,
}

impl DesktopEntryData {
    fn from_file(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        let mut keys = Vec::new();
        let mut values = Vec::new();
        let mut in_desktop_entry = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if trimmed == "[Desktop Entry]" {
                in_desktop_entry = true;
                continue;
            }
            if trimmed.starts_with('[') {
                in_desktop_entry = false;
                continue;
            }
            if in_desktop_entry {
                if let Some((k, v)) = trimmed.split_once('=') {
                    keys.push(k.to_string());
                    values.push(v.to_string());
                }
            }
        }

        Some(Self {
            path: path.to_path_buf(),
            keys,
            values,
        })
    }

    fn new_empty() -> Self {
        let home_apps = home_applications();
        let _ = fs::create_dir_all(&home_apps);
        Self {
            path: home_apps.join("my-application.desktop"),
            keys: vec![
                "Type".into(),
                "Name".into(),
                "Comment".into(),
                "Exec".into(),
                "Icon".into(),
                "Terminal".into(),
                "Categories".into(),
                "StartupNotify".into(),
            ],
            values: vec![
                "Application".into(),
                "My Application".into(),
                "A custom application".into(),
                "".into(),
                "".into(),
                "false".into(),
                "".into(),
                "true".into(),
            ],
        }
    }

    fn get(&self, key: &str) -> String {
        self.keys.iter().position(|k| k == key)
            .and_then(|i| self.values.get(i).cloned())
            .unwrap_or_default()
    }

    fn set(&mut self, key: &str, value: &str) {
        if let Some(i) = self.keys.iter().position(|k| k == key) {
            self.values[i] = value.to_string();
        } else {
            self.keys.push(key.to_string());
            self.values.push(value.to_string());
        }
    }

    fn to_file_data(&self) -> DesktopFileData {
        DesktopFileData {
            path: self.path.to_string_lossy().to_string().into(),
            is_new: !self.path.exists(),
            has_changes: false,
            name: self.get("Name").into(),
            generic_name: self.get("GenericName").into(),
            comment: self.get("Comment").into(),
            icon: self.get("Icon").into(),
            exec: self.get("Exec").into(),
            try_exec: self.get("TryExec").into(),
            desktop_type: self.get("Type").into(),
            categories: self.get("Categories").into(),
            mime_types: self.get("MimeType").into(),
            keywords: self.get("Keywords").into(),
            startup_wm_class: self.get("StartupWMClass").into(),
            terminal: self.get("Terminal").to_lowercase() == "true",
            startup_notify: self.get("StartupNotify").to_lowercase() == "true",
            no_display: self.get("NoDisplay").to_lowercase() == "true",
            hidden: self.get("Hidden").to_lowercase() == "true",
            dbus_activatable: self.get("DBusActivatable").to_lowercase() == "true",
            working_dir: self.get("Path").into(),
            only_show_in: self.get("OnlyShowIn").into(),
            not_show_in: self.get("NotShowIn").into(),
            actions: self.get("Actions").into(),
            implements: self.get("Implements").into(),
            raw_keys: ModelRc::new(VecModel::from(
                self.keys.iter().cloned().map(SharedString::from).collect::<Vec<_>>()
            )),
            raw_values: ModelRc::new(VecModel::from(
                self.values.iter().cloned().map(SharedString::from).collect::<Vec<_>>()
            )),
        }
    }

    fn apply_file_data(&mut self, data: &DesktopFileData) {
        let updates: [(&str, &str); 21] = [
            ("Name", &data.name),
            ("GenericName", &data.generic_name),
            ("Comment", &data.comment),
            ("Icon", &data.icon),
            ("Exec", &data.exec),
            ("TryExec", &data.try_exec),
            ("Type", &data.desktop_type),
            ("Categories", &data.categories),
            ("MimeType", &data.mime_types),
            ("Keywords", &data.keywords),
            ("StartupWMClass", &data.startup_wm_class),
            ("Terminal", if data.terminal { "true" } else { "false" }),
            ("StartupNotify", if data.startup_notify { "true" } else { "false" }),
            ("NoDisplay", if data.no_display { "true" } else { "false" }),
            ("Hidden", if data.hidden { "true" } else { "false" }),
            ("DBusActivatable", if data.dbus_activatable { "true" } else { "false" }),
            ("Path", &data.working_dir),
            ("OnlyShowIn", &data.only_show_in),
            ("NotShowIn", &data.not_show_in),
            ("Actions", &data.actions),
            ("Implements", &data.implements),
        ];

        let structured_keys: Vec<&str> = updates.iter().map(|(k, _)| *k).collect();
        for (key, value) in &updates {
            self.set(key, value);
        }

        // Apply raw edits for keys not covered by structured fields
        for i in 0..data.raw_keys.row_count() {
            let Some(key) = data.raw_keys.row_data(i) else { continue };
            if !structured_keys.contains(&key.as_str()) {
                if let Some(pos) = self.keys.iter().position(|k| key == k) {
                    if let Some(value) = data.raw_values.row_data(i) {
                        self.values[pos] = value.to_string();
                    }
                }
            }
        }
    }

    fn write_to_file(&self) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&self.path)?;
        use std::io::Write;
        writeln!(out, "[Desktop Entry]")?;
        for (i, key) in self.keys.iter().enumerate() {
            if let Some(value) = self.values.get(i) {
                writeln!(out, "{}={}", key, value)?;
            }
        }
        Ok(())
    }
}

fn scan_desktop_entries() -> Vec<DesktopEntryData> {
    let mut entries = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let mut dirs: Vec<PathBuf> = STANDARD_PATHS.iter().map(PathBuf::from).collect();
    dirs.push(home_applications());

    for dir in dirs {
        if !dir.is_dir() {
            continue;
        }
        let Ok(files) = fs::read_dir(&dir) else { continue };
        for file in files.flatten() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
                continue;
            }
            if let Some(entry) = DesktopEntryData::from_file(&path) {
                let name = entry.get("Name");
                if !name.is_empty() && seen.insert(name.clone()) {
                    entries.push(entry);
                }
            }
        }
    }

    entries.sort_by(|a, b| a.get("Name").to_lowercase().cmp(&b.get("Name").to_lowercase()));
    entries
}

fn main() {
    let app = AppWindow::new().unwrap();

    let all_entries: Rc<RefCell<Vec<DesktopEntryData>>> = Rc::new(RefCell::new(Vec::new()));
    let current_raw: Rc<RefCell<Option<DesktopEntryData>>> = Rc::new(RefCell::new(None));

    // --- Scan ---
    let entries_c = all_entries.clone();
    let weak = app.as_weak();
    app.on_scan_entries(move || {
        let scanned = scan_desktop_entries();
        let count = scanned.len();
        *entries_c.borrow_mut() = scanned;

        let weak = weak.clone();
        slint::invoke_from_event_loop(move || {
            if let Some(app) = weak.upgrade() {
                app.set_status_text(format!("Found {} desktop files", count).into());
                let results = app.invoke_search("".into());
                app.set_search_results(results);
            }
        }).unwrap();
    });

    // --- Search ---
    let entries_c = all_entries.clone();
    app.on_search(move |query: slint::SharedString| {
        let entries = entries_c.borrow();
        let q = query.to_lowercase();
        let mut results: Vec<SearchResult> = Vec::new();

        for entry in entries.iter() {
            let name = entry.get("Name");
            let comment = entry.get("Comment");
            let icon = entry.get("Icon");
            let path = entry.path.to_string_lossy().to_string();

            let matches = q.is_empty()
                || name.to_lowercase().contains(&q)
                || comment.to_lowercase().contains(&q);

            if matches {
                results.push(SearchResult {
                    name: name.into(),
                    icon: icon.into(),
                    path: path.into(),
                    comment: comment.into(),
                });
            }
        }
        ModelRc::new(VecModel::from(results))
    });

    // --- Load entry ---
    let entries_c = all_entries.clone();
    let current_c = current_raw.clone();
    app.on_load_entry(move |index: i32| {
        let entries = entries_c.borrow();
        if let Some(entry) = entries.get(index as usize) {
            let data = entry.to_file_data();
            *current_c.borrow_mut() = Some(entry.clone());
            data
        } else {
            DesktopFileData::default()
        }
    });

    // --- Save ---
    let current_c = current_raw.clone();
    let all_c = all_entries.clone();
    let weak = app.as_weak();
    app.on_save_entry(move || {
        let mut raw = current_c.borrow_mut();
        if let Some(ref mut entry) = *raw {
            if let Some(app) = weak.upgrade() {
                let data = app.get_current_file();
                entry.apply_file_data(&data);
                match entry.write_to_file() {
                    Ok(()) => {
                        let mut all = all_c.borrow_mut();
                        if let Some(pos) = all.iter().position(|e| e.path == entry.path) {
                            all[pos] = entry.clone();
                        }
                    }
                    Err(e) => eprintln!("Error saving: {}", e),
                }
            }
        }
    });

    // --- New ---
    let current_c = current_raw.clone();
    let weak = app.as_weak();
    app.on_new_entry(move || {
        let new_entry = DesktopEntryData::new_empty();
        let data = new_entry.to_file_data();
        *current_c.borrow_mut() = Some(new_entry);

        if let Some(app) = weak.upgrade() {
            app.set_current_file(data);
            app.set_has_selection(true);
            app.set_unsaved_changes(true);

            let search_results = app.get_search_results();
            let mut results: Vec<SearchResult> = (0..search_results.row_count())
                .filter_map(|i| search_results.row_data(i))
                .collect();
            results.insert(0, SearchResult {
                name: "New Entry".into(),
                icon: "".into(),
                path: home_applications().join("my-application.desktop").to_string_lossy().to_string().into(),
                comment: "New desktop entry".into(),
            });
            app.set_search_results(ModelRc::new(VecModel::from(results)));
            app.set_selected_index(0);
        }
    });

    // --- Delete ---
    let current_c = current_raw.clone();
    let all_c = all_entries.clone();
    let weak = app.as_weak();
    app.on_delete_entry(move || {
        {
            let mut raw = current_c.borrow_mut();
            if let Some(ref entry) = *raw {
                let _ = fs::remove_file(&entry.path);
                all_c.borrow_mut().retain(|e| e.path != entry.path);
            }
            *raw = None;
        }

        if let Some(app) = weak.upgrade() {
            app.set_has_selection(false);
            app.set_unsaved_changes(false);
            app.set_status_text("Entry deleted".into());
            let results = app.invoke_search("".into());
            app.set_search_results(results);
        }
    });

    // --- Open in external editor ---
    let current_c = current_raw.clone();
    app.on_open_in_editor(move || {
        let raw = current_c.borrow();
        if let Some(ref entry) = *raw {
            let editor = std::env::var("EDITOR").unwrap_or_else(|_| "nano".into());
            let _ = std::process::Command::new(&editor)
                .arg(&entry.path)
                .spawn();
        }
    });

    // --- Reload ---
    let current_c = current_raw.clone();
    let weak = app.as_weak();
    app.on_reload_entry(move || {
        let mut raw = current_c.borrow_mut();
        if let Some(ref mut entry) = *raw {
            if let Some(reloaded) = DesktopEntryData::from_file(&entry.path) {
                *entry = reloaded;
                let data = entry.to_file_data();
                if let Some(app) = weak.upgrade() {
                    app.set_current_file(data);
                }
            }
        }
    });

    // Trigger initial scan
    app.invoke_scan_entries();

    app.run().unwrap();
}
