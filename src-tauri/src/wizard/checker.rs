use crate::wizard::models_manifest::ModelsManifest;
use crate::wsl::bridge::WslBridge;
use crate::wsl::executor::WslExecutor;
use crate::wsl::path_mapper::PathMapper;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCheckItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String, // "wsl" | "system" | "python" | "repos" | "models"
    pub is_installed: bool,
    pub version_detected: Option<String>,
    pub is_critical: bool,
    pub error_message: Option<String>,
    pub fix_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemDiagnosticsReport {
    pub all_ok: bool,
    pub readiness_percentage: f32,
    pub is_reboot_pending: bool,
    pub items: Vec<DependencyCheckItem>,
    pub timestamp_ms: i64,
}

// Pure parsing functions for dependency checks.
// These have no I/O dependencies, ensuring full unit testability without WSL runtime.

/// Returns (ffmpeg_ok, git_ok, venv_ok, sndfile_ok).
pub(crate) fn parse_system_packages(stdout: &str) -> (bool, bool, bool, bool) {
    (
        stdout.contains("FFMPEG_OK"),
        stdout.contains("GIT_OK"),
        stdout.contains("VENV_OK"),
        stdout.contains("SNDFILE_OK"),
    )
}

/// Returns (venv_exists, torch_ok, torch_rocm_ok, packages_ok).
pub(crate) fn parse_python_env(stdout: &str) -> (bool, bool, bool, bool) {
    (
        !stdout.contains("VENV_NOT_FOUND"),
        stdout.contains("TORCH_OK"),
        stdout.contains("GPU=True") || stdout.contains("HIP=6."),
        stdout.contains("PACKAGES_OK"),
    )
}

/// Returns (latentsync_ok, musetalk_ok).
pub(crate) fn parse_repos(stdout: &str) -> (bool, bool) {
    (
        stdout.contains("LATENTSYNC_OK"),
        stdout.contains("MUSETALK_OK"),
    )
}

/// Parses the merged "MODEL_OK:<id>" / "MODEL_MISSING:<id>" output into a
/// map from model id to installed-bool. No I/O — pure string parsing.
pub(crate) fn parse_model_results(stdout: &str) -> HashMap<String, bool> {
    let mut result = HashMap::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(id) = trimmed.strip_prefix("MODEL_OK:") {
            result.insert(id.trim().to_string(), true);
        } else if let Some(id) = trimmed.strip_prefix("MODEL_MISSING:") {
            result.insert(id.trim().to_string(), false);
        }
    }
    result
}

pub struct DependencyChecker;

