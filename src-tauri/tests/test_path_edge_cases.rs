use ai_dubbing_lib::wsl::path_mapper::PathMapper;

#[test]
fn test_complex_windows_paths() {
    assert_eq!(
        PathMapper::win_to_wsl(r"C:\Users\John Doe\Videos\Sub Folder\video (1).mp4"),
        "/mnt/c/Users/John Doe/Videos/Sub Folder/video (1).mp4"
    );
    assert_eq!(
        PathMapper::win_to_wsl(r"e:\movies\presentation_2026.mkv"),
        "/mnt/e/movies/presentation_2026.mkv"
    );
    assert_eq!(
        PathMapper::win_to_wsl(r"\\wsl$\Ubuntu-24.04\home\user\project\output.mp4"),
        "/home/user/project/output.mp4"
    );
}

#[test]
fn test_wsl_to_win_edge_cases() {
    assert_eq!(
        PathMapper::wsl_to_win("/mnt/d/Audio/track.wav", "Ubuntu-24.04"),
        "D:\\Audio\\track.wav"
    );
    assert_eq!(
        PathMapper::wsl_to_win("/var/tmp/render.mp4", "Ubuntu-24.04"),
        "\\\\wsl.localhost\\Ubuntu-24.04\\var\\tmp\\render.mp4"
    );
}
