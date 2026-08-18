mod commands;
mod keys;

use tauri_plugin_updater::UpdaterExt;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_keyring::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Err(error) = update_on_start(handle).await {
                    eprintln!("automatic update check failed: {error}");
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::generate_identity,
            commands::load_identity,
            commands::clear_identity,
            commands::initiate_session,
            commands::complete_session,
            commands::encrypt_message,
            commands::decrypt_message,
            commands::get_or_create_lmk,
            commands::encrypt_attachment,
            commands::decrypt_attachment,
            commands::screenshot,
            commands::save_token,
            commands::get_token,
            commands::clear_token,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn update_on_start(app: tauri::AppHandle) -> tauri_plugin_updater::Result<()> {
    let Some(update) = app.updater()?.check().await? else {
        return Ok(());
    };

    update.download_and_install(|_, _| {}, || {}).await?;
    app.restart();
}
