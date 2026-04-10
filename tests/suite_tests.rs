use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
struct Config {
    repo_root: PathBuf,
    bin_path: PathBuf,
}

fn run_all(config: &Config, stage_root: &Path) -> io::Result<u32> {
    let well_typed_roots = vec![stage_root.join("well-typed"), stage_root.join("extra")];
    let ill_typed_roots = vec![stage_root.join("ill-typed"), stage_root.join("extra")];

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

    Ok(fail)
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
fn suite_run() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir_override = env::var("CARGO_TARGET_DIR").ok().map(PathBuf::from);
    let bin_path = resolve_bin(&repo_root, target_dir_override.clone())
        .expect("stella-typechecker binary should be built by cargo");

    let config = Config {
        repo_root: repo_root.clone(),
        bin_path,
    };
    let suite_root = repo_root.join("tests/stella_test_suite");

    let mut total_fail: u32 = 0;
    let mut entries: Vec<_> = fs::read_dir(&suite_root)
        .expect("test suite directory should exist")
        .map(|e| e.expect("directory entry").path())
        .filter(|p| p.is_dir())
        .collect();
    entries.sort();

    for stage_root in entries {
        let stage_name = stage_root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        eprintln!("=== Suite run: {} ===", stage_name);
        let fails = run_all(&config, &stage_root).expect("suite should run");
        total_fail += fails;
    }

    assert_eq!(total_fail, 0, "Suite reported failures");
}

// ---------------------------------------------------------------------------
// Etalon comparison (docker run -i fizruk/stella typecheck)
// ---------------------------------------------------------------------------

struct EtalonOutput {
    ok: bool,
    stdout: String,
    stderr: String,
}

/// Run the etalon typechecker on a file via Docker.
///
/// Returns `Ok(Some(_))` with the full output, or `Ok(None)` when Docker is
/// not available so the caller can skip the comparison.
fn run_etalon(file: &Path) -> io::Result<Option<EtalonOutput>> {
    let source = fs::read(file)?;

    let mut child = match Command::new("docker")
        .args(["run", "--rm", "-i", "fizruk/stella", "typecheck"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
        Ok(child) => child,
    };

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(&source)?;
    }

    let output = child.wait_with_output()?;
    Ok(Some(EtalonOutput {
        ok: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }))
}

/// Extract the error tag from our typechecker's output.
/// Our format: `ERROR_SOMETHING:\n  ...` — the tag is the first word before `:`.
fn extract_our_tag(output: &str) -> Option<&str> {
    let first = output.lines().next()?;
    let tag = first.split(':').next()?.trim();
    if tag.starts_with("ERROR_") {
        Some(tag)
    } else {
        None
    }
}

/// Extract the error tag from the etalon's output.
/// Handles two formats:
///  - `Type Error Tag: [ERROR_SOMETHING]`
///  - `Unsupported Syntax Error: ERROR_SOMETHING`
fn extract_etalon_tag(output: &str) -> Option<&str> {
    for line in output.lines() {
        if let Some(rest) = line.strip_prefix("Type Error Tag: [") {
            return Some(rest.trim_end_matches(']'));
        }
        if let Some(rest) = line.strip_prefix("Unsupported Syntax Error: ") {
            let tag = rest.trim();
            if tag.starts_with("ERROR_") {
                return Some(tag);
            }
        }
    }
    None
}

