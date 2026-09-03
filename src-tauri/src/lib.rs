use staq::{Staq, Task};
use std::sync::Mutex;

pub mod staq;

struct AppState {
    staq: Staq,
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_staq(state: tauri::State<Mutex<AppState>>) -> Result<Staq, String> {
    let state = state
        .lock()
        .map_err(|_| "failed to lock application state".to_string())?;
    Ok(state.staq.clone())
}

#[tauri::command]
fn staq_push_to_queue(state: tauri::State<Mutex<AppState>>, item: String) -> Result<Staq, String> {
    let mut state = state
        .lock()
        .map_err(|_| "failed to lock application state".to_string())?;
    state.staq.push(item);
    Ok(state.staq.clone())
}

#[tauri::command]
fn staq_push_on_stack(state: tauri::State<Mutex<AppState>>, item: String) -> Result<Staq, String> {
    let mut state = state
        .lock()
        .map_err(|_| "failed to lock application state".to_string())?;
    state.staq.push_on_stack(item);
    Ok(state.staq.clone())
}

#[tauri::command]
fn staq_pop(state: tauri::State<Mutex<AppState>>) -> Result<Staq, String> {
    let mut state = state
        .lock()
        .map_err(|_| "failed to lock application state".to_string())?;
    state.staq.pop();
    Ok(state.staq.clone())
}

#[tauri::command]
fn staq_peek(state: tauri::State<Mutex<AppState>>) -> Result<Option<Task>, String> {
    let state = state
        .lock()
        .map_err(|_| "failed to lock application state".to_string())?;
    Ok(state.staq.peek().cloned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState { staq: Staq::new() }))
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
