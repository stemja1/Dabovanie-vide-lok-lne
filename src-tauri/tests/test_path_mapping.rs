use ai_dubbing_lib::wsl::path_mapper::PathMapper;

#[test]
fn test_windows_to_wsl_mappings() {
    assert_eq!(
        PathMapper::win_to_wsl("C:\\Users\\JohnDoe\\Videos\\my_video.mp4"),
        "/mnt/c/Users/JohnDoe/Videos/my_video.mp4"
    );
    assert_eq!(
        PathMapper::win_to_wsl("D:\\Data\\Project\\test.wav"),
        "/mnt/d/Data/Project/test.wav"
    );
    assert_eq!(
        PathMapper::win_to_wsl("/mnt/c/already/linux.mp4"),
        "/mnt/c/already/linux.mp4"
    );
}

#[test]
fn test_wsl_to_windows_mappings() {
    assert_eq!(
        PathMapper::wsl_to_win("/mnt/c/Users/JohnDoe/output.mp4", "Ubuntu-24.04"),
        "C:\\Users\\JohnDoe\\output.mp4"
    );
    assert_eq!(
        PathMapper::wsl_to_win("/home/ubuntu/ai_dubbing_workspace/output.mp4", "Ubuntu-24.04"),
        "\\\\wsl.localhost\\Ubuntu-24.04\\home\\ubuntu\\ai_dubbing_workspace\\output.mp4"
    );
}
