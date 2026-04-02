use std::ffi::CStr;
use std::mem::size_of;

use yaml::{yaml_event_t, yaml_get_version, yaml_get_version_string, yaml_parser_t, yaml_token_t};

#[test]
fn version_tuple_matches_version_string() {
    unsafe {
        let mut major = -1;
        let mut minor = -1;
        let mut patch = -1;
        yaml_get_version(&mut major, &mut minor, &mut patch);
        let version = format!("{major}.{minor}.{patch}");
        assert_eq!(
            version,
            CStr::from_ptr(yaml_get_version_string()).to_string_lossy()
        );
    }
}

#[test]
fn public_abi_sizes_match_upstream_version_probe() {
    assert_eq!(size_of::<yaml_token_t>(), 80);
    assert_eq!(size_of::<yaml_event_t>(), 104);
    assert_eq!(size_of::<yaml_parser_t>(), 480);
}
