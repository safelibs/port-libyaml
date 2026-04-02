use std::env;
use std::ffi::CStr;
use std::fs;
use std::mem::size_of;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::ptr;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use yaml::types::yaml_char_t;
use yaml::{
    yaml_free, yaml_get_version, yaml_get_version_string, yaml_malloc, yaml_queue_extend,
    yaml_realloc, yaml_stack_extend, yaml_strdup, yaml_string_extend, yaml_string_join,
};

macro_rules! cstr {
    ($value:literal) => {
        unsafe { ::std::ffi::CStr::from_bytes_with_nul_unchecked(concat!($value, "\0").as_bytes()) }
    };
}

#[test]
fn allocator_exports_match_c_compat_rules() {
    unsafe {
        let first = yaml_malloc(0);
        assert!(!first.is_null());
        yaml_free(first);

        let second = yaml_realloc(ptr::null_mut(), 0);
        assert!(!second.is_null());

        let third = yaml_realloc(second, 0);
        assert!(!third.is_null());
        yaml_free(third);

        assert!(yaml_strdup(ptr::null()).is_null());

        let duplicated = yaml_strdup(cstr!("helper-symbols").as_ptr().cast());
        assert!(!duplicated.is_null());
        assert_eq!(CStr::from_ptr(duplicated.cast()), cstr!("helper-symbols"));
        yaml_free(duplicated.cast());

        let version = CStr::from_ptr(yaml_get_version_string());
        assert_eq!(version, cstr!("0.2.5"));

        let mut major = -1;
        let mut minor = -1;
        let mut patch = -1;
        yaml_get_version(&mut major, &mut minor, &mut patch);
        assert_eq!((major, minor, patch), (0, 2, 5));
    }
}

