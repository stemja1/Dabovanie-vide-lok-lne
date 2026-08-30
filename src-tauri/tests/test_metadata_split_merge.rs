use ai_dubbing_lib::pipeline::metadata::UtteranceMetadataDocument;

#[test]
fn test_split_and_merge_utterance_flow() {
    let mut doc = UtteranceMetadataDocument::create_demo_data("test.mp4");
    assert_eq!(doc.utterances.len(), 3);

    // Split utt_001 (duration 0.5 to 3.8s) at 2.0s
    let res = doc.split_utterance(
        "utt_001",
        2.0,
        "Dobrý deň,".to_string(),
        "vítam vás pri prezentácii nášho nového produktu.".to_string(),
        "您好，".to_string(),
        "欢迎来到我们新产品的展示会。".to_string(),
    );
    assert!(res.is_ok(), "Split must succeed: {:?}", res.err());
    assert_eq!(doc.utterances.len(), 4);

    let part1 = &doc.utterances[0];
    let part2 = &doc.utterances[1];
    assert_eq!(part1.id, "utt_001_a");
    assert_eq!(part1.start_time, 0.5);
    assert_eq!(part1.end_time, 2.0);
    assert_eq!(part1.slovak_text, "Dobrý deň,");

    assert_eq!(part2.id, "utt_001_b");
    assert_eq!(part2.start_time, 2.0);
    assert_eq!(part2.end_time, 3.8);

    // Merge them back
    let merge_res = doc.merge_utterances("utt_001_a", "utt_001_b");
    assert!(merge_res.is_ok(), "Merge must succeed: {:?}", merge_res.err());
    assert_eq!(doc.utterances.len(), 3);

    let merged = &doc.utterances[0];
    assert_eq!(merged.start_time, 0.5);
    assert_eq!(merged.end_time, 3.8);
    assert!(merged.slovak_text.contains("Dobrý deň,"));
}

#[test]
fn test_invalid_split_boundary_rejected() {
    let mut doc = UtteranceMetadataDocument::create_demo_data("test.mp4");
    // utt_001 is [0.5, 3.8]. Split at 5.0 must fail
    let res = doc.split_utterance("utt_001", 5.0, "a".into(), "b".into(), "c".into(), "d".into());
    assert!(res.is_err());
}
