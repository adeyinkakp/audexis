use super::*;

#[unsafe(no_mangle)]
pub extern "C" fn audexis_mobile_create() -> *mut MobileCore {
    Box::into_raw(Box::new(MobileCore::default()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn audexis_mobile_call(
    core: *mut MobileCore,
    request: *const c_char,
) -> *mut c_char {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if core.is_null() || request.is_null() {
            return Err("Invalid mobile handle".into());
        }
        let request = unsafe { CStr::from_ptr(request) }
            .to_str()
            .map_err(|_| "Invalid UTF-8")?;
        let request: Value = serde_json::from_str(request).map_err(|_| "Invalid mobile request")?;
        unsafe { &mut *core }.call(request)
    }))
    .unwrap_or_else(|_| Err("Mobile operation failed".into()));
    let value = match result {
        Ok(value) => json!({"ok":value}),
        Err(error) => json!({"error":error}),
    };
    CString::new(value.to_string()).unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn audexis_mobile_destroy(core: *mut MobileCore) {
    if !core.is_null() {
        drop(unsafe { Box::from_raw(core) });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn audexis_mobile_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}