#[test]
fn helper_exports_preserve_offsets_and_contents() {
    unsafe {
        let mut string_start = yaml_malloc(4).cast::<yaml_char_t>();
        assert!(!string_start.is_null());
        ptr::write_bytes(string_start, 0, 4);
        *string_start.add(0) = b'a';
        *string_start.add(1) = b'b';
        let mut string_pointer = string_start.add(2);
        let mut string_end = string_start.add(4);

        assert_eq!(
            yaml_string_extend(&mut string_start, &mut string_pointer, &mut string_end),
            1
        );
        assert_eq!(string_pointer as usize - string_start as usize, 2);
        assert_eq!(string_end as usize - string_start as usize, 8);
        let extended = slice::from_raw_parts(string_start, 8);
        assert_eq!(&extended[..2], b"ab");
        assert!(extended[4..].iter().all(|value| *value == 0));

        let mut join_start = string_start;
        let mut join_pointer = string_pointer;
        let mut join_end = string_end;

        let mut rhs_start = yaml_malloc(3).cast::<yaml_char_t>();
        assert!(!rhs_start.is_null());
        *rhs_start.add(0) = b'X';
        *rhs_start.add(1) = b'Y';
        *rhs_start.add(2) = b'Z';
        let mut rhs_pointer = rhs_start.add(3);
        let mut rhs_end = rhs_start.add(3);

        assert_eq!(
            yaml_string_join(
                &mut join_start,
                &mut join_pointer,
                &mut join_end,
                &mut rhs_start,
                &mut rhs_pointer,
                &mut rhs_end,
            ),
            1
        );
        assert_eq!(join_pointer as usize - join_start as usize, 5);
        assert_eq!(rhs_pointer as usize - rhs_start as usize, 3);
        assert_eq!(slice::from_raw_parts(join_start, 5), b"abXYZ");
        yaml_free(join_start.cast());
        yaml_free(rhs_start.cast());

        let mut null_end_lhs_start = yaml_malloc(4).cast::<yaml_char_t>();
        assert!(!null_end_lhs_start.is_null());
        ptr::write_bytes(null_end_lhs_start, 0, 4);
        *null_end_lhs_start.add(0) = b'L';
        let mut null_end_lhs_pointer = null_end_lhs_start.add(1);
        let mut null_end_lhs_end = null_end_lhs_start.add(4);

        let mut null_end_rhs_start = yaml_malloc(2).cast::<yaml_char_t>();
        assert!(!null_end_rhs_start.is_null());
        *null_end_rhs_start.add(0) = b'R';
        *null_end_rhs_start.add(1) = b'S';
        let mut null_end_rhs_pointer = null_end_rhs_start.add(2);

        assert_eq!(
            yaml_string_join(
                &mut null_end_lhs_start,
                &mut null_end_lhs_pointer,
                &mut null_end_lhs_end,
                &mut null_end_rhs_start,
                &mut null_end_rhs_pointer,
                ptr::null_mut(),
            ),
            1
        );
        assert_eq!(slice::from_raw_parts(null_end_lhs_start, 3), b"LRS");
        yaml_free(null_end_lhs_start.cast());
        yaml_free(null_end_rhs_start.cast());

        let mut stack_start = yaml_malloc(2 * size_of::<u32>());
        assert!(!stack_start.is_null());
        (stack_start.cast::<u32>()).add(0).write(11);
        let mut stack_top = stack_start.cast::<u8>().add(size_of::<u32>()).cast();
        let mut stack_end = stack_start.cast::<u8>().add(2 * size_of::<u32>()).cast();
        assert_eq!(
            yaml_stack_extend(&mut stack_start, &mut stack_top, &mut stack_end),
            1
        );
        assert_eq!(stack_top as usize - stack_start as usize, size_of::<u32>());
        assert_eq!(
            stack_end as usize - stack_start as usize,
            4 * size_of::<u32>()
        );
        assert_eq!(*(stack_start.cast::<u32>()), 11);
        yaml_free(stack_start);

        let mut queue_start = yaml_malloc(2 * size_of::<u32>());
        assert!(!queue_start.is_null());
        queue_start.cast::<u32>().add(0).write(7);
        queue_start.cast::<u32>().add(1).write(8);
        let mut queue_head = queue_start;
        let mut queue_tail = queue_start.cast::<u8>().add(2 * size_of::<u32>()).cast();
        let mut queue_end = queue_tail;
        assert_eq!(
            yaml_queue_extend(
                &mut queue_start,
                &mut queue_head,
                &mut queue_tail,
                &mut queue_end
            ),
            1
        );
        assert_eq!(queue_head as usize - queue_start as usize, 0);
        assert_eq!(
            queue_tail as usize - queue_start as usize,
            2 * size_of::<u32>()
        );
        assert_eq!(
            queue_end as usize - queue_start as usize,
            4 * size_of::<u32>()
        );
        assert_eq!(slice::from_raw_parts(queue_start.cast::<u32>(), 2), &[7, 8]);
        yaml_free(queue_start);

        let mut move_start = yaml_malloc(4 * size_of::<u32>());
        assert!(!move_start.is_null());
        let move_words = move_start.cast::<u32>();
        move_words.add(0).write(1);
        move_words.add(1).write(2);
        move_words.add(2).write(3);
        move_words.add(3).write(4);
        let mut move_head = move_start.cast::<u8>().add(2 * size_of::<u32>()).cast();
        let mut move_tail = move_start.cast::<u8>().add(4 * size_of::<u32>()).cast();
        let mut move_end = move_tail;
        assert_eq!(
            yaml_queue_extend(
                &mut move_start,
                &mut move_head,
                &mut move_tail,
                &mut move_end
            ),
            1
        );
        assert_eq!(move_head as usize - move_start as usize, 0);
        assert_eq!(
            move_tail as usize - move_start as usize,
            2 * size_of::<u32>()
        );
        assert_eq!(
            move_end as usize - move_start as usize,
            4 * size_of::<u32>()
        );
        assert_eq!(slice::from_raw_parts(move_start.cast::<u32>(), 2), &[3, 4]);
        yaml_free(move_start);
    }
}

