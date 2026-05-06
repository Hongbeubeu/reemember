mod commands;

use commands::{
    assign_word_to_topic,
    create_collection, create_topic,
    delete_collection, delete_topic, delete_word,
    import_vocabulary, list_collections, list_topics, list_words,
    next_question, save_export, submit_answer,
    update_collection, update_topic,
    list_grammar_docs, get_grammar_doc, import_grammar, delete_grammar_doc,
    list_grammar_groups, create_grammar_group, update_grammar_group,
    delete_grammar_group, move_grammar_doc,
};
use tauri::{WebviewUrl, WebviewWindowBuilder};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            next_question,
            submit_answer,
            list_words,
            delete_word,
            import_vocabulary,
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
            list_grammar_docs,
            get_grammar_doc,
            import_grammar,
            delete_grammar_doc,
            list_grammar_groups,
            create_grammar_group,
            update_grammar_group,
            delete_grammar_group,
            move_grammar_doc,
        ])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("Reemember")
                .inner_size(1000.0, 750.0)
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