/// Collect every `.stella` file reachable from `roots` without any marker
/// filtering (used for the etalon comparison where we don't use expected
/// well-/ill-typed labels).
fn collect_stella_files(roots: &[PathBuf]) -> io::Result<Vec<PathBuf>> {
    let mut stack: Vec<PathBuf> = roots.to_vec();
    let mut files: Vec<PathBuf> = Vec::new();

    while let Some(path) = stack.pop() {
        let metadata = match fs::metadata(&path) {
            Ok(meta) => meta,
            Err(err) if err.kind() == io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err),
        };

        if metadata.is_dir() {
            for entry in fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
        } else if metadata.is_file() && path.extension() == Some(OsStr::new("stella")) {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn run_compare_case(
    bin_path: &Path,
    repo_root: &Path,
    file: &Path,
    pass: &mut u32,
    fail: &mut u32,
    docker_missing: &mut bool,
) -> io::Result<()> {
    if *docker_missing {
        return Ok(());
    }

    let our_output = Command::new(bin_path).arg(file).output().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("failed to run {}: {e}", bin_path.display()),
        )
    })?;
    let our_ok = our_output.status.success();
    let our_stdout = String::from_utf8_lossy(&our_output.stdout).into_owned();
    let our_stderr = String::from_utf8_lossy(&our_output.stderr).into_owned();

    let etalon = match run_etalon(file)? {
        None => {
            eprintln!("[SKIP] docker not available — etalon comparison disabled");
            *docker_missing = true;
            return Ok(());
        }
        Some(v) => v,
    };

    let rel = file
        .strip_prefix(repo_root)
        .unwrap_or(file)
        .display()
        .to_string();

    // Phase 1: compare pass/fail
    if our_ok != etalon.ok {
        *fail += 1;
        eprintln!("[DISAGREE outcome] {}", rel);
        eprintln!(
            "  ours: {}  etalon: {}",
            if our_ok { "OK" } else { "ERROR" },
            if etalon.ok { "OK" } else { "ERROR" },
        );
        print_output("ours", &our_stdout, &our_stderr);
        print_output("etalon", &etalon.stdout, &etalon.stderr);
        return Ok(());
    }

    // Phase 2: when both report an error, compare the error tag
    if !our_ok {
        let our_tag = extract_our_tag(&our_stderr);
        let etalon_tag =
            extract_etalon_tag(&etalon.stdout).or_else(|| extract_etalon_tag(&etalon.stderr));
        if our_tag != etalon_tag {
            *fail += 1;
            eprintln!("[DISAGREE tag] {}", rel);
            eprintln!(
                "  ours: {}  etalon: {}",
                our_tag.unwrap_or("<no tag>"),
                etalon_tag.unwrap_or("<no tag>"),
            );
            print_output("ours", &our_stdout, &our_stderr);
            print_output("etalon", &etalon.stdout, &etalon.stderr);
            return Ok(());
        }
    }

    *pass += 1;
    Ok(())
}

fn print_output(label: &str, stdout: &str, stderr: &str) {
    let combined = format!("{}{}", stdout, stderr);
    let mut lines = combined.lines().peekable();
    if lines.peek().is_some() {
        eprintln!("  {} output:", label);
        for line in lines.take(6) {
            eprintln!("    {}", line);
        }
    }
}

fn run_all_compare(config: &Config, stage_root: &Path) -> io::Result<u32> {
    let roots = vec![
        stage_root.join("well-typed"),
        stage_root.join("ill-typed"),
        stage_root.join("extra"),
        stage_root.join("failed"),
    ];

    let files = collect_stella_files(&roots)?;
    let mut pass: u32 = 0;
    let mut fail: u32 = 0;
    let mut docker_missing = false;

    for file in &files {
        run_compare_case(
            &config.bin_path,
            &config.repo_root,
            file,
            &mut pass,
            &mut fail,
            &mut docker_missing,
        )?;
    }

    if docker_missing {
        eprintln!("Docker was not found; etalon comparison was skipped entirely.");
    } else {
        println!("\nEtalon comparison: {}/{} agree", pass, pass + fail);
    }

    Ok(fail)
}

/// Compare our typechecker against the reference Docker image on all stages.
/// Run with:
///
/// ```
/// STELLA_COMPARE_ETALON=1 cargo test suite_compare_etalon -- --nocapture
/// ```
#[test]
fn suite_compare_etalon() {
    if env::var("STELLA_COMPARE_ETALON").unwrap_or_default() != "1" {
        eprintln!("Skipping etalon comparison (set STELLA_COMPARE_ETALON=1 to enable)");
        return;
    }

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target_dir_override = env::var("CARGO_TARGET_DIR").ok().map(PathBuf::from);
    let bin_path = resolve_bin(&repo_root, target_dir_override)
        .expect("stella-typechecker binary should be built by cargo");

    let config = Config {
        repo_root: repo_root.clone(),
        bin_path,
    };
    let suite_root = repo_root.join("tests/stella_test_suite");

    let mut total_fail: u32 = 0;
    for entry in fs::read_dir(&suite_root).expect("test suite directory should exist") {
        let entry = entry.expect("directory entry");
        let stage_root = entry.path();
        if !stage_root.is_dir() {
            continue;
        }
        let stage_name = stage_root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        eprintln!("=== Etalon comparison: {} ===", stage_name);
        let fails = run_all_compare(&config, &stage_root).expect("etalon comparison should run");
        total_fail += fails;
    }

    assert_eq!(
        total_fail, 0,
        "Etalon comparison found disagreements between our typechecker and fizruk/stella"
    );
}