#[test]
fn helper_exports_accept_zero_capacity_states() {
    unsafe {
        let mut string_start = ptr::null_mut::<yaml_char_t>();
        let mut string_pointer = ptr::null_mut::<yaml_char_t>();
        let mut string_end = ptr::null_mut::<yaml_char_t>();
        assert_eq!(
            yaml_string_extend(&mut string_start, &mut string_pointer, &mut string_end),
            1
        );
        assert!(!string_start.is_null());
        assert_eq!(string_pointer, string_start);
        assert_eq!(string_end, string_start);
        yaml_free(string_start.cast());

        let mut stack_start = ptr::null_mut();
        let mut stack_top = ptr::null_mut();
        let mut stack_end = ptr::null_mut();
        assert_eq!(
            yaml_stack_extend(&mut stack_start, &mut stack_top, &mut stack_end),
            1
        );
        assert!(!stack_start.is_null());
        assert_eq!(stack_top, stack_start);
        assert_eq!(stack_end, stack_start);
        yaml_free(stack_start);

        let mut queue_start = ptr::null_mut();
        let mut queue_head = ptr::null_mut();
        let mut queue_tail = ptr::null_mut();
        let mut queue_end = ptr::null_mut();
        assert_eq!(
            yaml_queue_extend(
                &mut queue_start,
                &mut queue_head,
                &mut queue_tail,
                &mut queue_end
            ),
            1
        );
        assert!(!queue_start.is_null());
        assert_eq!(queue_head, queue_start);
        assert_eq!(queue_tail, queue_start);
        assert_eq!(queue_end, queue_start);
        yaml_free(queue_start);
    }
}

#[test]
fn staged_install_exports_full_manifest_and_runs_vendored_version_test() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = temp_dir("stage-root");
    let arch = multiarch();
    let stage_lib_dir = stage_root.join("usr/lib").join(&arch);

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install",
    );

    assert!(stage_root.join("usr/include/yaml.h").is_file());
    assert!(stage_lib_dir.join("libyaml-0.so.2").is_file());
    assert!(stage_lib_dir.join("libyaml.a").is_file());
    assert!(stage_lib_dir.join("libyaml.so").exists());
    assert!(stage_lib_dir.join("pkgconfig/yaml-0.1.pc").is_file());

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/verify-exported-symbols.sh"))
            .arg(&stage_root),
        "verify-exported-symbols",
    );

    let nm_wrapper_dir = temp_dir("verify-symbols-wrapper");
    let nm_wrapper = nm_wrapper_dir.join("nm");
    let real_nm = env::var("NM").unwrap_or_else(|_| String::from("/usr/bin/nm"));
    fs::write(
        &nm_wrapper,
        format!(
            "#!/usr/bin/env bash\nset -euo pipefail\n\"{}\" \"$@\"\nprintf 'unexpected_extra_export T 0\\n'\n",
            real_nm
        ),
    )
    .expect("failed to write nm wrapper");
    let mut wrapper_permissions = fs::metadata(&nm_wrapper)
        .expect("failed to stat nm wrapper")
        .permissions();
    wrapper_permissions.set_mode(0o755);
    fs::set_permissions(&nm_wrapper, wrapper_permissions)
        .expect("failed to make nm wrapper executable");

    let wrapped_path = match env::var_os("PATH") {
        Some(path) => format!(
            "{}:{}",
            nm_wrapper_dir.display(),
            PathBuf::from(path).display()
        ),
        None => nm_wrapper_dir.display().to_string(),
    };
    let wrapped_verify = Command::new("bash")
        .arg(manifest_dir.join("scripts/verify-exported-symbols.sh"))
        .arg(&stage_root)
        .env("PATH", wrapped_path)
        .output()
        .expect("failed to run wrapped verify-exported-symbols");
    assert!(
        !wrapped_verify.status.success(),
        "verify-exported-symbols should reject unexpected non-yaml exports\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&wrapped_verify.stdout),
        String::from_utf8_lossy(&wrapped_verify.stderr)
    );

    let compiler = compiler();
    let binary = temp_dir("upstream-test-version").join("test-version");
    let compile_status = Command::new(&compiler)
        .arg("-std=c11")
        .arg(manifest_dir.join("compat/original-tests/test-version.c"))
        .arg("-I")
        .arg(stage_root.join("usr/include"))
        .arg("-L")
        .arg(&stage_lib_dir)
        .arg(format!("-Wl,-rpath,{}", stage_lib_dir.display()))
        .arg("-lyaml")
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("failed to compile upstream version test");
    assert!(
        compile_status.status.success(),
        "upstream version test failed to compile\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&compile_status.stdout),
        String::from_utf8_lossy(&compile_status.stderr)
    );

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/assert-staged-loader.sh"))
            .arg(&stage_root)
            .arg(&binary),
        "assert-staged-loader",
    );

    let run_output = Command::new(&binary)
        .output()
        .expect("failed to run upstream version test");
    assert!(
        run_output.status.success(),
        "upstream version test exited with failure\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let stdout = String::from_utf8(run_output.stdout).expect("upstream test emitted invalid UTF-8");
    assert!(stdout.contains("sizeof(token) = 80"));
    assert!(stdout.contains("sizeof(event) = 104"));
    assert!(stdout.contains("sizeof(parser) = 480"));
}

