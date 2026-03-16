use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
struct Config {
    repo_root: PathBuf,
    bin_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
struct Outcome {
    pass: u32,
    fail: u32,
}

fn run_all(config: &Config) -> io::Result<Outcome> {
    let suite_root = config.repo_root.join("tests/stella_test_suite/stage1");
    let well_typed_roots = vec![suite_root.join("well-typed"), suite_root.join("extra")];
    let ill_typed_roots = vec![suite_root.join("ill-typed"), suite_root.join("extra")];

    let mut pass: u32 = 0;
    let mut fail: u32 = 0;

    run_group(
        &config.bin_path,
        &config.repo_root,
        &well_typed_roots,
        "well-typed",
        true,
        &mut pass,
        &mut fail,
    )?;
    run_group(
        &config.bin_path,
        &config.repo_root,
        &ill_typed_roots,
        "ill-typed",
        false,
        &mut pass,
        &mut fail,
    )?;

    println!("\nResults: {}/{} passed", pass, pass + fail);

    Ok(Outcome { pass, fail })
}

fn run_group(
    bin_path: &Path,
    repo_root: &Path,
    roots: &[PathBuf],
    marker: &str,
    expect_ok: bool,
    pass: &mut u32,
    fail: &mut u32,
) -> io::Result<()> {
    let files = collect_tests(roots, marker)?;

    for file in files {
        let matched = run_case(bin_path, repo_root, &file, expect_ok)?;
        if matched {
            *pass += 1;
        } else {
            *fail += 1;
        }
    }

    Ok(())
}

fn collect_tests(roots: &[PathBuf], marker: &str) -> io::Result<Vec<PathBuf>> {
    let mut stack: Vec<PathBuf> = roots.to_vec();
    let mut files: Vec<PathBuf> = Vec::new();
    let marker_os = OsStr::new(marker);

    while let Some(path) = stack.pop() {
        let metadata = match fs::metadata(&path) {
            Ok(meta) => meta,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };

        if metadata.is_dir() {
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                stack.push(entry.path());
            }
        } else if metadata.is_file()
            && path.extension() == Some(OsStr::new("stella"))
            && path.components().any(|c| c.as_os_str() == marker_os)
        {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn run_case(bin_path: &Path, repo_root: &Path, file: &Path, expect_ok: bool) -> io::Result<bool> {
    let output = Command::new(bin_path).arg(file).output().map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("failed to run {}: {err}", bin_path.display()),
        )
    })?;

    let status_ok = output.status.success();
    let matched = if expect_ok { status_ok } else { !status_ok };

    if matched {
        return Ok(true);
    }

    let rel = file
        .strip_prefix(repo_root)
        .unwrap_or(file)
        .display()
        .to_string();

    let expected = if expect_ok {
        "Type OK (exit 0)"
    } else {
        "type error (exit != 0)"
    };
    let actual = output
        .status
        .code()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "signal".to_string());

    eprintln!(
        "[FAIL] {} {}",
        if expect_ok { "well-typed" } else { "ill-typed" },
        rel
    );
    eprintln!("  expected {}  got exit {}", expected, actual);

    let mut combined = String::new();
    combined.push_str(&String::from_utf8_lossy(&output.stdout));
    combined.push_str(&String::from_utf8_lossy(&output.stderr));

    let mut lines = combined.lines();
    if let Some(first) = lines.next() {
        eprintln!("  output:");
        eprintln!("    {}", first);
        for line in lines.take(4) {
            eprintln!("    {}", line);
        }
    }

    Ok(false)
}

fn resolve_bin(repo_root: &Path, target_dir_override: Option<PathBuf>) -> io::Result<PathBuf> {
    if let Ok(bin) = env::var("CARGO_BIN_EXE_stella-typechecker") {
        return Ok(PathBuf::from(bin));
    }

    let target_dir = if let Some(path) = target_dir_override {
        normalize_dir(repo_root, &path)?
    } else if let Ok(dir) = env::var("CARGO_TARGET_DIR") {
        normalize_dir(repo_root, Path::new(&dir))?
    } else {
        repo_root.join("target")
    };

    let candidate = target_dir.join("debug/stella-typechecker");
    if candidate.is_file() {
        Ok(candidate)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("binary not found at {}; build the project first (e.g., `cargo build --all-targets`)", candidate.display()),
        ))
    }
}

fn normalize_dir(base: &Path, dir: &Path) -> io::Result<PathBuf> {
    Ok(if dir.is_absolute() {
        dir.to_path_buf()
    } else {
        base.join(dir)
    })
}

#[test]
fn stage1_suite_run() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir_override = env::var("CARGO_TARGET_DIR").ok().map(PathBuf::from);
    let bin_path = resolve_bin(&repo_root, target_dir_override.clone())
        .expect("stella-typechecker binary should be built by cargo");

    let config = Config {
        repo_root,
        bin_path,
    };

    let outcome = run_all(&config).expect("suite should run");
    assert_eq!(outcome.fail, 0, "Stage 1 suite reported failures");
}
