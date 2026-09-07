use ai_dubbing_lib::pipeline::metadata::UtteranceMetadataDocument;

#[test]
fn test_demo_metadata_creation_and_integrity() {
    let doc = UtteranceMetadataDocument::create_demo_data("presentation.mp4");
    assert_eq!(doc.source_language, "slk_Latn");
    assert_eq!(doc.target_language, "zho_Hans");
    assert_eq!(doc.utterances.len(), 3);

    let first = &doc.utterances[0];
    assert_eq!(first.id, "utt_001");
    assert!(first.slovak_text.contains("Dobrý deň"));
    assert!(first.chinese_text.contains("您好"));
    assert_eq!(first.words.len(), 8);
}

#[test]
fn test_metadata_security_validation() {
    let mut doc = UtteranceMetadataDocument::create_demo_data("test.mp4");
    assert!(doc.validate_integrity().is_ok());

    // Path traversal in target_audio_file must fail
    doc.utterances[0].target_audio_file = Some("../../etc/shadow".to_string());
    assert!(doc.validate_integrity().is_err());

    // Absolute path in target_audio_file must fail
    doc.utterances[0].target_audio_file = Some("/var/log/hack.wav".to_string());
    assert!(doc.validate_integrity().is_err());

    // Windows absolute path must fail
    doc.utterances[0].target_audio_file = Some("C:\\Windows\\system32\\hack.wav".to_string());
    assert!(doc.validate_integrity().is_err());

    // Duplicate IDs must fail
    let mut doc2 = UtteranceMetadataDocument::create_demo_data("test.mp4");
    doc2.utterances[1].id = "utt_001".to_string();
    assert!(doc2.validate_integrity().is_err());
}
