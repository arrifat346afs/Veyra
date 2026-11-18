use super::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    env: Environment,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "veyra-setup-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("source-binary");
        fs::write(&executable, b"fixture executable").unwrap();
        let env = Environment {
            config_home: root.join("config"),
            data_home: root.join("data"),
            state_home: root.join("state"),
            config_dirs: vec![root.join("system-config")],
            data_dirs: vec![root.join("system-data")],
            desktops: vec!["hyprland".into()],
            managed_dir: None,
            executable,
        };
        Self { root, env }
    }
    fn write(&self, path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn enable_disable_restores_existing_preference_and_keeps_other_portals() {
    let fixture = Fixture::new();
    let env = &fixture.env;
    let path = env.target_config();
    let original = format!(
        "# User choices\n[preferred]\ndefault=hyprland;gtk;\n{KEY}=dfiles;gtk;\norg.freedesktop.impl.portal.ScreenCast=hyprland\n"
    );
    fixture.write(&path, &original);
    install(env).unwrap();
    install(env).unwrap();
    let enabled = fs::read_to_string(&path).unwrap();
    assert_eq!(config::value(&enabled, KEY).unwrap(), "veyra;dfiles;gtk;");
    assert!(enabled.contains("ScreenCast=hyprland"));
    assert!(inspect(env).unwrap().enabled);
    assert!(env.binary().is_file());
    assert_eq!(fs::read(env.binary()).unwrap(), b"fixture executable");
    assert!(!uninstall(env).unwrap());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!env.portal_file().exists());
    assert!(!env.service_file().exists());
    install(env).unwrap();
    assert!(inspect(env).unwrap().enabled);
    uninstall(env).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
}
#[test]
fn fresh_configuration_inherits_desktop_defaults_and_is_removed_on_disable() {
    let fixture = Fixture::new();
    let env = &fixture.env;
    fixture.write(
        &env.config_dirs[0].join("xdg-desktop-portal/hyprland-portals.conf"),
        "[preferred]\ndefault=hyprland;gtk;\norg.freedesktop.impl.portal.ScreenCast=hyprland\n",
    );
    install(env).unwrap();
    let path = env.target_config();
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("default=hyprland;gtk;"));
    assert!(text.contains("FileChooser=veyra;hyprland;gtk;"));
    uninstall(env).unwrap();
    assert!(!path.exists());
}
#[test]
fn disabling_preserves_later_edits_and_user_service_files() {
    let fixture = Fixture::new();
    let env = &fixture.env;
    let path = env.target_config();
    let original = format!("[preferred]\n{KEY}=dfiles;gtk;\n");
    fixture.write(&path, &original);
    fixture.write(&env.service_file(), "[D-BUS Service]\nName=old-service\n");
    install(env).unwrap();
    let edited =
        fs::read_to_string(&path).unwrap() + "org.freedesktop.impl.portal.ScreenCast=hyprland\n";
    fs::write(&path, edited).unwrap();
    uninstall(env).unwrap();
    let restored = fs::read_to_string(&path).unwrap();
    assert!(restored.contains("FileChooser=dfiles;gtk;"));
    assert!(restored.contains("ScreenCast=hyprland"));
    assert_eq!(
        fs::read_to_string(env.service_file()).unwrap(),
        "[D-BUS Service]\nName=old-service\n"
    );
    install(env).unwrap();
    fs::write(&path, format!("[preferred]\n{KEY}=kde;\n")).unwrap();
    assert!(uninstall(env).unwrap());
    assert!(
        fs::read_to_string(path)
            .unwrap()
            .contains("FileChooser=kde;")
    );
}
#[test]
fn system_managed_portals_and_linked_configuration_are_not_overwritten() {
    let mut fixture = Fixture::new();
    fixture.env.managed_dir = Some(fixture.root.join("managed"));
    assert!(install(&fixture.env).is_err());
    assert!(!fixture.env.state_file().exists());
    fixture.env.managed_dir = None;
    let target = fixture.root.join("generated.conf");
    fixture.write(&target, "[preferred]\ndefault=gtk\n");
    let path = fixture.env.target_config();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert!(install(&fixture.env).is_err());
    assert!(!fixture.env.portal_file().exists());
    assert_eq!(
        fs::read_to_string(target).unwrap(),
        "[preferred]\ndefault=gtk\n"
    );
}
#[test]
fn config_editor_respects_sections_comments_and_duplicate_keys() {
    let text = format!(
        "[other]\n{KEY}=not-a-preference\n[preferred]\n# comment\n{KEY}=gtk\n{KEY}=kde\n[another]\nx=y\n"
    );
    let updated = config::set_line(&text, Some(&format!("{KEY}=veyra;gtk;")));
    assert_eq!(config::value(&updated, KEY).unwrap(), "veyra;gtk;");
    assert!(updated.contains("# comment"));
    assert!(updated.contains("not-a-preference"));
    assert!(!updated.contains("FileChooser=kde"));
    assert!(updated.ends_with("[another]\nx=y\n"));
}

#[test]
fn setup_failure_preserves_configuration_and_permissions() {
    let fixture = Fixture::new();
    let env = &fixture.env;
    let path = env.target_config();
    let original = format!("[preferred]\n{KEY}=gtk;\n");
    fixture.write(&path, &original);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(&env.executable).unwrap();
    assert!(install(env).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    assert!(!env.portal_file().exists());
    assert!(!env.service_file().exists());
    fs::write(&env.executable, b"fixture executable").unwrap();
    install(env).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    uninstall(env).unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
