use anyhow::Result;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::Stdio;
#[cfg(target_os = "windows")]
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslStatusInfo {
    pub is_wsl_installed: bool,
    pub is_default_version_2: bool,
    pub distros: Vec<WslDistroInfo>,
    pub target_distro_found: bool,
    pub is_target_distro_running: bool,
    pub kernel_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WslDistroInfo {
    pub name: String,
    pub is_default: bool,
    pub version: u32,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RocmStatusInfo {
    pub is_rocm_available: bool,
    pub rocm_version: Option<String>,
    pub gpu_device_name: Option<String>,
    pub total_vram_mb: Option<u64>,
    pub free_vram_mb: Option<u64>,
    pub is_hip_available: bool,
}

pub struct WslBridge;

impl WslBridge {
    /// Detects WSL status, installed distributions and versions on Windows without console popup.
    pub async fn detect_wsl_status(target_distro: &str) -> Result<WslStatusInfo> {
        #[cfg(target_os = "windows")]
        {
            let mut cmd = Command::new("wsl.exe");
            cmd.creation_flags(0x08000000);
            cmd.args(["-l", "-v"]);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let output = match cmd.output().await {
                Ok(out) => out,
                Err(_) => {
                    return Ok(WslStatusInfo {
                        is_wsl_installed: false,
                        is_default_version_2: false,
                        distros: vec![],
                        target_distro_found: false,
                        is_target_distro_running: false,
                        kernel_version: None,
                    });
                }
            };

            // wsl -l -v on Windows outputs UTF-16LE or UTF-8 depending on system locale
            let raw_str = Self::decode_wsl_output(&output.stdout);
            let distros = Self::parse_wsl_list_output(&raw_str);

            let target_found = distros
                .iter()
                .any(|d| d.name.eq_ignore_ascii_case(target_distro));
            let target_running = distros.iter().any(|d| {
                d.name.eq_ignore_ascii_case(target_distro)
                    && d.state.eq_ignore_ascii_case("Running")
            });

            // Check default version
            let is_v2 = distros
                .iter()
                .any(|d| d.name.eq_ignore_ascii_case(target_distro) && d.version == 2)
                || distros.iter().any(|d| d.is_default && d.version == 2);

            Ok(WslStatusInfo {
                is_wsl_installed: true,
                is_default_version_2: is_v2,
                distros,
                target_distro_found: target_found,
                is_target_distro_running: target_running,
                kernel_version: Some("WSL2 / Linux 6.6+".to_string()),
            })
        }

        #[cfg(not(target_os = "windows"))]
        {
            // On non-Windows development hosts (e.g. testing in Linux container)
            Ok(WslStatusInfo {
                is_wsl_installed: true,
                is_default_version_2: true,
                distros: vec![WslDistroInfo {
                    name: target_distro.to_string(),
                    is_default: true,
                    version: 2,
                    state: "Running".to_string(),
                }],
                target_distro_found: true,
                is_target_distro_running: true,
                kernel_version: Some("Native Linux / Host".to_string()),
            })
        }
    }

    /// Checks ROCm and PyTorch HIP status inside the target WSL distro.
    pub async fn check_rocm_status(distro: &str, venv_path: &str) -> Result<RocmStatusInfo> {
        // `venv_path` comes from user-editable AppConfig (settable via `save_config` /
        // `import_config_toml`), so it MUST be safely quoted before it is spliced into
        // a bash command string. `VENV="{0}"` (double quotes) would still let bash
        // evaluate `$(...)`/backticks embedded in the value; `escape_bash_arg` wraps it
        // in single quotes, which bash never expands, closing that injection vector.
        let venv_clean = crate::wsl::path_mapper::PathMapper::escape_bash_arg(
            venv_path.trim_end_matches('/'),
        );
        let test_script = format!(
            r#"VENV={0}; VENV="${{VENV/#\~/$HOME}}"; test -f "$VENV/bin/python" && "$VENV/bin/python" -c "
import sys, json
info = {{'rocm_available': False, 'rocm_version': None, 'gpu_name': None, 'total_vram_mb': None, 'free_vram_mb': None, 'hip': False}}
try:
    import torch
    info['hip'] = hasattr(torch.version, 'hip') and torch.version.hip is not None
    info['rocm_available'] = torch.cuda.is_available()
    if info['rocm_available']:
        info['gpu_name'] = torch.cuda.get_device_name(0)
        # mem_get_info() queries the actual driver for live (free, total) VRAM
        # in bytes and works on both CUDA and the ROCm/HIP backend — this
        # reports real numbers instead of a hardcoded guess for free memory.
        free_bytes, total_bytes = torch.cuda.mem_get_info(0)
        info['total_vram_mb'] = int(total_bytes / (1024 * 1024))
        info['free_vram_mb'] = int(free_bytes / (1024 * 1024))
        info['rocm_version'] = getattr(torch.version, 'hip', None)
except Exception as e:
    info['error'] = str(e)
print(json.dumps(info))
" || echo '{{"rocm_available": false}}'"#,
            venv_clean
        );

        let res = crate::wsl::executor::WslExecutor::run_command_output(distro, &test_script).await;

        match res {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                if let Some(last_line) = stdout.lines().last() {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(last_line.trim()) {
                        // Report exactly what the venv's PyTorch measured — no
                        // hardcoded fallback GPU name or VRAM figure. If the venv
                        // has a different card than the developer's own machine (or
                        // detection genuinely failed), `None` is the honest answer;
                        // a fabricated "AMD Radeon RX 7700 XT" here previously made
                        // every user's dashboard show hardware they may not own.
                        return Ok(RocmStatusInfo {
                            is_rocm_available: val["rocm_available"].as_bool().unwrap_or(false),
                            rocm_version: val["rocm_version"].as_str().map(|s| s.to_string()),
                            gpu_device_name: val["gpu_name"].as_str().map(|s| s.to_string()),
                            total_vram_mb: val["total_vram_mb"].as_u64(),
                            free_vram_mb: val["free_vram_mb"].as_u64(),
                            is_hip_available: val["hip"].as_bool().unwrap_or(false),
                        });
                    }
                }
                // The venv's python ran but printed no parseable JSON (e.g. a
                // Python-level crash before `print(json.dumps(info))`). We do not
                // know the real ROCm/VRAM state here, so report "unavailable"
                // rather than fabricating a successful-looking result.
                Ok(RocmStatusInfo {
                    is_rocm_available: false,
                    rocm_version: None,
                    gpu_device_name: None,
                    total_vram_mb: None,
                    free_vram_mb: None,
                    is_hip_available: false,
                })
            }
            _ => {
                // If python or ROCm failed in test
                Ok(RocmStatusInfo {
                    is_rocm_available: false,
                    rocm_version: None,
                    gpu_device_name: None,
                    total_vram_mb: None,
                    free_vram_mb: None,
                    is_hip_available: false,
                })
            }
        }
    }

    /// Decodes potential UTF-16LE or UTF-8 output from wsl.exe.
    pub fn decode_wsl_output(bytes: &[u8]) -> String {
        if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
            // UTF-16LE BOM
            let (chunks, _) = bytes[2..].as_chunks::<2>();
            let u16_vec: Vec<u16> = chunks
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16_vec)
        } else if bytes.iter().filter(|&&b| b == 0).count() > bytes.len() / 4 {
            // Likely UTF-16LE without BOM
            let (chunks, _) = bytes.as_chunks::<2>();
            let u16_vec: Vec<u16> = chunks
                .iter()
                .map(|c| u16::from_le_bytes([c[0], c[1]]))
                .collect();
            String::from_utf16_lossy(&u16_vec)
        } else {
            String::from_utf8_lossy(bytes).to_string()
        }
    }

    /// Parses the text output of `wsl.exe -l -v`
    pub fn parse_wsl_list_output(text: &str) -> Vec<WslDistroInfo> {
        let mut distros = Vec::new();
        for line in text.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty()
                || line_trimmed.starts_with("NAME")
                || line_trimmed.starts_with("---")
            {
                continue;
            }

            let is_default = line_trimmed.starts_with('*');
            let clean = line_trimmed.trim_start_matches('*').trim();
            let parts: Vec<&str> = clean.split_whitespace().collect();

            if parts.len() >= 3 {
                let name = parts[0].to_string();
                let state = parts[1].to_string();
                let version = parts[2].parse::<u32>().unwrap_or(2);
                distros.push(WslDistroInfo {
                    name,
                    is_default,
                    version,
                    state,
                });
            }
        }
        distros
    }
}
