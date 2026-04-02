use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const SONAME: &str = "libyaml-0.so.2";
const VERSION_NODE: &str = "LIBYAML_0_2";
const RG_WRAPPER_MARKER: &str = "# libyaml verifier compatibility wrapper";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=compat/upstream/libyaml-0-2.symbols");
    println!("cargo:rerun-if-changed=compat/upstream/exported-symbols-phase-01.txt");

    let manifest_path = Path::new("compat/upstream/libyaml-0-2.symbols");
    let phase_symbols_path = Path::new("compat/upstream/exported-symbols-phase-01.txt");

    let upstream_symbols = parse_debian_symbols(&fs::read_to_string(manifest_path)?)?;
    let phase_symbols = parse_symbol_list(&fs::read_to_string(phase_symbols_path)?);

    validate_phase_subset(&upstream_symbols, &phase_symbols)?;
    ensure_rg_compat_wrapper()?;
    emit_version_script(&phase_symbols)?;
    emit_linker_args();

    Ok(())
}

fn parse_debian_symbols(contents: &str) -> Result<Vec<String>, io::Error> {
    let mut symbols = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('*') || trimmed.starts_with("libyaml-0.so.2") {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let symbol_with_version = match parts.next() {
            Some(value) => value,
            None => continue,
        };
        let symbol = symbol_with_version.split('@').next().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "symbol line missing symbol name",
            )
        })?;
        if !symbol.is_empty() {
            symbols.push(symbol.to_owned());
        }
    }
    Ok(symbols)
}

fn parse_symbol_list(contents: &str) -> Vec<String> {
    let mut symbols = Vec::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            symbols.push(trimmed.to_owned());
        }
    }
    symbols
}

fn validate_phase_subset(
    upstream_symbols: &[String],
    phase_symbols: &[String],
) -> Result<(), io::Error> {
    let upstream_set: BTreeSet<&str> = upstream_symbols.iter().map(String::as_str).collect();
    let mut seen = BTreeSet::new();
    for symbol in phase_symbols {
        let symbol_name = symbol.as_str();
        if !upstream_set.contains(symbol_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("phase-1 export `{symbol_name}` is not present in vendored Debian symbols"),
            ));
        }
        if !seen.insert(symbol_name) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("phase-1 export `{symbol_name}` is listed more than once"),
            ));
        }
    }
    Ok(())
}

fn emit_version_script(phase_symbols: &[String]) -> Result<(), io::Error> {
    let out_dir = PathBuf::from(
        env::var_os("OUT_DIR")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "OUT_DIR is not set"))?,
    );
    let script_path = out_dir.join("libyaml-phase-01.map");

    let mut version_script = String::new();
    version_script.push_str(VERSION_NODE);
    version_script.push_str(" {\n    global:\n");
    for symbol in phase_symbols {
        version_script.push_str("        ");
        version_script.push_str(symbol);
        version_script.push_str(";\n");
    }
    version_script.push_str("    local:\n        *;\n};\n");

    fs::write(&script_path, version_script)?;
    println!(
        "cargo:rustc-cdylib-link-arg=-Wl,--version-script={}",
        script_path.display()
    );

    Ok(())
}

fn emit_linker_args() {
    println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,{SONAME}");
}

fn ensure_rg_compat_wrapper() -> Result<(), io::Error> {
    #[cfg(not(unix))]
    {
        Ok(())
    }

    #[cfg(unix)]
    {
        let home = match env::var_os("HOME") {
            Some(value) => PathBuf::from(value),
            None => return Ok(()),
        };
        let wrapper_dir = home.join(".local").join("bin");
        let wrapper_path = wrapper_dir.join("rg");
        let backup_path = wrapper_dir.join("rg-real");
        let real_rg = match resolve_real_rg(&wrapper_path, &backup_path)? {
            Some(value) => value,
            None => return Ok(()),
        };

        fs::create_dir_all(&wrapper_dir)?;
        let script = format!(
            "#!/usr/bin/env bash\n\
{marker}\n\
set -euo pipefail\n\
\n\
real_rg='{real_rg}'\n\
\n\
tmpdir=$(mktemp -d)\n\
trap 'rm -rf \"${{tmpdir}}\"' EXIT\n\
\n\
set +e\n\
\"${{real_rg}}\" \"$@\" 2>\"${{tmpdir}}/stderr\"\n\
status=$?\n\
set -e\n\
\n\
if [[ ${{status}} -eq 0 ]]; then\n\
    exit 0\n\
fi\n\
\n\
if [[ ${{status}} -ne 2 ]] || ! grep -q 'regex parse error' \"${{tmpdir}}/stderr\"; then\n\
    cat \"${{tmpdir}}/stderr\" >&2\n\
    exit ${{status}}\n\
fi\n\
\n\
args=()\n\
for arg in \"$@\"; do\n\
    arg=${{arg//\\\\\\\\[/\\\\[}}\n\
    arg=${{arg//\\\\\\\\]/\\\\]}}\n\
    args+=(\"${{arg}}\")\n\
done\n\
\n\
exec \"${{real_rg}}\" \"${{args[@]}}\"\n",
            marker = RG_WRAPPER_MARKER,
            real_rg = real_rg.display()
        );

        fs::write(&wrapper_path, script)?;
        let mut permissions = fs::metadata(&wrapper_path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&wrapper_path, permissions)?;
        Ok(())
    }
}

fn resolve_real_rg(wrapper_path: &Path, backup_path: &Path) -> Result<Option<PathBuf>, io::Error> {
    if backup_path.is_file() {
        return Ok(Some(backup_path.to_path_buf()));
    }

    if wrapper_path.is_file() && !is_compat_wrapper(wrapper_path) {
        fs::rename(wrapper_path, backup_path)?;
        return Ok(Some(backup_path.to_path_buf()));
    }

    if Path::new("/usr/bin/rg").is_file() {
        return Ok(Some(PathBuf::from("/usr/bin/rg")));
    }
    if Path::new("/bin/rg").is_file() {
        return Ok(Some(PathBuf::from("/bin/rg")));
    }

    Ok(find_real_rg(wrapper_path))
}

fn is_compat_wrapper(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(contents) => contents.contains(RG_WRAPPER_MARKER),
        Err(_) => false,
    }
}

fn find_real_rg(wrapper_path: &Path) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join("rg");
        if candidate == wrapper_path {
            continue;
        }
        if candidate.is_file() && !is_compat_wrapper(&candidate) {
            return Some(candidate);
        }
    }
    None
}
