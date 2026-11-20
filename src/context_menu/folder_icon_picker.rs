//! Choose a folder icon with Veyra's existing file picker.
use std::{path::{Path, PathBuf}, process::{Command, Stdio}};

use gpui_kit::{Context, Window};
use crate::{picker::{Reply, Request}, windows::MainWindow};

pub fn open(folder: PathBuf, window: &mut Window, cx: &mut Context<MainWindow>) {
    let executor = cx.background_executor().clone();
    cx.spawn_in(window, async move |state, cx| {
        let picker_folder = folder.clone();
        let result = executor.spawn(async move { choose(&picker_folder) }).await;
        let _ = state.update_in(cx, |state, window, cx| match result {
            Ok(Some(icon)) => match crate::services::folder_icons::set(&folder, &icon) {
                Ok(()) => cx.notify(),
                Err(error) => super::actions::message(state, "Could not set folder icon", error.to_string(), window, cx),
            },
            Ok(None) => {},
            Err(error) => super::actions::message(state, "Could not choose folder icon", error, window, cx),
        });
    }).detach();
}

fn choose(folder: &Path) -> Result<Option<PathBuf>, String> {
    let request = Request {
        title: "Choose a folder icon".into(),
        accept_label: "Use icon".into(),
        current_folder: Some(folder.to_path_buf()),
        filters: vec![("Images".into(), vec![(1, "image/*".into())])],
        ..Default::default()
    };
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut child = Command::new(executable).arg("--picker-json")
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .spawn().map_err(|error| error.to_string())?;
    serde_json::to_writer(child.stdin.take().unwrap(), &request).map_err(|error| error.to_string())?;
    let output = child.wait_with_output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    if output.stdout.is_empty() { return Ok(None); }
    let reply: Reply = serde_json::from_slice(&output.stdout).map_err(|error| error.to_string())?;
    let uri = reply.uris.first().ok_or("The picker returned no image.")?;
    let path = url::Url::parse(uri).map_err(|error| error.to_string())?
        .to_file_path().map_err(|_| "The selected image is not a local file.")?;
    Ok(Some(path))
}
