//! Unit tests for the batch-rename preview logic.
use super::state::*;
use std::path::PathBuf;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(|n| PathBuf::from("/tmp").join(n)).collect()
}

#[test]
fn replace_simple_find_and_replace() {
    let p = paths(&["photo_001.jpg", "photo_002.jpg", "photo_003.jpg"]);
    let rows = compute_replace(&p, "photo", "image");
    assert_eq!(rows[0].renamed, "image_001.jpg");
    assert_eq!(rows[1].renamed, "image_002.jpg");
    assert_eq!(rows[2].renamed, "image_003.jpg");
}

#[test]
fn replace_empty_find_returns_originals() {
    let p = paths(&["a.txt", "b.txt"]);
    let rows = compute_replace(&p, "", "anything");
    assert_eq!(rows[0].renamed, "a.txt");
    assert_eq!(rows[1].renamed, "b.txt");
}

#[test]
fn replace_no_match_keeps_original() {
    let p = paths(&["hello.txt"]);
    let rows = compute_replace(&p, "xyz", "abc");
    assert_eq!(rows[0].renamed, "hello.txt");
}

#[test]
fn format_counter_substitution() {
    let p = paths(&["alpha.txt", "beta.txt", "gamma.txt"]);
    let rows = compute_format(&p, "file_[n][ext]", 1);
    assert_eq!(rows[0].renamed, "file_1.txt");
    assert_eq!(rows[1].renamed, "file_2.txt");
    assert_eq!(rows[2].renamed, "file_3.txt");
}

#[test]
fn format_zero_padded_counter() {
    let p = paths(&["a.txt", "b.txt"]);
    let rows = compute_format(&p, "item_[0n][ext]", 5);
    assert_eq!(rows[0].renamed, "item_05.txt");
    assert_eq!(rows[1].renamed, "item_06.txt");

    let rows = compute_format(&p, "item_[00n][ext]", 5);
    assert_eq!(rows[0].renamed, "item_005.txt");
    assert_eq!(rows[1].renamed, "item_006.txt");
}

#[test]
fn format_name_and_ext_tokens() {
    let p = paths(&["report.pdf", "photo.jpg"]);
    let rows = compute_format(&p, "[name]_backup[ext]", 1);
    assert_eq!(rows[0].renamed, "report_backup.pdf");
    assert_eq!(rows[1].renamed, "photo_backup.jpg");
}

#[test]
fn format_empty_template_keeps_originals() {
    let p = paths(&["keep.txt"]);
    let rows = compute_format(&p, "", 1);
    assert_eq!(rows[0].renamed, "keep.txt");
}

#[test]
fn format_no_extension_file() {
    let p = paths(&["Makefile"]);
    let rows = compute_format(&p, "[name]_v2[ext]", 1);
    assert_eq!(rows[0].renamed, "Makefile_v2");
}

#[test]
fn format_dotfile_treated_as_name() {
    let p = paths(&[".gitignore"]);
    let rows = compute_format(&p, "[name][ext]", 1);
    // A leading-dot file has no extension; the entire name is the stem.
    assert_eq!(rows[0].renamed, ".gitignore");
}

#[test]
fn counter_start_offset() {
    let p = paths(&["a.txt", "b.txt"]);
    let rows = compute_format(&p, "file[n]", 10);
    assert_eq!(rows[0].renamed, "file10");
    assert_eq!(rows[1].renamed, "file11");
}
