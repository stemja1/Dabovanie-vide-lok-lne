use serde::Serialize;
use tauri::State;

use crate::commands::pipeline_commands::OrchestratorState;
use crate::error::AppResult;
use crate::pipeline::metadata::{UtteranceItem, UtteranceMetadataDocument};

/// Payload for the review editor: the document plus the path it must be saved to.
#[derive(Debug, Clone, Serialize)]
pub struct ReviewMetadataPayload {
    pub document: UtteranceMetadataDocument,
    /// Absolute Windows path to save back into. `None` when no pipeline run is
    /// active, in which case saving is a no-op rather than writing a stray file
    /// into the process working directory.
    pub file_path: Option<String>,
    /// `true` when the document came from an actual pipeline run.
    pub is_real_run: bool,
}

#[tauri::command]
pub fn load_utterance_metadata(file_path: String) -> AppResult<UtteranceMetadataDocument> {
    Ok(UtteranceMetadataDocument::load_from_file(&file_path)?)
}

/// Returns the metadata document the review editor should show for the current
/// pipeline state.
///
/// When a pipeline run has produced a metadata file, that exact file is returned
/// and `file_path` carries the path it came from, so a subsequent
/// `save_utterance_metadata` writes back to the same location. The editor used
/// to call `get_demo_utterance_metadata` unconditionally and save to a
/// hardcoded relative path, so every user edit was discarded and the review
/// stage could not influence the pipeline at all.
#[tauri::command]
pub async fn get_review_utterance_metadata(
    orchestrator_state: State<'_, OrchestratorState>,
) -> AppResult<ReviewMetadataPayload> {
    // Async lock rather than `blocking_lock()`: this command runs on the Tauri
    // async runtime, where blocking a worker thread can deadlock the executor.
    let state = orchestrator_state.0.state.lock().await;

    if let Some(doc) = UtteranceMetadataDocument::resolve_review_document(&state)? {
        let path = state.metadata_json_path_win.clone().unwrap_or_default();
        return Ok(ReviewMetadataPayload {
            document: doc,
            file_path: Some(path),
            is_real_run: true,
        });
    }

    // No metadata file for this run yet. Only offer a save target when a video is
    // actually selected, so a fresh session cannot "save" into a path no stage
    // will ever read.
    let save_path = if state.input_video_path_win.trim().is_empty() {
        None
    } else {
        state
            .metadata_json_path_win
            .clone()
            .filter(|p| !p.trim().is_empty())
    };

    Ok(ReviewMetadataPayload {
        document: UtteranceMetadataDocument::create_demo_data(&state.input_video_path_win),
        file_path: save_path,
        is_real_run: false,
    })
}

#[tauri::command]
pub fn save_utterance_metadata(
    file_path: String,
    document: UtteranceMetadataDocument,
) -> AppResult<()> {
    Ok(document.save_to_file(&file_path)?)
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
) -> AppResult<UtteranceMetadataDocument> {
    document.split_utterance(
        &utterance_id,
        split_time,
        sk_part1,
        sk_part2,
        zh_part1,
        zh_part2,
    )?;
    Ok(document)
}

#[tauri::command]
pub fn merge_utterance_items(
    mut document: UtteranceMetadataDocument,
    id1: String,
    id2: String,
) -> AppResult<UtteranceMetadataDocument> {
    document.merge_utterances(&id1, &id2)?;
    Ok(document)
}
