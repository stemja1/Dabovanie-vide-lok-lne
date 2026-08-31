use crate::pipeline::metadata::{UtteranceItem, UtteranceMetadataDocument};

#[tauri::command]
pub fn load_utterance_metadata(file_path: String) -> Result<UtteranceMetadataDocument, String> {
    UtteranceMetadataDocument::load_from_file(&file_path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_utterance_metadata(
    file_path: String,
    document: UtteranceMetadataDocument,
) -> Result<(), String> {
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
    if let Some(pos) = document
        .utterances
        .iter()
        .position(|u| u.id == updated_item.id)
    {
        document.utterances[pos] = updated_item;
        document.utterances[pos].is_edited = true;
    }
    document.recalculate_timings();
    document
}

#[tauri::command]
pub fn split_utterance_item(
    mut document: UtteranceMetadataDocument,
    utterance_id: String,
    split_time: f64,
    sk_part1: String,
    sk_part2: String,
    zh_part1: String,
    zh_part2: String,
) -> Result<UtteranceMetadataDocument, String> {
    document
        .split_utterance(
            &utterance_id,
            split_time,
            sk_part1,
            sk_part2,
            zh_part1,
            zh_part2,
        )
        .map_err(|e| e.to_string())?;
    Ok(document)
}

#[tauri::command]
pub fn merge_utterance_items(
    mut document: UtteranceMetadataDocument,
    id1: String,
    id2: String,
) -> Result<UtteranceMetadataDocument, String> {
    document
        .merge_utterances(&id1, &id2)
        .map_err(|e| e.to_string())?;
    Ok(document)
}
