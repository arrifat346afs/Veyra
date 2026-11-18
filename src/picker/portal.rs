//! org.freedesktop.impl.portal.FileChooser backend. Each request gets an isolated
//! picker process, so concurrent callers and cancellation cannot share selection.
use super::{Choice, Filter, Reply, Request};
use futures_lite::io::AsyncWriteExt;
use std::{collections::HashMap, os::unix::ffi::OsStringExt, path::PathBuf, process::Stdio};
use zbus::{
    Connection,
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};

type Options = HashMap<String, OwnedValue>;
type Response = (u32, Options);
struct FileChooser;
struct PendingRequest(async_channel::Sender<()>);

#[zbus::interface(name = "org.freedesktop.impl.portal.Request")]
impl PendingRequest {
    async fn close(&self) {
        let _ = self.0.try_send(());
    }
}

#[zbus::interface(name = "org.freedesktop.impl.portal.FileChooser")]
impl FileChooser {
    #[zbus(out_args("response", "results"))]
    async fn open_file(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: Options,
        #[zbus(connection)] connection: &Connection,
    ) -> (u32, Options) {
        let _ = (app_id, parent_window);
        choose(
            connection,
            handle,
            parse_request(title, &options, false, false),
        )
        .await
    }
    #[zbus(out_args("response", "results"))]
    async fn save_file(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: Options,
        #[zbus(connection)] connection: &Connection,
    ) -> (u32, Options) {
        let _ = (app_id, parent_window);
        choose(
            connection,
            handle,
            parse_request(title, &options, true, false),
        )
        .await
    }
    #[zbus(out_args("response", "results"))]
    async fn save_files(
        &self,
        handle: OwnedObjectPath,
        app_id: String,
        parent_window: String,
        title: String,
        options: Options,
        #[zbus(connection)] connection: &Connection,
    ) -> (u32, Options) {
        let _ = (app_id, parent_window);
        choose(
            connection,
            handle,
            parse_request(title, &options, false, true),
        )
        .await
    }
}

