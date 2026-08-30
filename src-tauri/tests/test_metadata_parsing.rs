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
