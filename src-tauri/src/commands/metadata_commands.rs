use crate::pipeline::metadata::{UtteranceItem, UtteranceMetadataDocument};

#[tauri::command]
pub fn load_utterance_metadata(file_path: String) -> Result<UtteranceMetadataDocument, String> {
    UtteranceMetadataDocument::load_from_file(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_utterance_metadata(file_path: String, document: UtteranceMetadataDocument) -> Result<(), String> {
    document.save_to_file(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_demo_utterance_metadata() -> UtteranceMetadataDocument {
    UtteranceMetadataDocument::create_demo_data("sample_presentation.mp4")
}

#[tauri::command]
pub fn update_utterance_item(
    mut document: UtteranceMetadataDocument,
    updated_item: UtteranceItem,
) -> UtteranceMetadataDocument {
    if let Some(pos) = document.utterances.iter().position(|u| u.id == updated_item.id) {
        document.utterances[pos] = updated_item;
        document.utterances[pos].is_edited = true;
    }
    document
}
