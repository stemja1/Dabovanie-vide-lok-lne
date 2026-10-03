use ai_dubbing_lib::pipeline::metadata::UtteranceMetadataDocument;
use ai_dubbing_lib::pipeline::orchestrator::PipelineExecutionState;
use ai_dubbing_lib::pipeline::stages::StageFactory;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Builds a pipeline state shaped like a real run: a video path plus the
/// per-video metadata path derived from it by `set_input_video`.
fn state_with_metadata_path(meta: Option<String>) -> PipelineExecutionState {
    PipelineExecutionState {
        is_running: false,
        is_paused_for_review: false,
        current_stage_index: 0,
        stages: StageFactory::build_default_stages(),
        input_video_path_win: r"C:\AI_Dubbing\Videos\prezentacia.mp4".to_string(),
        input_video_path_wsl: "/mnt/c/AI_Dubbing/Videos/prezentacia.mp4".to_string(),
        output_video_path_win: None,
        output_video_path_wsl: None,
        metadata_json_path_win: meta,
        metadata_json_path_wsl: None,
        error_summary: None,
        active_lipsync_engine: "LatentSync 1.5".to_string(),
    }
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("aidubbing-review-{tag}-{nanos}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

/// A real pipeline run leaves a metadata file on disk; the editor must load that
/// exact file instead of the hardcoded demo document.
#[test]
fn resolves_existing_metadata_file() {
    let dir = temp_dir("exists");
    let path = dir.join("prezentacia_utterance_metadata.json");
    UtteranceMetadataDocument::create_demo_data("prezentacia.mp4")
        .save_to_file(&path)
        .expect("save metadata");

    let state = state_with_metadata_path(Some(path.to_string_lossy().to_string()));
    let resolved = UtteranceMetadataDocument::resolve_review_document(&state)
        .expect("resolve must not error")
        .expect("existing file must resolve to a document");

    assert_eq!(resolved.utterances.len(), 3);
    fs::remove_dir_all(&dir).ok();
}

/// Before stage 3 there is no file yet. The editor must fall back to the demo
/// document rather than erroring out, so the UI stays usable.
#[test]
fn falls_back_when_file_missing() {
    let dir = temp_dir("missing");
    let path = dir.join("does_not_exist_utterance_metadata.json");

    let state = state_with_metadata_path(Some(path.to_string_lossy().to_string()));
    let resolved =
        UtteranceMetadataDocument::resolve_review_document(&state).expect("resolve must not error");

    assert!(resolved.is_none(), "missing file must resolve to None");
    fs::remove_dir_all(&dir).ok();
}

/// No video selected at all: same graceful fallback.
#[test]
fn falls_back_when_no_path_configured() {
    let state = state_with_metadata_path(None);
    let resolved =
        UtteranceMetadataDocument::resolve_review_document(&state).expect("resolve must not error");
    assert!(resolved.is_none());

    let blank = state_with_metadata_path(Some("   ".to_string()));
    let resolved_blank =
        UtteranceMetadataDocument::resolve_review_document(&blank).expect("resolve must not error");
    assert!(
        resolved_blank.is_none(),
        "whitespace-only path must resolve to None"
    );
}

/// A malformed file on disk must surface as an error rather than silently
/// falling back to demo data, otherwise the user would edit the wrong document.
#[test]
fn malformed_file_surfaces_error() {
    let dir = temp_dir("malformed");
    let path = dir.join("broken_utterance_metadata.json");
    fs::write(&path, "{ this is not valid json").expect("write malformed file");

    let state = state_with_metadata_path(Some(path.to_string_lossy().to_string()));
    let result = UtteranceMetadataDocument::resolve_review_document(&state);

    assert!(
        result.is_err(),
        "malformed metadata must not silently fall back to demo data"
    );
    fs::remove_dir_all(&dir).ok();
}

/// The document duration is the latest `end_time`, not the sum of segment
/// durations. Summing ignores the silence between segments and produced a value
/// shorter than the video, which made stage 4 size the dubbed track too small and
/// drop trailing utterances.
#[test]
fn total_duration_uses_latest_end_time_not_duration_sum() {
    let mut doc = UtteranceMetadataDocument::create_demo_data("test.mp4");
    // Demo: 0.5-3.8, 4.2-9.0, 9.6-14.8 => durations sum to 13.3, latest end 14.8.
    let duration_sum: f64 = doc.utterances.iter().map(|u| u.duration).sum();
    doc.recalculate_timings();

    assert!(
        doc.total_duration > duration_sum,
        "total_duration ({}) should exceed the sum of segment durations ({})",
        doc.total_duration,
        duration_sum
    );
    assert_eq!(doc.total_duration, 14.8);
}

/// A user deleting every segment must not leave a stale duration behind.
#[test]
fn total_duration_zero_after_deleting_all_segments() {
    let mut doc = UtteranceMetadataDocument::create_demo_data("test.mp4");
    doc.utterances.clear();
    doc.recalculate_timings();
    assert_eq!(doc.total_duration, 0.0);
}
