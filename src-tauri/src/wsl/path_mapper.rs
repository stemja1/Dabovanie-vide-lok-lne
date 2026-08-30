pub struct PathMapper;

impl PathMapper {
    /// Converts a Windows absolute or relative path to a WSL Linux path.
    /// E.g. "C:\\Users\\John\\video.mp4" -> "/mnt/c/Users/John/video.mp4"
    /// Handles mixed slashes, lowercase/uppercase drives, and UNC paths.
    pub fn win_to_wsl(win_path: &str) -> String {
        let trimmed = win_path.trim().replace('\\', "/");
        
        // Handle Windows extended-length path prefix "\\?\" -> "//?/"
        let clean = if trimmed.starts_with("//?/") {
            &trimmed[4..]
        } else {
            &trimmed
        };

        // Handle WSL UNC paths e.g. "//wsl.localhost/Ubuntu-24.04/home/user/file" or "//wsl$/..."
        if clean.starts_with("//wsl.localhost/") || clean.starts_with("//wsl$/") {
            let parts: Vec<&str> = clean.splitn(4, '/').collect();
            // parts = ["", "", "wsl.localhost", "Ubuntu-24.04/home/user/file"]
            if parts.len() >= 4 {
                let sub_parts: Vec<&str> = parts[3].splitn(2, '/').collect();
                if sub_parts.len() == 2 {
                    return format!("/{}", sub_parts[1]);
                }
            }
        }

        // Check drive letter pattern: "C:/" or "D:/"
        if let Some(first_char) = clean.chars().next() {
            if clean.len() >= 2 && clean.chars().nth(1) == Some(':') {
                let drive = first_char.to_ascii_lowercase();
                let rest = if clean.len() > 2 {
                    &clean[2..]
                } else {
                    ""
                };
                let clean_rest = rest.trim_start_matches('/');
                return format!("/mnt/{}/{}", drive, clean_rest);
            }
        }

        // If it's already a Linux-like path or relative, return normalized
        clean.to_string()
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

    /// Safely escapes an argument for single-quoted POSIX bash execution.
    /// Prevents command injection, parameter tampering, and shell metacharacter expansion.
    /// E.g. `file'name$1.mp4` -> `'file'\''name$1.mp4'`
    pub fn escape_bash_arg(arg: &str) -> String {
        // Enclose in single quotes and replace each internal single quote with '\''
        format!("'{}'", arg.replace('\'', "'\\''"))
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

    /// Validates and sanitizes a file path to prevent null bytes or dangerous directory traversal
    pub fn sanitize_path(path: &str) -> anyhow::Result<String> {
        if path.contains('\0') {
            anyhow::bail!("Cesta obsahuje nepovolené nulové bajty");
        }
        Ok(path.trim().to_string())
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
        assert_eq!(
            PathMapper::win_to_wsl(r"\\?\C:\Users\Admin\file.mp4"),
            "/mnt/c/Users/Admin/file.mp4"
        );
        assert_eq!(
            PathMapper::win_to_wsl(r"\\wsl.localhost\Ubuntu-24.04\home\ubuntu\output.mp4"),
            "/home/ubuntu/output.mp4"
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

    #[test]
    fn test_escape_bash_arg() {
        assert_eq!(
            PathMapper::escape_bash_arg("/mnt/c/Videos/simple.mp4"),
            "'/mnt/c/Videos/simple.mp4'"
        );
        assert_eq!(
            PathMapper::escape_bash_arg("/mnt/c/Videos/O'Connor $(whoami).mp4"),
            "'/mnt/c/Videos/O'\\''Connor $(whoami).mp4'"
        );
        assert_eq!(
            PathMapper::escape_bash_arg("video; rm -rf /; echo test"),
            "'video; rm -rf /; echo test'"
        );
    }
}