#[test]
fn verify_link_objects_script_passes_against_fixed_stage_root() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let stage_root = PathBuf::from("/tmp/libyaml-safe-install");

    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/stage-install.sh"))
            .arg(&stage_root),
        "stage-install /tmp/libyaml-safe-install",
    );
    run_command(
        Command::new("bash")
            .arg(manifest_dir.join("scripts/verify-link-objects.sh"))
            .arg(&stage_root),
        "verify-link-objects",
    );
}

#[test]
fn exported_symbols_route_through_ffi_boundaries() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for relative in [
        "src/lib.rs",
        "src/api.rs",
        "src/document.rs",
        "src/dumper.rs",
        "src/emitter.rs",
        "src/event.rs",
        "src/loader.rs",
        "src/parser.rs",
        "src/reader.rs",
        "src/scanner.rs",
        "src/writer.rs",
    ] {
        let source = fs::read_to_string(manifest_dir.join(relative))
            .expect("failed to read exported source");
        for block in source.split("#[no_mangle]").skip(1) {
            let signature = block
                .lines()
                .find(|line| line.contains("extern \"C\" fn"))
                .expect("missing exported signature")
                .trim()
                .to_owned();
            assert!(
                block.contains("ffi::int_boundary(")
                    || block.contains("ffi::ptr_boundary(")
                    || block.contains("ffi::const_ptr_boundary(")
                    || block.contains("ffi::void_boundary("),
                "{relative}: exported symbol does not route through ffi boundary: {signature}"
            );
        }
    }
}

#[test]
fn library_build_configuration_never_uses_panic_abort() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![
        manifest_dir.join("Cargo.toml"),
        manifest_dir.join("debian/rules"),
    ];
    if manifest_dir.join(".cargo/config.toml").is_file() {
        files.push(manifest_dir.join(".cargo/config.toml"));
    }

    for path in files {
        let contents = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read {}: {error}", path.display());
        });
        assert!(
            !contents.contains("panic = \"abort\"")
                && !contents.contains("panic=\"abort\"")
                && !contents.contains("-Cpanic=abort"),
            "unexpected abort panic mode in {}",
            path.display()
        );
    }
}

fn compiler() -> String {
    match env::var("CC") {
        Ok(value) if !value.is_empty() => value,
        _ => String::from("cc"),
    }
}

fn multiarch() -> String {
    for candidate in ["cc", "gcc"] {
        let output = Command::new(candidate).arg("-print-multiarch").output();
        if let Ok(value) = output {
            if value.status.success() {
                let arch = String::from_utf8_lossy(&value.stdout).trim().to_owned();
                if !arch.is_empty() {
                    return arch;
                }
            }
        }
    }

    format!("{}-linux-gnu", env::consts::ARCH)
}

fn run_command(command: &mut Command, label: &str) {
    let output = command.output().expect("failed to spawn command");
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("test-artifacts")
        .join(format!("{prefix}-{nonce}"));
    fs::create_dir_all(&dir).expect("failed to create temp directory");
    dir
}
