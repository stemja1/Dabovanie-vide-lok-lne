use ai_dubbing_lib::wsl::path_mapper::PathMapper;

#[test]
fn test_command_injection_prevention() {
    let malicious_inputs = vec![
        "video'; rm -rf /; echo 'hacked.mp4",
        "video$(whoami).mp4",
        "video`id`.mp4",
        "video | cat /etc/passwd | test.mp4",
        "file with spaces and $HOME and 'quotes'.mp4",
        "&& echo 'attack' > /tmp/pwned && video.mp4",
        "\"; echo dangerous; #.mp4",
    ];

    for input in malicious_inputs {
        let escaped = PathMapper::escape_bash_arg(input);
        // Must start and end with single quote
        assert!(escaped.starts_with('\''), "Escaped must start with single quote: {}", escaped);
        assert!(escaped.ends_with('\''), "Escaped must end with single quote: {}", escaped);
        // Must not allow unescaped single quote
        let internal = &escaped[1..escaped.len() - 1];
        let has_unescaped_quote = internal.contains('\'') && !internal.contains("'\\''");
        assert!(!has_unescaped_quote, "No raw single quotes allowed: {}", escaped);
    }
}

#[test]
fn test_null_byte_sanitization() {
    let bad_path = "video\0malicious.mp4";
    assert!(PathMapper::sanitize_path(bad_path).is_err(), "Null bytes must be rejected");

    let good_path = "C:\\Videos\\clean_video.mp4";
    assert!(PathMapper::sanitize_path(good_path).is_ok());
}
