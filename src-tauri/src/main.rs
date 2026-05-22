#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod commands;

use reemember::db::init_db;
use reemember::repository::WordRepository;
use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_notification::NotificationExt;

const DB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/reemember.db");

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::next_question,
            commands::set_app_theme,
            commands::submit_answer,
            commands::list_words,
            commands::delete_word,
            commands::import_vocabulary,
            commands::sync_manifest_url,
            commands::save_export,
            commands::list_collections,
            commands::create_collection,
            commands::update_collection,
            commands::delete_collection,
            commands::list_topics,
            commands::create_topic,
            commands::update_topic,
            commands::delete_topic,
            commands::assign_word_to_topic,
            commands::list_bilingual_articles,
            commands::get_bilingual_article,
            commands::import_bilingual,
            commands::list_grammar_docs,
            commands::get_grammar_doc,
            commands::import_grammar,
            commands::delete_grammar_doc,
            commands::list_grammar_groups,
            commands::create_grammar_group,
            commands::update_grammar_group,
            commands::delete_grammar_group,
            commands::move_grammar_doc,
            commands::get_stats,
            commands::import_vocabulary_batch,
            commands::sync_local_data,
        ])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Reemember")
                .inner_size(1000.0, 750.0)
                .build()?;

            if let Ok(conn) = init_db(DB_PATH) {
                let repo = WordRepository::new(conn);
                if let Ok(stats) = repo.get_stats() {
                    if stats.due_count > 0 {
                        let _ = app.notification()
                            .builder()
                            .title("Reemember")
                            .body(format!("Bạn có {} từ đang chờ ôn tập!", stats.due_count))
                            .show();
                    }
                }
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
