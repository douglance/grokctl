//! Rust-only project maintenance commands.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const PROD_FILE_LIMIT: usize = 300;
const TEST_FILE_LIMIT: usize = 400;
const MODULE_FILE_LIMIT: usize = 100;
const FUNCTION_LIMIT: usize = 40;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("quality") => quality(),
        Some("manifest") => refresh_manifest(&args.collect::<Vec<_>>()),
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: cargo xtask quality");
    eprintln!("       cargo xtask manifest <host-source> <host-version> <output-json>");
    ExitCode::FAILURE
}

fn refresh_manifest(args: &[String]) -> ExitCode {
    let [source_path, version, output_path] = args else {
        return usage();
    };
    let source = match fs::read_to_string(source_path) {
        Ok(value) => value,
        Err(error) => return task_error("read host source", error),
    };
    let manifest = match grokctl_manifest::extract_host_manifest(&source, version) {
        Ok(value) => value,
        Err(error) => return task_error("extract host manifest", error),
    };
    let json = match serde_json::to_string_pretty(&manifest) {
        Ok(value) => value,
        Err(error) => return task_error("serialize host manifest", error),
    };
    match fs::write(output_path, format!("{json}\n")) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => task_error("write host manifest", error),
    }
}

fn task_error(action: &str, error: impl std::fmt::Display) -> ExitCode {
    eprintln!("cannot {action}: {error}");
    ExitCode::FAILURE
}

fn quality() -> ExitCode {
    let files = rust_files(Path::new("."));
    let mut violations = files.iter().flat_map(|path| inspect(path)).collect::<Vec<_>>();
    violations.sort();
    for violation in &violations {
        eprintln!("{violation}");
    }
    if violations.is_empty() {
        println!("quality: passed ({} Rust files)", files.len());
        ExitCode::SUCCESS
    } else {
        eprintln!("quality: failed ({} violations)", violations.len());
        ExitCode::FAILURE
    }
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit(root, &mut files);
    files.sort();
    files
}

fn visit(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && !ignored_directory(&path) {
            visit(&path, files);
        } else if path.extension() == Some(OsStr::new("rs")) {
            files.push(path);
        }
    }
}

fn ignored_directory(path: &Path) -> bool {
    matches!(path.file_name().and_then(OsStr::to_str), Some("target" | ".git"))
}

fn inspect(path: &Path) -> Vec<String> {
    let Ok(source) = fs::read_to_string(path) else {
        return vec![format!("{}: cannot read", path.display())];
    };
    let lines = source.lines().collect::<Vec<_>>();
    let mut violations = file_size_violations(path, lines.len());
    violations.extend(attribute_violations(path, &lines));
    violations.extend(function_violations(path, &lines));
    violations
}

fn file_size_violations(path: &Path, lines: usize) -> Vec<String> {
    let is_module = path.file_name() == Some(OsStr::new("mod.rs"));
    let is_test = path.components().any(|part| part.as_os_str() == OsStr::new("tests"));
    let limit = if is_module {
        MODULE_FILE_LIMIT
    } else if is_test {
        TEST_FILE_LIMIT
    } else {
        PROD_FILE_LIMIT
    };
    (lines > limit)
        .then(|| format!("{}: {lines} lines exceeds {limit}", path.display()))
        .into_iter()
        .collect()
}

fn attribute_violations(path: &Path, lines: &[&str]) -> Vec<String> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            let line = line.trim_start();
            let invalid = line.starts_with("#[allow(")
                || (line.starts_with("#[expect(") && !line.contains("reason ="));
            invalid
                .then(|| format!("{}:{}: unreasoned lint suppression", path.display(), index + 1))
        })
        .collect()
}

fn function_violations(path: &Path, lines: &[&str]) -> Vec<String> {
    let mut violations = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if is_function_start(line) {
            let length = function_length(&lines[index..]);
            violations.extend(function_violation(path, index + 1, length));
            index += length.max(1);
        } else {
            index += 1;
        }
    }
    violations
}

fn function_violation(path: &Path, line: usize, length: usize) -> Option<String> {
    (length > FUNCTION_LIMIT).then(|| {
        format!("{}:{line}: function is {length} lines; limit is {FUNCTION_LIMIT}", path.display())
    })
}

fn is_function_start(line: &str) -> bool {
    !line.starts_with("//")
        && (line.starts_with("fn ")
            || line.starts_with("pub fn ")
            || line.starts_with("pub(crate) fn ")
            || line.starts_with("async fn ")
            || line.starts_with("pub async fn ")
            || line.starts_with("pub(crate) async fn ")
            || line.starts_with("const fn ")
            || line.starts_with("pub const fn "))
}

fn function_length(lines: &[&str]) -> usize {
    let mut depth = 0_i32;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate() {
        depth += brace_delta(line);
        opened |= line.contains('{');
        if opened && depth <= 0 {
            return index + 1;
        }
    }
    lines.len()
}

fn brace_delta(line: &str) -> i32 {
    let opens = line.bytes().filter(|byte| *byte == b'{').count();
    let closes = line.bytes().filter(|byte| *byte == b'}').count();
    let opens = i32::try_from(opens).map_or(i32::MAX, |value| value);
    let closes = i32::try_from(closes).map_or(i32::MAX, |value| value);
    opens - closes
}
