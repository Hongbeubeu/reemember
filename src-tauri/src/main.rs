#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod commands;

use commands::{
    assign_word_to_topic, create_collection, create_grammar_group, create_topic, delete_collection,
    delete_grammar_doc, delete_grammar_group, delete_topic, delete_word, get_bilingual_article,
    get_grammar_doc, get_stats, import_bilingual, import_grammar, import_vocabulary,
    import_vocabulary_batch, list_bilingual_articles, list_collections, list_grammar_docs,
    list_grammar_groups, list_topics, list_words, move_grammar_doc, next_question, save_export,
    set_app_theme, submit_answer, sync_manifest_url, update_collection, update_grammar_group,
    update_topic,
};
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
            next_question,
            set_app_theme,
            submit_answer,
            list_words,
            delete_word,
            import_vocabulary,
            sync_manifest_url,
            save_export,
            list_collections,
            create_collection,
            update_collection,
            delete_collection,
            list_topics,
            create_topic,
            update_topic,
            delete_topic,
            assign_word_to_topic,
            list_bilingual_articles,
            get_bilingual_article,
            import_bilingual,
            list_grammar_docs,
            get_grammar_doc,
            import_grammar,
            delete_grammar_doc,
            list_grammar_groups,
            create_grammar_group,
            update_grammar_group,
            delete_grammar_group,
            move_grammar_doc,
            get_stats,
            import_vocabulary_batch,
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