fn flag(options: &Options, key: &str) -> bool {
    options
        .get(key)
        .and_then(|v| bool::try_from(v).ok())
        .unwrap_or(false)
}
fn string(options: &Options, key: &str) -> String {
    options
        .get(key)
        .and_then(|v| <&str>::try_from(v).ok())
        .unwrap_or_default()
        .to_owned()
}
fn path(options: &Options, key: &str) -> Option<PathBuf> {
    let mut bytes = Vec::<u8>::try_from(options.get(key)?.try_clone().ok()?).ok()?;
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    if bytes.contains(&0) || bytes.is_empty() {
        return None;
    }
    let path = PathBuf::from(std::ffi::OsString::from_vec(bytes));
    path.is_absolute().then_some(path)
}
fn parse_request(title: String, options: &Options, save: bool, save_files: bool) -> Request {
    let mut filters = options
        .get("filters")
        .and_then(|v| Vec::<Filter>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    let current_filter = options
        .get("current_filter")
        .and_then(|v| Filter::try_from(v.try_clone().ok()?).ok());
    let filter_index = current_filter
        .map(|filter| {
            filters
                .iter()
                .position(|f| *f == filter)
                .unwrap_or_else(|| {
                    filters.push(filter);
                    filters.len() - 1
                })
        })
        .unwrap_or(0);
    let mut choices = options
        .get("choices")
        .and_then(|v| Vec::<Choice>::try_from(v.try_clone().ok()?).ok())
        .unwrap_or_default();
    for (_, _, values, selected) in &mut choices {
        if values.is_empty() {
            if selected != "true" {
                *selected = "false".into();
            }
        } else if !values.iter().any(|(id, _)| id == selected) {
            *selected = values[0].0.clone();
        }
    }
    let mut request = Request {
        title,
        save,
        directory: save_files || flag(options, "directory"),
        multiple: !save && !save_files && flag(options, "multiple"),
        accept_label: string(options, "accept_label").replace('_', ""),
        current_folder: path(options, "current_folder"),
        current_name: string(options, "current_name"),
        filters,
        filter_index,
        choices,
        files: Vec::new(),
    };
    if save {
        if let Some(file) = path(options, "current_file") {
            request.current_folder = file.parent().map(PathBuf::from);
            request.current_name = file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
        }
    }
    if save_files {
        let names = options
            .get("files")
            .and_then(|v| Vec::<Vec<u8>>::try_from(v.try_clone().ok()?).ok())
            .unwrap_or_default();
        request.files = names
            .into_iter()
            .map(|mut bytes| {
                if bytes.last() == Some(&0) {
                    bytes.pop();
                }
                String::from_utf8(bytes).unwrap_or_default()
            })
            .collect();
    }
    request
}

async fn choose(connection: &Connection, handle: OwnedObjectPath, request: Request) -> Response {
    let (cancel, cancelled) = async_channel::bounded(1);
    if let Err(error) = connection
        .object_server()
        .at(handle.clone(), PendingRequest(cancel))
        .await
    {
        eprintln!("Cannot register picker request: {error}");
        return (2, Options::new());
    }
    let result = futures_lite::future::race(async { run_picker(request).await }, async {
        let _ = cancelled.recv().await;
        Ok(None)
    })
    .await;
    let _ = connection
        .object_server()
        .remove::<PendingRequest, _>(handle)
        .await;
    match result {
        Ok(Some(reply)) => {
            let mut results = Options::new();
            results.insert("uris".into(), Value::from(reply.uris).try_into().unwrap());
            if let Some(filter) = reply.current_filter {
                results.insert(
                    "current_filter".into(),
                    Value::from(filter).try_into().unwrap(),
                );
            }
            results.insert(
                "choices".into(),
                Value::from(reply.choices).try_into().unwrap(),
            );
            (0, results)
        }
        Ok(None) => (1, Options::new()),
        Err(error) => {
            eprintln!("Veyra picker failed: {error}");
            (2, Options::new())
        }
    }
}

async fn run_picker(
    request: Request,
) -> Result<Option<Reply>, Box<dyn std::error::Error + Send + Sync>> {
    let mut child = async_process::Command::new(std::env::current_exe()?)
        .arg("--picker-json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true)
        .reap_on_drop(true)
        .spawn()?;
    let bytes = serde_json::to_vec(&request)?;
    let mut stdin = child.stdin.take().ok_or("Missing picker input pipe")?;
    stdin.write_all(&bytes).await?;
    drop(stdin);
    let output = child.output().await?;
    if !output.status.success() {
        return Err(format!("Picker exited with {}", output.status).into());
    }
    if output.stdout.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_slice(&output.stdout)?))
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    futures_lite::future::block_on(async {
        let connection = zbus::connection::Builder::session()?
            .name("org.freedesktop.impl.portal.desktop.veyra")?
            .serve_at("/org/freedesktop/portal/desktop", FileChooser)?
            .build()
            .await?;
        connection.closed().await;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portal_options_decode_byte_paths_filters_and_choices() {
        let mut options = Options::new();
        options.insert("multiple".into(), true.into());
        options.insert(
            "current_folder".into(),
            Value::from(b"/tmp\0".to_vec()).try_into().unwrap(),
        );
        let filter: Filter = ("Text".into(), vec![(0, "*.txt".into())]);
        options.insert(
            "current_filter".into(),
            Value::from(filter.clone()).try_into().unwrap(),
        );
        options.insert(
            "choices".into(),
            Value::from(vec![(
                "encoding".to_owned(),
                "Encoding".to_owned(),
                vec![("utf8".to_owned(), "UTF-8".to_owned())],
                "".to_owned(),
            )])
            .try_into()
            .unwrap(),
        );
        let request = parse_request("Upload".into(), &options, false, false);
        assert!(request.multiple);
        assert_eq!(request.current_folder, Some(PathBuf::from("/tmp")));
        assert_eq!(request.filters[request.filter_index], filter);
        assert_eq!(request.choices[0].3, "utf8");
        options.insert(
            "current_folder".into(),
            Value::from(b"relative\0".to_vec()).try_into().unwrap(),
        );
        assert_eq!(path(&options, "current_folder"), None);
    }
}
