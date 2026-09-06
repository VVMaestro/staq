use staq::{Staq, Task};
use state::AppState;
use tauri::Manager;

mod database;
pub mod staq;
pub mod state;
#[cfg(windows)]
mod tray;

#[cfg(test)]
mod persistence_tests;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn get_staq(state: tauri::State<'_, AppState>) -> Result<Staq, String> {
    Ok(state.staq.lock().await.clone())
}

#[tauri::command]
async fn staq_push_to_queue(
    state: tauri::State<'_, AppState>,
    item: String,
) -> Result<Staq, String> {
    state
        .push(&item, database::Placement::Queue)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_push_on_stack(
    state: tauri::State<'_, AppState>,
    item: String,
) -> Result<Staq, String> {
    state
        .push(&item, database::Placement::Stack)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_pop(state: tauri::State<'_, AppState>) -> Result<Staq, String> {
    state.pop().await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_peek(state: tauri::State<'_, AppState>) -> Result<Option<Task>, String> {
    Ok(state.staq.lock().await.peek().cloned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    // Register before database setup so only the primary instance loads state.
    #[cfg(desktop)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
        show_main_window(app);
    }));

    builder
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;

            let database_path = app_data_dir.join("staq.sqlite3");
            let state = tauri::async_runtime::block_on(async {
                let pool = database::initialize(&database_path).await?;
                AppState::load(pool).await
            })?;
            app.manage(state);

            #[cfg(windows)]
            if let Err(error) = tray::setup(app) {
                eprintln!("failed to create system tray; close will exit the app: {error}");
            }

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_staq,
            staq_push_to_queue,
            staq_push_on_stack,
            staq_pop,
            staq_peek
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(desktop)]
fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.show() {
            eprintln!("failed to show main window: {error}");
        }
        if let Err(error) = window.unminimize() {
            eprintln!("failed to restore main window: {error}");
        }
        if let Err(error) = window.set_focus() {
            eprintln!("failed to focus main window: {error}");
        }
    }
}
