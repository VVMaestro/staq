use database::{CreateStaqData, Placement};
use staq::{Staq, Task};
use state::{AppState, TaskPushData};
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
async fn get_staq(state: tauri::State<'_, AppState>, staq_id: Option<i64>) -> Result<Staq, String> {
    let app_state: &AppState = state.inner();
    let staq = app_state
        .staq(staq_id)
        .await
        .map_err(|error| error.to_string())?;
    let staq = staq.lock().await;

    Ok(staq.clone())
}

#[tauri::command]
async fn get_staqs(state: tauri::State<'_, AppState>) -> Result<Vec<Staq>, String> {
    Ok(state.list_staqs().await)
}

#[tauri::command]
async fn staq_push_to_queue(
    state: tauri::State<'_, AppState>,
    item: TaskPushData,
    staq_id: Option<i64>,
) -> Result<Staq, String> {
    state
        .push(&item, Placement::Queue, staq_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_push_on_stack(
    state: tauri::State<'_, AppState>,
    item: TaskPushData,
    staq_id: Option<i64>,
) -> Result<Staq, String> {
    state
        .push(&item, Placement::Stack, staq_id)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_pop(state: tauri::State<'_, AppState>, staq_id: Option<i64>) -> Result<Staq, String> {
    state.pop(staq_id).await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_peek(
    state: tauri::State<'_, AppState>,
    staq_id: Option<i64>,
) -> Result<Option<Task>, String> {
    let staq = state
        .staq(staq_id)
        .await
        .map_err(|error| error.to_string())?;
    let staq = staq.lock().await;

    Ok(staq.peek().cloned())
}

#[tauri::command]
async fn staq_create(
    state: tauri::State<'_, AppState>,
    data: CreateStaqData,
) -> Result<Staq, String> {
    state
        .create_staq(data)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn staq_delete(state: tauri::State<'_, AppState>, staq_id: i64) -> Result<(), String> {
    state
        .delete_staq(staq_id)
        .await
        .map_err(|error| error.to_string())
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

            #[cfg(target_os = "macos")]
            {
                let window = app
                    .get_webview_window("main")
                    .ok_or("main window missing")?;
                let main_window = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        // Preserve the WebView and unfinished input until an explicit Quit.
                        api.prevent_close();
                        if let Err(error) = main_window.hide() {
                            eprintln!("failed to hide main window: {error}");
                        }
                    }
                });
            }

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
            get_staqs,
            staq_push_to_queue,
            staq_push_on_stack,
            staq_pop,
            staq_peek,
            staq_create,
            staq_delete,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app, _event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen { .. } = _event {
                show_main_window(_app);
            }
        });
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
