
pub struct PathMapper;

impl PathMapper {
    /// Converts a Windows absolute or relative path to a WSL Linux path.
    /// E.g. "C:\\Users\\John\\video.mp4" -> "/mnt/c/Users/John/video.mp4"
    pub fn win_to_wsl(win_path: &str) -> String {
        let trimmed = win_path.trim().replace('\\', "/");
        
        // Check drive letter pattern: "C:/" or "D:/"
        if let Some(first_char) = trimmed.chars().next() {
            if trimmed.len() >= 2 && trimmed.chars().nth(1) == Some(':') {
                let drive = first_char.to_ascii_lowercase();
                let rest = if trimmed.len() > 2 {
                    &trimmed[2..]
                } else {
                    ""
                };
                let clean_rest = rest.trim_start_matches('/');
                return format!("/mnt/{}/{}", drive, clean_rest);
            }
        }

        // If it's already a Linux-like path or relative, return normalized
        trimmed
    }

    /// Converts a WSL Linux path to a Windows UNC or local path.
    /// E.g. "/home/user/workspace/out.mp4" (distro="Ubuntu-24.04") -> "\\\\wsl.localhost\\Ubuntu-24.04\\home\\user\\workspace\\out.mp4"
    /// E.g. "/mnt/c/Users/John/file.txt" -> "C:\\Users\\John\\file.txt"
    pub fn wsl_to_win(wsl_path: &str, distro: &str) -> String {
        let trimmed = wsl_path.trim().replace('\\', "/");

        // If path starts with /mnt/X/
        if trimmed.starts_with("/mnt/") && trimmed.len() >= 7 {
            let parts: Vec<&str> = trimmed.splitn(4, '/').collect();
            // parts = ["", "mnt", "c", "Users/John/file.txt"]
            if parts.len() >= 3 && parts[2].len() == 1 {
                let drive = parts[2].to_ascii_uppercase();
                let rest = if parts.len() == 4 { parts[3] } else { "" };
                let win_rest = rest.replace('/', "\\");
                return format!("{}:\\{}", drive, win_rest);
            }
        }

        // Otherwise it's an internal WSL path, map to UNC path
        let clean_path = trimmed.trim_start_matches('/').replace('/', "\\");
        format!("\\\\wsl.localhost\\{}\\{}", distro, clean_path)
    }

    /// Expands "~" with WSL home path in script paths
    pub fn expand_wsl_home(path: &str, wsl_user: Option<&str>) -> String {
        if path.starts_with('~') {
            let user = wsl_user.unwrap_or("$USER");
            path.replacen('~', &format!("/home/{}", user), 1)
        } else {
            path.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_win_to_wsl() {
        assert_eq!(
            PathMapper::win_to_wsl("C:\\Users\\Default\\Videos\\sample.mp4"),
            "/mnt/c/Users/Default/Videos/sample.mp4"
        );
        assert_eq!(
            PathMapper::win_to_wsl("d:\\data\\input.wav"),
            "/mnt/d/data/input.wav"
        );
    }

    #[test]
    fn test_wsl_to_win() {
        assert_eq!(
            PathMapper::wsl_to_win("/mnt/c/Users/Default/video.mp4", "Ubuntu-24.04"),
            "C:\\Users\\Default\\video.mp4"
        );
        assert_eq!(
            PathMapper::wsl_to_win("/home/ubuntu/output/dubbed.mp4", "Ubuntu-24.04"),
            "\\\\wsl.localhost\\Ubuntu-24.04\\home\\ubuntu\\output\\dubbed.mp4"
        );
    }
}
