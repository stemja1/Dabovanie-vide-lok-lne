use serde::{Deserialize, Serialize};
use crate::wsl::bridge::WslBridge;
use crate::wsl::executor::WslExecutor;
use crate::wizard::models_manifest::ModelsManifest;
use anyhow::Result;

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

pub struct DependencyChecker;

impl DependencyChecker {
    pub async fn run_full_check(distro: &str, venv_path: &str, workspace_dir: &str) -> Result<SystemDiagnosticsReport> {
        let mut items = Vec::new();

        // 1. Check WSL2
        let wsl_info = WslBridge::detect_wsl_status(distro).await.unwrap_or_else(|_| {
            crate::wsl::bridge::WslStatusInfo {
                is_wsl_installed: false,
                is_default_version_2: false,
                distros: vec![],
                target_distro_found: false,
                is_target_distro_running: false,
                kernel_version: None,
            }
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
            fix_hint: Some("Spustite inštaláciu WSL2 cez Setup Wizard alebo manuálne 'wsl --install'.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "distro_ubuntu_24_04".to_string(),
            title: format!("Distribúcia WSL ({})", distro),
            description: "Cieľové Linuxové prostredie Ubuntu 24.04 LTS".to_string(),
            category: "wsl".to_string(),
            is_installed: wsl_info.target_distro_found,
            version_detected: if wsl_info.target_distro_found { Some("Ubuntu 24.04 (WSL2)".to_string()) } else { None },
            is_critical: true,
            error_message: if !wsl_info.target_distro_found {
                Some(format!("Distribúcia '{}' nebola nájdená vo WSL.", distro))
            } else {
                None
            },
            fix_hint: Some(format!("Nainštalujte distribúciu cez 'wsl --install -d {}'", distro)),
        });

        // If WSL is not installed or distro not found, return early
        if !wsl_info.is_wsl_installed || !wsl_info.target_distro_found {
            return Ok(Self::build_report(items, false));
        }

        // 2. Check System packages in WSL (ffmpeg, git, python3-venv, libsndfile1)
        let check_sys_cmd = r#"
ffmpeg -version >/dev/null 2>&1 && echo "FFMPEG_OK" || echo "FFMPEG_MISSING";
git --version >/dev/null 2>&1 && echo "GIT_OK" || echo "GIT_MISSING";
python3 -m venv --help >/dev/null 2>&1 && echo "VENV_OK" || echo "VENV_MISSING";
dpkg -l | grep -q libsndfile1 && echo "SNDFILE_OK" || echo "SNDFILE_MISSING";
"#;
        let sys_res = WslExecutor::run_command_output(distro, &check_sys_cmd.replace('\n', " ")).await;
        let sys_stdout = sys_res.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();

        let ffmpeg_ok = sys_stdout.contains("FFMPEG_OK");
        let git_ok = sys_stdout.contains("GIT_OK");
        let venv_ok = sys_stdout.contains("VENV_OK");
        let sndfile_ok = sys_stdout.contains("SNDFILE_OK");

        items.push(DependencyCheckItem {
            id: "pkg_ffmpeg".to_string(),
            title: "FFmpeg Media Framework".to_string(),
            description: "Nástroj na extrakciu audia, strih, demuxing a záverečný video muxing".to_string(),
            category: "system".to_string(),
            is_installed: ffmpeg_ok,
            version_detected: if ffmpeg_ok { Some("FFmpeg 6.x/7.x".to_string()) } else { None },
            is_critical: true,
            error_message: if !ffmpeg_ok { Some("FFmpeg nie je nainštalovaný v Ubuntu prostredí.".to_string()) } else { None },
            fix_hint: Some("Spustite 'sudo apt update && sudo apt install -y ffmpeg'.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "pkg_git".to_string(),
            title: "Git CLI".to_string(),
            description: "Správa verzií a klonovanie AI repozitárov (LatentSync, MuseTalk)".to_string(),
            category: "system".to_string(),
            is_installed: git_ok,
            version_detected: if git_ok { Some("Git CLI".to_string()) } else { None },
            is_critical: true,
            error_message: if !git_ok { Some("Git nie je nainštalovaný v Ubuntu prostredí.".to_string()) } else { None },
            fix_hint: Some("Spustite 'sudo apt install -y git'.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "pkg_python_tools".to_string(),
            title: "Python 3 & VirtualEnv & libsndfile".to_string(),
            description: "Systémový Python3 a podpora virtuálnych prostredí".to_string(),
            category: "system".to_string(),
            is_installed: venv_ok && sndfile_ok,
            version_detected: if venv_ok { Some("Python 3.12+".to_string()) } else { None },
            is_critical: true,
            error_message: if !(venv_ok && sndfile_ok) { Some("Chýbajú systémové balíky python3-venv alebo libsndfile1.".to_string()) } else { None },
            fix_hint: Some("Spustite 'sudo apt install -y python3-pip python3-venv libsndfile1'.".to_string()),
        });

        // 3. Check Python venv and PyTorch ROCm
        let python_bin = format!("{}/bin/python", venv_path.trim_end_matches('/'));
        let check_py_cmd = format!(
            r#"test -f {0} && {0} -c "
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
            python_bin
        );

        let py_res = WslExecutor::run_command_output(distro, &check_py_cmd).await;
        let py_stdout = py_res.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();

        let venv_exists = !py_stdout.contains("VENV_NOT_FOUND");
        let torch_ok = py_stdout.contains("TORCH_OK");
        let torch_rocm_ok = py_stdout.contains("GPU=True") || py_stdout.contains("HIP=6.");
        let packages_ok = py_stdout.contains("PACKAGES_OK");

        items.push(DependencyCheckItem {
            id: "python_venv".to_string(),
            title: "Python Virtuálne Prostredie".to_string(),
            description: format!("Izolované prostredie v '{}'", venv_path),
            category: "python".to_string(),
            is_installed: venv_exists,
            version_detected: if venv_exists { Some("Virtualenv aktívny".to_string()) } else { None },
            is_critical: true,
            error_message: if !venv_exists { Some("Virtuálne prostredie neexistuje.".to_string()) } else { None },
            fix_hint: Some("Setup Wizard vytvorí venv a nainštaluje potrebné knižnice automaticky.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "pytorch_rocm".to_string(),
            title: "PyTorch s AMD ROCm podporou".to_string(),
            description: "Akcelerácia na AMD Radeon RX 7700 XT cez ROCm 6.2/6.4".to_string(),
            category: "python".to_string(),
            is_installed: torch_ok && torch_rocm_ok,
            version_detected: if torch_ok { Some("PyTorch ROCm".to_string()) } else { None },
            is_critical: true,
            error_message: if !torch_rocm_ok { Some("PyTorch nemá detegovanú ROCm / GPU akceleráciu.".to_string()) } else { None },
            fix_hint: Some("Inštalujte PyTorch cez ROCm index: pip install torch --index-url https://download.pytorch.org/whl/rocm6.2".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "dubbing_python_packages".to_string(),
            title: "Dabingové knižnice (open_dubbing, piper, transformers)".to_string(),
            description: "Spracovanie ASR, MT a TTS modelov".to_string(),
            category: "python".to_string(),
            is_installed: packages_ok,
            version_detected: if packages_ok { Some("Inštalované".to_string()) } else { None },
            is_critical: true,
            error_message: if !packages_ok { Some("Chýbajú Python balíčky pre pipeline.".to_string()) } else { None },
            fix_hint: Some("Spustite inštaláciu závislostí cez Setup Wizard.".to_string()),
        });

        // 4. Check Repositories (LatentSync 1.5 and MuseTalk)
        let check_repos_cmd = format!(
            r#"test -d {0}/latentsync && echo "LATENTSYNC_OK" || echo "LATENTSYNC_MISSING";
test -d {0}/musetalk && echo "MUSETALK_OK" || echo "MUSETALK_MISSING";"#,
            workspace_dir
        );
        let repo_res = WslExecutor::run_command_output(distro, &check_repos_cmd).await;
        let repo_stdout = repo_res.map(|o| String::from_utf8_lossy(&o.stdout).to_string()).unwrap_or_default();

        let latentsync_ok = repo_stdout.contains("LATENTSYNC_OK");
        let musetalk_ok = repo_stdout.contains("MUSETALK_OK");

        items.push(DependencyCheckItem {
            id: "repo_latentsync".to_string(),
            title: "LatentSync 1.5 Repozitár & SDPA Patch".to_string(),
            description: "Primárny UNet lip-sync engine (~7 GB VRAM) s ROCm natívnou attention".to_string(),
            category: "repos".to_string(),
            is_installed: latentsync_ok,
            version_detected: if latentsync_ok { Some("LatentSync 1.5".to_string()) } else { None },
            is_critical: true,
            error_message: if !latentsync_ok { Some("Repozitár LatentSync nie je naklonovaný.".to_string()) } else { None },
            fix_hint: Some("Klonujte repozitár LatentSync do workspace zložky cez Setup Wizard.".to_string()),
        });

        items.push(DependencyCheckItem {
            id: "repo_musetalk".to_string(),
            title: "MuseTalk Repozitár (Odľahčený Fallback)".to_string(),
            description: "Záložný rýchly lip-sync engine s nízkou spotrebou (~4.5 GB VRAM)".to_string(),
            category: "repos".to_string(),
            is_installed: musetalk_ok,
            version_detected: if musetalk_ok { Some("MuseTalk Engine".to_string()) } else { None },
            is_critical: false,
            error_message: if !musetalk_ok { Some("MuseTalk repozitár nie je naklonovaný.".to_string()) } else { None },
            fix_hint: Some("Klonujte repozitár MuseTalk pre zaistenie OOM fallbacku.".to_string()),
        });

        // 5. Check Models
        let all_models = ModelsManifest::get_all_models();
        for model in all_models {
            let check_model_cmd = format!("test -e {0}/{1} && echo 'MODEL_OK' || echo 'MODEL_MISSING'", workspace_dir, model.local_relative_path);
            let m_res = WslExecutor::run_command_output(distro, &check_model_cmd).await;
            let m_ok = m_res.map(|o| String::from_utf8_lossy(&o.stdout).contains("MODEL_OK")).unwrap_or(false);

            items.push(DependencyCheckItem {
                id: format!("model_{}", model.id),
                title: model.name,
                description: format!("{} (veľkosť cca {} MB, {})", model.description, model.approximate_size_mb, model.license),
                category: "models".to_string(),
                is_installed: m_ok,
                version_detected: if m_ok { Some("Prítomný".to_string()) } else { None },
                is_critical: model.is_required_for_mvp,
                error_message: if !m_ok && model.is_required_for_mvp {
                    Some("Požadovaný modelový checkpoint chýba.".to_string())
                } else {
                    None
                },
                fix_hint: Some(format!("Stiahnite model cez záložku 'Sťahovanie modelov' v Setup Wizarde.")),
            });
        }

        Ok(Self::build_report(items, false))
    }

    fn build_report(items: Vec<DependencyCheckItem>, reboot_pending: bool) -> SystemDiagnosticsReport {
        let total = items.len() as f32;
        let installed = items.iter().filter(|i| i.is_installed).count() as f32;
        let percentage = if total > 0.0 { (installed / total) * 100.0 } else { 0.0 };
        
        let critical_ok = items.iter().filter(|i| i.is_critical).all(|i| i.is_installed);
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
