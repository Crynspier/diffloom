#![allow(unsafe_code)]

//! C ABI for Diffloom.

use diffloom_core::{
    patch_apply, patch_from_text, patch_make, patch_to_text, DiffEngine, DiffOptions, MatchOptions,
    Operation,
};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

unsafe fn read_str(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .ok()
        .map(str::to_owned)
}

fn into_c(value: String) -> *mut c_char {
    CString::new(value).map_or(ptr::null_mut(), CString::into_raw)
}

/// # Safety
///
/// ptr must be null or a valid allocation previously returned by Diffloom and
/// must not have been freed already.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diffloom_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(ptr));
    }
}

/// # Safety
///
/// old and new must be null or valid NUL-terminated UTF-8 C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diffloom_diff(
    old: *const c_char,
    new: *const c_char,
) -> *mut c_char {
    let (Some(old), Some(new)) = (unsafe { read_str(old) }, unsafe { read_str(new) }) else {
        return ptr::null_mut();
    };

    let diffs = DiffEngine::new(DiffOptions::default()).diff(&old, &new);
    let mut out = String::new();

    for diff in diffs {
        let code = match diff.operation {
            Operation::Delete => 'D',
            Operation::Insert => 'I',
            Operation::Equal => 'E',
        };
        out.push(code);
        out.push('\t');
        out.push_str(&diff.text.replace('\n', "\\n"));
        out.push('\n');
    }

    into_c(out)
}

/// # Safety
///
/// old and new must be null or valid NUL-terminated UTF-8 C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diffloom_patch_make(
    old: *const c_char,
    new: *const c_char,
) -> *mut c_char {
    let (Some(old), Some(new)) = (unsafe { read_str(old) }, unsafe { read_str(new) }) else {
        return ptr::null_mut();
    };

    let patches = patch_make(&old, &new, DiffOptions::default());
    into_c(patch_to_text(&patches))
}

/// # Safety
///
/// patch and input must be null or valid NUL-terminated UTF-8 C strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn diffloom_patch_apply(
    patch: *const c_char,
    input: *const c_char,
) -> *mut c_char {
    let (Some(patch), Some(input)) =
        (unsafe { read_str(patch) }, unsafe { read_str(input) })
    else {
        return ptr::null_mut();
    };

    let parsed = match patch_from_text(&patch) {
        Ok(value) => value,
        Err(_) => return ptr::null_mut(),
    };

    let result = match patch_apply(&parsed, &input, MatchOptions::default()) {
        Ok(value) => value,
        Err(_) => return ptr::null_mut(),
    };

    if result.applied.iter().any(|value| !value) {
        return ptr::null_mut();
    }

    into_c(result.text)
}

/// Returns the stable C ABI version.
#[unsafe(no_mangle)]
pub extern "C" fn diffloom_abi_version() -> c_int {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_is_stable() {
        assert_eq!(diffloom_abi_version(), 1);
    }

    #[test]
    fn c_diff_roundtrip() {
        let old = CString::new("hello").unwrap();
        let new = CString::new("hullo").unwrap();
        let value = unsafe { diffloom_diff(old.as_ptr(), new.as_ptr()) };
        assert!(!value.is_null());

        let result = unsafe { CStr::from_ptr(value) }
            .to_str()
            .unwrap()
            .to_owned();

        unsafe { diffloom_free_string(value) };

        assert!(result.contains("D"));
        assert!(result.contains("I"));
    }
}
