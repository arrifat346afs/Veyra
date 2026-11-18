//! Per-user file picker registration and reversible desktop portal configuration.
mod config;
use config::KEY;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::Write,
    os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

const BUS: &str = "org.freedesktop.impl.portal.desktop.veyra";
const FRONTEND: &str = "org.freedesktop.portal.Desktop";
const PORTAL: &str = include_str!("../../../integration/veyra.portal");
const UNIT: &str = "xdg-desktop-portal.service";

#[derive(Clone, Debug)]
pub struct Environment {
    config_home: PathBuf,
    data_home: PathBuf,
    state_home: PathBuf,
    config_dirs: Vec<PathBuf>,
    data_dirs: Vec<PathBuf>,
    desktops: Vec<String>,
    managed_dir: Option<PathBuf>,
    executable: PathBuf,
}

impl Environment {
    fn from_process() -> Result<Self, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("Home folder is unavailable.")?;
        let get = |name: &str, fallback: PathBuf| {
            std::env::var_os(name)
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or(fallback)
        };
        let dirs = |name: &str, fallback: &str| {
            std::env::split_paths(
                &std::env::var_os(name)
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| fallback.into()),
            )
            .collect()
        };
        Ok(Self {
            config_home: get("XDG_CONFIG_HOME", home.join(".config")),
            data_home: get("XDG_DATA_HOME", home.join(".local/share")),
            state_home: get("XDG_STATE_HOME", home.join(".local/state")),
            config_dirs: dirs("XDG_CONFIG_DIRS", "/etc/xdg"),
            data_dirs: dirs("XDG_DATA_DIRS", "/usr/local/share:/usr/share"),
            desktops: desktop_names(&std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default()),
            managed_dir: std::env::var_os("XDG_DESKTOP_PORTAL_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            executable: std::env::current_exe().map_err(|e| e.to_string())?,
        })
    }
    fn use_frontend_environment(&mut self, pid: u32) {
        let Ok(bytes) = fs::read(format!("/proc/{pid}/environ")) else {
            return;
        };
        let values: HashMap<_, _> = bytes
            .split(|b| *b == 0)
            .filter_map(|entry| {
                let index = entry.iter().position(|b| *b == b'=')?;
                Some((
                    String::from_utf8_lossy(&entry[..index]).into_owned(),
                    std::ffi::OsString::from_vec(entry[index + 1..].to_vec()),
                ))
            })
            .collect();
        for (key, target) in [
            ("XDG_CONFIG_HOME", &mut self.config_home),
            ("XDG_DATA_HOME", &mut self.data_home),
        ] {
            if let Some(value) = values.get(key).filter(|v| !v.is_empty()) {
                *target = PathBuf::from(value);
            }
        }
        if let Some(value) = values.get("XDG_CURRENT_DESKTOP") {
            self.desktops = desktop_names(&value.to_string_lossy());
        }
        self.managed_dir = values
            .get("XDG_DESKTOP_PORTAL_DIR")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);
        if let Some(value) = values.get("XDG_CONFIG_DIRS").filter(|v| !v.is_empty()) {
            self.config_dirs = std::env::split_paths(value).collect();
        }
        if let Some(value) = values.get("XDG_DATA_DIRS").filter(|v| !v.is_empty()) {
            self.data_dirs = std::env::split_paths(value).collect();
        }
    }
    fn state_file(&self) -> PathBuf {
        self.state_home.join("veyra/portal-setup.json")
    }
    fn pending_file(&self) -> PathBuf {
        self.state_home.join("veyra/portal-restart-required")
    }
    fn binary(&self) -> PathBuf {
        self.data_home.join("veyra/portal/veyra")
    }
    fn portal_file(&self) -> PathBuf {
        self.data_home
            .join("xdg-desktop-portal/portals/veyra.portal")
    }
    fn service_file(&self) -> PathBuf {
        self.data_home
            .join(format!("dbus-1/services/{BUS}.service"))
    }
    fn service_text(&self) -> Result<String, String> {
        let path = self.binary();
        let path = path
            .to_str()
            .ok_or("The installation folder is not valid UTF-8.")?;
        if path.contains(['\n', '\r']) {
            return Err("The installation folder contains a line break.".into());
        }
        let quoted = path.replace('\\', "\\\\").replace('"', "\\\"");
        Ok(format!(
            "[D-BUS Service]\nName={BUS}\nExec=\"{quoted}\" --portal\n"
        ))
    }
    fn config_names(&self) -> Vec<String> {
        self.desktops
            .iter()
            .map(|d| format!("{d}-portals.conf"))
            .chain(std::iter::once("portals.conf".into()))
            .collect()
    }
    fn config_locations(&self) -> Vec<PathBuf> {
        if let Some(dir) = &self.managed_dir {
            return vec![dir.clone()];
        }
        std::iter::once(self.config_home.clone())
            .chain(self.config_dirs.clone())
            .chain([PathBuf::from("/etc"), self.data_home.clone()])
            .chain(self.data_dirs.clone())
            .chain([PathBuf::from("/usr/share")])
            .map(|p| p.join("xdg-desktop-portal"))
            .collect()
    }
    fn effective_config(&self) -> Option<(PathBuf, String)> {
        for directory in self.config_locations() {
            for name in self.config_names() {
                let path = directory.join(name);
                if let Ok(text) = fs::read_to_string(&path) {
                    if text.lines().any(|line| line.trim() == "[preferred]") {
                        return Some((path, text));
                    }
                }
            }
        }
        None
    }
    fn target_config(&self) -> PathBuf {
        let dir = self.config_home.join("xdg-desktop-portal");
        self.config_names()
            .iter()
            .map(|name| dir.join(name))
            .find(|p| p.is_file())
            .unwrap_or_else(|| dir.join(&self.config_names()[0]))
    }
}
fn desktop_names(value: &str) -> Vec<String> {
    value
        .split(':')
        .filter(|v| {
            !v.is_empty()
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
        .map(str::to_ascii_lowercase)
        .collect()
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub enabled: bool,
    pub managed: bool,
    pub restart_needed: bool,
    pub portal_registered: bool,
    pub service_registered: bool,
    pub frontend_running: bool,
    pub current_backend: String,
    pub unavailable: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    path: PathBuf,
    before: Option<String>,
    installed: String,
}
#[derive(Serialize, Deserialize)]
struct Installation {
    config: Snapshot,
    portal: Snapshot,
    service: Snapshot,
    binary: PathBuf,
    binary_created: bool,
    pending_restart: bool,
    enabled: bool,
}
#[derive(Clone, Copy)]
pub enum Action {
    Refresh,
    Enable,
    Disable,
    Restart,
}

fn read_installation(env: &Environment) -> Result<Option<Installation>, String> {
    match fs::read(env.state_file()) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("Could not read file picker setup history: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
fn save_installation(env: &Environment, state: &Installation) -> Result<(), String> {
    atomic_write(
        &env.state_file(),
        &serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?,
        0o600,
    )
}
fn read_optional(path: &Path) -> Result<Option<String>, String> {
    refuse_symlink(path)?;
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}
fn refuse_symlink(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(format!(
            "{} is managed through a symbolic link. Change its source configuration instead.",
            path.display()
        ));
    }
    Ok(())
}
fn atomic_write(path: &Path, bytes: &[u8], mode: u32) -> Result<(), String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    refuse_symlink(path)?;
    let mode = fs::metadata(path)
        .map(|m| m.permissions().mode() & 0o777)
        .unwrap_or(mode);
    let parent = path.parent().ok_or("Missing parent folder")?;
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(
        ".veyra-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.set_permissions(fs::Permissions::from_mode(mode))?;
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|e| format!("{}: {e}", path.display()))
}
fn remove(path: &Path) -> Result<(), String> {
    refuse_symlink(path)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
fn restore(snapshot: &Snapshot, config: bool) -> Result<bool, String> {
    let current = read_optional(&snapshot.path)?;
    if current.as_deref() == Some(&snapshot.installed) {
        if let Some(before) = &snapshot.before {
            atomic_write(&snapshot.path, before.as_bytes(), 0o644)?;
        } else {
            remove(&snapshot.path)?;
        }
        return Ok(false);
    }
    if config {
        if let Some(current) = current {
            if config::key_line(&current) == config::key_line(&snapshot.installed) {
                let original_line = snapshot.before.as_deref().and_then(config::key_line);
                let updated = config::set_line(&current, original_line.as_deref());
                atomic_write(&snapshot.path, updated.as_bytes(), 0o644)?;
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn install(env: &Environment) -> Result<(), String> {
    if env.managed_dir.is_some() {
        return Err("The desktop portal list is managed by your system.".into());
    }
    let previous = read_installation(env)?;
    let previous = previous.filter(|state| state.enabled);
    let path = previous
        .as_ref()
        .map(|s| s.config.path.clone())
        .unwrap_or_else(|| env.target_config());
    let current = read_optional(&path)?;
    let source = current
        .clone()
        .or_else(|| env.effective_config().map(|(_, text)| text))
        .unwrap_or_else(|| "[preferred]\ndefault=*\n".into());
    let fallback = config::value(&source, KEY)
        .or_else(|| config::value(&source, "default"))
        .unwrap_or_else(|| "*".into());
    let fallback: Vec<_> = fallback
        .split(';')
        .map(str::trim)
        .filter(|name| !name.is_empty() && *name != "veyra")
        .collect();
    let line = format!(
        "{KEY}=veyra;{};",
        if fallback.is_empty() {
            "*".into()
        } else {
            fallback.join(";")
        }
    );
    let config_text = config::set_line(&source, Some(&line));
    let mut state = if let Some(mut previous) = previous {
        previous.config.installed = config_text;
        previous.service.installed = env.service_text()?;
        previous.pending_restart = true;
        previous
    } else {
        Installation {
            config: Snapshot {
                path,
                before: current,
                installed: config_text,
            },
            portal: Snapshot {
                path: env.portal_file(),
                before: read_optional(&env.portal_file())?,
                installed: PORTAL.into(),
            },
            service: Snapshot {
                path: env.service_file(),
                before: read_optional(&env.service_file())?,
                installed: env.service_text()?,
            },
            binary: env.binary(),
            binary_created: !env.binary().exists(),
            pending_restart: true,
            enabled: true,
        }
    };
    refuse_symlink(&state.binary)?;
    if !state.binary_created && read_installation(env)?.is_none() {
        return Err("The file picker installation folder already contains a binary. Choose another installation or remove it first.".into());
    }
    // Save recovery information before changing any desktop configuration.
    save_installation(env, &state)?;
    let result = (|| {
        let parent = state.binary.parent().unwrap();
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let temporary = parent.join(format!(".veyra-install-{}", std::process::id()));
        let copy_result = (|| -> std::io::Result<()> {
            let mut source = fs::File::open(&env.executable)?;
            let mut target = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            std::io::copy(&mut source, &mut target)?;
            target.set_permissions(fs::Permissions::from_mode(0o755))?;
            target.sync_all()?;
            fs::rename(&temporary, &state.binary)
        })();
        if copy_result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        copy_result.map_err(|e| e.to_string())?;
        atomic_write(&state.portal.path, state.portal.installed.as_bytes(), 0o644)?;
        atomic_write(
            &state.service.path,
            state.service.installed.as_bytes(),
            0o644,
        )?;
        atomic_write(&state.config.path, state.config.installed.as_bytes(), 0o644)
    })();
    if let Err(error) = result {
        let _ = restore(&state.config, true);
        let _ = restore(&state.portal, false);
        let _ = restore(&state.service, false);
        state.enabled = false;
        let _ = save_installation(env, &state);
        return Err(error);
    }
    Ok(())
}

fn uninstall(env: &Environment) -> Result<bool, String> {
    if env.managed_dir.is_some() {
        return Err("The desktop portal list is managed by your system.".into());
    }
    let Some(mut state) = read_installation(env)? else {
        // Also support registrations made by the optional command-line installer.
        let path = env.target_config();
        if let Some(text) = read_optional(&path)? {
            if let Some(value) = config::value(&text, KEY) {
                let remaining: Vec<_> = value
                    .split(';')
                    .map(str::trim)
                    .filter(|name| !name.is_empty() && *name != "veyra")
                    .collect();
                let line =
                    (!remaining.is_empty()).then(|| format!("{KEY}={};", remaining.join(";")));
                atomic_write(
                    &path,
                    config::set_line(&text, line.as_deref()).as_bytes(),
                    0o644,
                )?;
            }
        }
        return Ok(false);
    };
    let mut kept = restore(&state.config, true)?;
    kept |= restore(&state.portal, false)?;
    kept |= restore(&state.service, false)?;
    // Keep the stable binary for active picker requests and later re-enabling.
    state.enabled = false;
    state.pending_restart = true;
    save_installation(env, &state)?;
    Ok(kept)
}

fn inspect(env: &Environment) -> Result<Report, String> {
    let text = env
        .effective_config()
        .map(|(_, text)| text)
        .unwrap_or_default();
    let backend = config::value(&text, KEY)
        .or_else(|| config::value(&text, "default"))
        .unwrap_or_default();
    let first = backend
        .split(';')
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or("");
    let mut portal_dirs = vec![env.data_home.join("xdg-desktop-portal/portals")];
    portal_dirs.extend(
        env.data_dirs
            .iter()
            .map(|p| p.join("xdg-desktop-portal/portals")),
    );
    portal_dirs.push("/usr/share/xdg-desktop-portal/portals".into());
    if let Some(dir) = &env.managed_dir {
        portal_dirs = vec![dir.clone()];
    }
    let portal_registered = portal_dirs.iter().any(|p| {
        fs::read_to_string(p.join("veyra.portal")).is_ok_and(|s| s.contains(BUS) && s.contains(KEY))
    });
    let service_registered = std::iter::once(env.data_home.clone())
        .chain(env.data_dirs.clone())
        .filter_map(|p| fs::read_to_string(p.join(format!("dbus-1/services/{BUS}.service"))).ok())
        .next()
        .is_some_and(|text| {
            text.lines()
                .find_map(|line| line.strip_prefix("Exec="))
                .and_then(|exec| shell_words::split(exec).ok())
                .and_then(|args| args.first().cloned())
                .is_some_and(|path| {
                    fs::metadata(path)
                        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
                })
        });
    let pending =
        env.pending_file().is_file() || read_installation(env)?.is_some_and(|s| s.pending_restart);
    Ok(Report {
        enabled: first == "veyra" && portal_registered && service_registered,
        managed: env.managed_dir.is_some(),
        restart_needed: pending,
        portal_registered,
        service_registered,
        current_backend: if first.is_empty() {
            "Desktop default".into()
        } else {
            first.into()
        },
        ..Default::default()
    })
}

pub fn perform(action: Action) -> Result<(Report, Option<String>), String> {
    let mut env = Environment::from_process()?;
    let connection = match zbus::blocking::Connection::session() {
        Ok(connection) => connection,
        Err(error) => {
            return Ok((
                Report {
                    unavailable: Some(format!("Cannot connect to your desktop session: {error}")),
                    ..Default::default()
                },
                None,
            ));
        }
    };
    let bus = zbus::blocking::fdo::DBusProxy::new(&connection).map_err(|e| e.to_string())?;
    let frontend_pid = bus
        .get_connection_unix_process_id(FRONTEND.try_into().unwrap())
        .ok();
    if let Some(pid) = frontend_pid {
        env.use_frontend_environment(pid);
    }
    let mut message = None;
    match action {
        Action::Refresh => {}
        Action::Enable => {
            install(&env)?;
            atomic_write(&env.pending_file(), b"restart required\n", 0o600)?;
            let _ = bus.reload_config();
            message =
                Some("Veyra is selected. Restart the portal service to apply the change.".into());
        }
        Action::Disable => {
            let kept = uninstall(&env)?;
            atomic_write(&env.pending_file(), b"restart required\n", 0o600)?;
            let _ = bus.reload_config();
            message = Some(
                if kept {
                    "Disabled Veyra. Settings edited outside Veyra were preserved."
                } else {
                    "Your previous file picker settings have been restored."
                }
                .into(),
            );
        }
        Action::Restart => {
            restart(frontend_pid)?;
            remove(&env.pending_file())?;
            if let Some(mut state) = read_installation(&env)? {
                state.pending_restart = false;
                save_installation(&env, &state)?;
            }
            message = Some(
                "Portal service restarted. Applications can now use your selected file picker."
                    .into(),
            );
        }
    }
    let mut report = inspect(&env)?;
    let frontend_activatable = std::iter::once(env.data_home.clone())
        .chain(env.data_dirs.clone())
        .any(|dir| {
            dir.join("dbus-1/services/org.freedesktop.portal.Desktop.service")
                .is_file()
        });
    if frontend_pid.is_none() && !frontend_activatable {
        report.unavailable =
            Some("Install xdg-desktop-portal to use Veyra as your file picker.".into());
    }
    report.frontend_running = bus
        .name_has_owner(FRONTEND.try_into().unwrap())
        .unwrap_or(false);
    // Compare file changes with the frontend process start time after a manual
    // restart or a later login, so a stale setup marker does not keep prompting.
    if report.restart_needed {
        if let (Some(pid), Ok(metadata)) = (
            bus.get_connection_unix_process_id(FRONTEND.try_into().unwrap())
                .ok(),
            fs::metadata(if env.pending_file().is_file() {
                env.pending_file()
            } else {
                env.state_file()
            }),
        ) {
            if let (Some(started), Ok(modified)) = (process_started(pid), metadata.modified()) {
                if started > modified {
                    report.restart_needed = false;
                }
            }
        }
    }
    Ok((report, message))
}
fn process_started(pid: u32) -> Option<SystemTime> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let after = stat.rsplit_once(") ")?.1;
    let ticks: u64 = after.split_whitespace().nth(19)?.parse().ok()?;
    let boot: u64 = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .parse()
        .ok()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(boot) + Duration::from_millis(ticks * 10))
}
fn restart(pid: Option<u32>) -> Result<(), String> {
    match Command::new("systemctl")
        .args(["--user", "restart", UNIT])
        .output()
    {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "Could not restart the portal service: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(pid) = pid {
                // Recheck its executable before signalling a process from a past snapshot.
                let executable =
                    fs::read_link(format!("/proc/{pid}/exe")).map_err(|e| e.to_string())?;
                if executable
                    .file_name()
                    .is_none_or(|name| name != "xdg-desktop-portal")
                {
                    return Err("The portal process changed. Refresh and try again.".into());
                }
                if unsafe { libc::kill(pid as i32, libc::SIGTERM) } != 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
            }
            Ok(())
        }
        Err(error) => Err(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
