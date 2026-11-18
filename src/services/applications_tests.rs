use super::*;
use std::{
    fs,
    process::Command,
    time::{Duration, Instant},
};

#[test]
fn installed_apps_filter_by_type_and_launch_without_changing_defaults() {
    // GIO caches the XDG application registry. Isolate the fixture in a child
    // process rather than changing process-wide environment during other tests.
    if let Some(root) = std::env::var_os("VEYRA_APP_TEST_CHILD") {
        let root = PathBuf::from(root);
        let paths = vec![root.join("notes # $HOME.txt"), root.join("image.png")];
        let apps = available_for(&paths).unwrap();
        let recorder = apps
            .iter()
            .find(|app| app.id == "veyra-recorder.desktop")
            .unwrap();
        assert!(recorder.recommended);
        assert!(recorder.matches("RECORDER"));
        assert!(
            !apps
                .iter()
                .find(|app| app.id == "veyra-text-only.desktop")
                .unwrap()
                .recommended
        );
        assert!(!apps.iter().any(|app|app.id=="veyra-hidden.desktop" || app.id=="veyra-no-display.desktop"));
        let defaults = root.join("config/mimeapps.list");
        let original = fs::read(&defaults).unwrap();
        launch(&recorder.id, &paths).unwrap();
        let output = root.join("arguments.json");
        let deadline = Instant::now() + Duration::from_secs(5);
        while !output.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let arguments: Vec<PathBuf> = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
        assert_eq!(arguments, paths);
        assert_eq!(fs::read(defaults).unwrap(), original);
        assert!(launch("veyra-not-installed.desktop", &paths).is_err());
        return;
    }
    let root = std::env::temp_dir().join(format!("veyra-applications-test-{}", std::process::id()));
    fs::create_dir_all(root.join("data/applications")).unwrap();
    fs::create_dir_all(root.join("config")).unwrap();
    fs::write(root.join("notes # $HOME.txt"), "text fixture\n").unwrap();
    image::RgbaImage::new(1, 1)
        .save(root.join("image.png"))
        .unwrap();
    fs::write(
        root.join("record.py"),
        "import json,sys,pathlib\npathlib.Path(sys.argv[1]).write_text(json.dumps(sys.argv[2:]))\n",
    )
    .unwrap();
    let exec = format!(
        "/usr/bin/python3 \"{}\" \"{}\" %F",
        root.join("record.py").display(),
        root.join("arguments.json").display()
    );
    for (id, name, mime, extra) in [
        (
            "veyra-recorder",
            "Veyra Recorder",
            "text/plain;image/png;",
            "",
        ),
        ("veyra-text-only", "Veyra Text", "text/plain;", ""),
        (
            "veyra-hidden",
            "Hidden",
            "text/plain;image/png;",
            "Hidden=true\n",
        ),
        (
            "veyra-no-display",
            "No Display",
            "text/plain;image/png;",
            "NoDisplay=true\n",
        ),
    ] {
        fs::write(root.join(format!("data/applications/{id}.desktop")),format!("[Desktop Entry]\nType=Application\nName={name}\nExec={exec}\nMimeType={mime}\n{extra}")).unwrap();
    }
    fs::write(root.join("data/applications/mimeinfo.cache"),"[MIME Cache]\ntext/plain=veyra-recorder.desktop;veyra-text-only.desktop;\nimage/png=veyra-recorder.desktop;\n").unwrap();
    fs::write(root.join("config/mimeapps.list"),"[Default Applications]\ntext/plain=veyra-text-only.desktop;\n[Added Associations]\ntext/plain=veyra-recorder.desktop;veyra-text-only.desktop;\nimage/png=veyra-recorder.desktop;\n").unwrap();
    let result=Command::new(std::env::current_exe().unwrap())
        .args(["--exact","services::applications::tests::installed_apps_filter_by_type_and_launch_without_changing_defaults","--nocapture"])
        .env("VEYRA_APP_TEST_CHILD",&root).env("XDG_DATA_HOME",root.join("data")).env("XDG_CONFIG_HOME",root.join("config"))
        .env("XDG_DATA_DIRS","/usr/share").env("XDG_CONFIG_DIRS",root.join("system-config"))
        .output().unwrap();
    let _ = fs::remove_dir_all(root);
    assert!(
        result.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}