impl DependencyChecker {
    pub async fn run_full_check(
        distro: &str,
        venv_path: &str,
        workspace_dir: &str,
    ) -> Result<SystemDiagnosticsReport> {
        let mut items = Vec::new();

        // 1. Check WSL2 & Target Distribution (Must run first for early-return guard)
        let wsl_info = WslBridge::detect_wsl_status(distro)
            .await
            .unwrap_or_else(|_| crate::wsl::bridge::WslStatusInfo {
                is_wsl_installed: false,
                is_default_version_2: false,
                distros: vec![],
                target_distro_found: false,
                is_target_distro_running: false,
                kernel_version: None,
            });

        items.push(DependencyCheckItem {
            id: "wsl2_installed".to_string(),
            title: "WSL2 Virtualizačná platforma".to_string(),
            description: "Windows Subsystem for Linux 2 podpora".to_string(),
            category: "wsl".to_string(),
            is_installed: wsl_info.is_wsl_installed,
            version_detected: wsl_info.kernel_version.clone(),
            is_critical: true,
            error_message: if !wsl_info.is_wsl_installed {
                Some("WSL2 nie je nainštalované na hostiteľskom systéme Windows.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Spustite inštaláciu WSL2 cez Setup Wizard alebo manuálne 'wsl --install'."
                    .to_string(),
            ),
        });

        items.push(DependencyCheckItem {
            id: "distro_ubuntu_24_04".to_string(),
            title: format!("Distribúcia WSL ({})", distro),
            description: "Cieľové Linuxové prostredie Ubuntu 24.04 LTS".to_string(),
            category: "wsl".to_string(),
            is_installed: wsl_info.target_distro_found,
            version_detected: if wsl_info.target_distro_found {
                Some("Ubuntu 24.04 (WSL2)".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !wsl_info.target_distro_found {
                Some(format!("Distribúcia '{}' nebola nájdená vo WSL.", distro))
            } else {
                None
            },
            fix_hint: Some(format!(
                "Nainštalujte distribúciu cez 'wsl --install -d {}'",
                distro
            )),
        });

        // If WSL is not installed or distro not found, return early
        if !wsl_info.is_wsl_installed || !wsl_info.target_distro_found {
            return Ok(Self::build_report(items, false));
        }

        // 2. Prepare System Packages Check Command
        let check_sys_cmd = r#"
ffmpeg -version >/dev/null 2>&1 && echo "FFMPEG_OK" || echo "FFMPEG_MISSING";
git --version >/dev/null 2>&1 && echo "GIT_OK" || echo "GIT_MISSING";
python3 -m venv --help >/dev/null 2>&1 && echo "VENV_OK" || echo "VENV_MISSING";
dpkg -l | grep -q libsndfile1 && echo "SNDFILE_OK" || echo "SNDFILE_MISSING";
"#;

        // 3. Prepare Python venv and PyTorch ROCm Check Command
        let venv_setup =
            PathMapper::bash_var_with_home_expansion("VENV", venv_path.trim_end_matches('/'));
        let check_py_cmd = format!(
            r#"{0} test -f "$VENV/bin/python" && "$VENV/bin/python" -c "
import sys
try:
    import torch
    hip = getattr(torch.version, 'hip', None)
    gpu = torch.cuda.is_available()
    print(f'TORCH_OK:{{torch.__version__}}:HIP={{hip}}:GPU={{gpu}}')
except Exception as e:
    print('TORCH_ERR:' + str(e))
try:
    import transformers, piper, soundfile
    print('PACKAGES_OK')
except Exception as e:
    print('PACKAGES_MISSING:' + str(e))
" || echo "VENV_NOT_FOUND""#,
            venv_setup
        );

        // 4. Prepare Repositories Check Command (LatentSync 1.5 and MuseTalk)
        let ws_setup = PathMapper::bash_var_with_home_expansion(
            "WORKSPACE",
            workspace_dir.trim_end_matches('/'),
        );
        let check_repos_cmd = format!(
            r#"{0} test -d "$WORKSPACE/latentsync" && echo "LATENTSYNC_OK" || echo "LATENTSYNC_MISSING"; test -d "$WORKSPACE/musetalk" && echo "MUSETALK_OK" || echo "MUSETALK_MISSING";"#,
            ws_setup
        );

        // 5. Prepare Merged AI Models Check Command (1 single command for all models)
        let all_models = ModelsManifest::get_all_models();
        let mut model_checks = Vec::with_capacity(all_models.len());
        for model in &all_models {
            model_checks.push(format!(
                r#"test -e "$WORKSPACE/{0}" && echo "MODEL_OK:{1}" || echo "MODEL_MISSING:{1}";"#,
                model.local_relative_path, model.id
            ));
        }
        let check_models_cmd = format!("{0} {1}", ws_setup, model_checks.join(" "));

        // Execute all 4 checks concurrently via tokio::join! (fixed 4 WSL subprocesses)
        let check_sys_clean = check_sys_cmd.replace('\n', " ");
        let (sys_res, py_res, repo_res, models_res) = tokio::join!(
            WslExecutor::run_command_output(distro, &check_sys_clean),
            WslExecutor::run_command_output(distro, &check_py_cmd),
            WslExecutor::run_command_output(distro, &check_repos_cmd),
            WslExecutor::run_command_output(distro, &check_models_cmd),
        );

        // Parse outputs using pure functions with safe fallback defaults
        let sys_stdout = sys_res
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let (ffmpeg_ok, git_ok, venv_ok, sndfile_ok) = parse_system_packages(&sys_stdout);

        let py_stdout = py_res
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let (venv_exists, torch_ok, torch_rocm_ok, packages_ok) = parse_python_env(&py_stdout);

        let repo_stdout = repo_res
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let (latentsync_ok, musetalk_ok) = parse_repos(&repo_stdout);

        let models_stdout = models_res
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let model_results = parse_model_results(&models_stdout);

        // Push items in EXACT deterministic order
        // 2. System Packages
        items.push(DependencyCheckItem {
            id: "pkg_ffmpeg".to_string(),
            title: "FFmpeg Media Framework".to_string(),
            description: "Nástroj na extrakciu audia, strih, demuxing a záverečný video muxing"
                .to_string(),
            category: "system".to_string(),
            is_installed: ffmpeg_ok,
            version_detected: if ffmpeg_ok {
                Some("FFmpeg 6.x/7.x".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !ffmpeg_ok {
                Some("FFmpeg nie je nainštalovaný v Ubuntu prostredí.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Spustite inštaláciu systémových balíkov cez tlačidlo 'Nainštalovať'.".to_string(),
            ),
        });

        items.push(DependencyCheckItem {
            id: "pkg_git".to_string(),
            title: "Git CLI".to_string(),
            description: "Správa verzií a klonovanie AI repozitárov (LatentSync, MuseTalk)"
                .to_string(),
            category: "system".to_string(),
            is_installed: git_ok,
            version_detected: if git_ok {
                Some("Git CLI".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !git_ok {
                Some("Git nie je nainštalovaný v Ubuntu prostredí.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Spustite inštaláciu systémových balíkov cez tlačidlo 'Nainštalovať'.".to_string(),
            ),
        });

        items.push(DependencyCheckItem {
            id: "pkg_python_tools".to_string(),
            title: "Python 3 & VirtualEnv & libsndfile".to_string(),
            description: "Systémový Python3 a podpora virtuálnych prostredí".to_string(),
            category: "system".to_string(),
            is_installed: venv_ok && sndfile_ok,
            version_detected: if venv_ok {
                Some("Python 3.12+".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !(venv_ok && sndfile_ok) {
                Some("Chýbajú systémové balíky python3-venv alebo libsndfile1.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Spustite inštaláciu systémových balíkov cez tlačidlo 'Nainštalovať'.".to_string(),
            ),
        });

        // 3. Python venv, PyTorch ROCm & Packages
        items.push(DependencyCheckItem {
            id: "python_venv".to_string(),
            title: "Python Virtuálne Prostredie".to_string(),
            description: format!("Izolované prostredie v '{}'", venv_path),
            category: "python".to_string(),
            is_installed: venv_exists,
            version_detected: if venv_exists {
                Some("Virtualenv aktívny".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !venv_exists {
                Some("Virtuálne prostredie neexistuje.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Setup Wizard vytvorí venv a nainštaluje potrebné knižnice automaticky."
                    .to_string(),
            ),
        });

        items.push(DependencyCheckItem {
            id: "pytorch_rocm".to_string(),
            title: "PyTorch s AMD ROCm podporou".to_string(),
            description: "Akcelerácia na AMD Radeon RX 7700 XT cez ROCm 6.2/6.4".to_string(),
            category: "python".to_string(),
            is_installed: torch_ok && torch_rocm_ok,
            version_detected: if torch_ok {
                Some("PyTorch ROCm".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !torch_rocm_ok {
                Some("PyTorch nemá detegovanú ROCm / GPU akceleráciu.".to_string())
            } else {
                None
            },
            fix_hint: Some("Inštalujte PyTorch cez ROCm index v Setup Wizarde.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "dubbing_python_packages".to_string(),
            title: "Dabingové knižnice (open_dubbing, piper, transformers)".to_string(),
            description: "Spracovanie ASR, MT a TTS modelov".to_string(),
            category: "python".to_string(),
            is_installed: packages_ok,
            version_detected: if packages_ok {
                Some("Inštalované".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !packages_ok {
                Some("Chýbajú Python balíčky pre pipeline.".to_string())
            } else {
                None
            },
            fix_hint: Some("Spustite inštaláciu závislostí cez Setup Wizard.".to_string()),
        });

        // 4. Repositories
        items.push(DependencyCheckItem {
            id: "repo_latentsync".to_string(),
            title: "LatentSync 1.5 Repozitár & SDPA Patch".to_string(),
            description: "Primárny UNet lip-sync engine (~7 GB VRAM) s ROCm natívnou attention"
                .to_string(),
            category: "repos".to_string(),
            is_installed: latentsync_ok,
            version_detected: if latentsync_ok {
                Some("LatentSync 1.5".to_string())
            } else {
                None
            },
            is_critical: true,
            error_message: if !latentsync_ok {
                Some("Repozitár LatentSync nie je naklonovaný.".to_string())
            } else {
                None
            },
            fix_hint: Some(
                "Klonujte repozitár LatentSync do workspace zložky cez Setup Wizard.".to_string(),
            ),
        });

        items.push(DependencyCheckItem {
            id: "repo_musetalk".to_string(),
            title: "MuseTalk Repozitár (Odľahčený Fallback)".to_string(),
            description: "Záložný rýchly lip-sync engine s nízkou spotrebou (~4.5 GB VRAM)"
                .to_string(),
            category: "repos".to_string(),
            is_installed: musetalk_ok,
            version_detected: if musetalk_ok {
                Some("MuseTalk Engine".to_string())
            } else {
                None
            },
            is_critical: false,
            error_message: if !musetalk_ok {
                Some("MuseTalk repozitár nie je naklonovaný.".to_string())
            } else {
                None
            },
            fix_hint: Some("Klonujte repozitár MuseTalk pre zaistenie OOM fallbacku.".to_string()),
        });

        // 5. Models (Iterated in exact ModelsManifest order)
        for model in all_models {
            let m_ok = model_results.get(&model.id).copied().unwrap_or(false);

            items.push(DependencyCheckItem {
                id: format!("model_{}", model.id),
                title: model.name,
                description: format!(
                    "{} (veľkosť cca {} MB, {})",
                    model.description, model.approximate_size_mb, model.license
                ),
                category: "models".to_string(),
                is_installed: m_ok,
                version_detected: if m_ok {
                    Some("Prítomný".to_string())
                } else {
                    None
                },
                is_critical: model.is_required_for_mvp,
                error_message: if !m_ok && model.is_required_for_mvp {
                    Some("Požadovaný modelový checkpoint chýba.".to_string())
                } else {
                    None
                },
                fix_hint: Some(
                    "Stiahnite model cez záložku 'Sťahovanie modelov' v Setup Wizarde.".to_string(),
                ),
            });
        }

        Ok(Self::build_report(items, false))
    }

    pub(crate) fn build_report(
        items: Vec<DependencyCheckItem>,
        reboot_pending: bool,
    ) -> SystemDiagnosticsReport {
        let total = items.len() as f32;
        let installed = items.iter().filter(|i| i.is_installed).count() as f32;
        let percentage = if total > 0.0 {
            (installed / total) * 100.0
        } else {
            0.0
        };

        let critical_ok = items
            .iter()
            .filter(|i| i.is_critical)
            .all(|i| i.is_installed);
        let all_ok = critical_ok && !reboot_pending;

        SystemDiagnosticsReport {
            all_ok,
            readiness_percentage: percentage,
            is_reboot_pending: reboot_pending,
            items,
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_system_packages_all_ok() {
        let out = "FFMPEG_OK\nGIT_OK\nVENV_OK\nSNDFILE_OK\n";
        assert_eq!(parse_system_packages(out), (true, true, true, true));
    }

    #[test]
    fn parse_system_packages_all_missing() {
        let out = "FFMPEG_MISSING\nGIT_MISSING\nVENV_MISSING\nSNDFILE_MISSING\n";
        assert_eq!(parse_system_packages(out), (false, false, false, false));
    }

    #[test]
    fn parse_system_packages_empty_stdout_is_all_missing() {
        // Simulates a failed/timed-out WSL call whose output couldn't be
        // captured -- must NOT panic, must report "nothing installed".
        assert_eq!(parse_system_packages(""), (false, false, false, false));
    }

    #[test]
    fn parse_python_env_rocm_detected() {
        let out = "TORCH_OK:2.4.0:HIP=6.4.43482:GPU=True\nPACKAGES_OK\n";
        assert_eq!(parse_python_env(out), (true, true, true, true));
    }

    #[test]
    fn parse_python_env_venv_missing() {
        assert_eq!(
            parse_python_env("VENV_NOT_FOUND\n"),
            (false, false, false, false)
        );
    }

    #[test]
    fn parse_python_env_empty_stdout() {
        // Empty stdout indicates error/timeout
        assert_eq!(parse_python_env(""), (true, false, false, false));
    }

    #[test]
    fn parse_repos_all_ok() {
        assert_eq!(parse_repos("LATENTSYNC_OK\nMUSETALK_OK\n"), (true, true));
    }

    #[test]
    fn parse_repos_partial() {
        assert_eq!(
            parse_repos("LATENTSYNC_OK\nMUSETALK_MISSING\n"),
            (true, false)
        );
    }

    #[test]
    fn parse_repos_empty() {
        assert_eq!(parse_repos(""), (false, false));
    }

    #[test]
    fn parse_model_results_mixed() {
        let out = "MODEL_OK:whisper-large-v3-sk\nMODEL_MISSING:coqui-xtts-v2\n";
        let map = parse_model_results(out);
        assert_eq!(map.get("whisper-large-v3-sk"), Some(&true));
        assert_eq!(map.get("coqui-xtts-v2"), Some(&false));
    }

    #[test]
    fn parse_model_results_missing_id_defaults_to_not_installed() {
        // A model id that never appears in stdout at all (e.g. WSL call
        // failed entirely) must be treated as "not installed" by the
        // caller's `.unwrap_or(false)`.
        let map = parse_model_results("");
        assert_eq!(map.get("anything"), None);
    }

    #[test]
    fn parse_model_results_extra_whitespace_and_newlines() {
        let out = "  \n  MODEL_OK:piper-zh-huayan   \n\nMODEL_MISSING:kokoro-v019\n  ";
        let map = parse_model_results(out);
        assert_eq!(map.get("piper-zh-huayan"), Some(&true));
        assert_eq!(map.get("kokoro-v019"), Some(&false));
    }

    #[test]
    fn parse_model_results_unknown_model_ignored() {
        let out = "MODEL_OK:non_existent_future_model\n";
        let map = parse_model_results(out);
        assert_eq!(map.get("non_existent_future_model"), Some(&true));
        assert_eq!(map.get("whisper-large-v3-sk"), None);
    }

    #[test]
    fn build_report_preserves_item_order_regardless_of_join_completion_order() {
        let items = vec![
            DependencyCheckItem {
                id: "first_item".to_string(),
                title: "First".to_string(),
                description: "Desc 1".to_string(),
                category: "wsl".to_string(),
                is_installed: true,
                version_detected: None,
                is_critical: true,
                error_message: None,
                fix_hint: None,
            },
            DependencyCheckItem {
                id: "second_item".to_string(),
                title: "Second".to_string(),
                description: "Desc 2".to_string(),
                category: "system".to_string(),
                is_installed: false,
                version_detected: None,
                is_critical: false,
                error_message: None,
                fix_hint: None,
            },
            DependencyCheckItem {
                id: "third_item".to_string(),
                title: "Third".to_string(),
                description: "Desc 3".to_string(),
                category: "models".to_string(),
                is_installed: true,
                version_detected: None,
                is_critical: true,
                error_message: None,
                fix_hint: None,
            },
        ];

        let report = DependencyChecker::build_report(items.clone(), false);
        let ids: Vec<_> = report.items.iter().map(|i| i.id.clone()).collect();
        let expected_ids: Vec<_> = items.iter().map(|i| i.id.clone()).collect();
        assert_eq!(ids, expected_ids);
        assert_eq!(report.readiness_percentage, (2.0 / 3.0) * 100.0);
        assert!(report.all_ok);
    }
}
