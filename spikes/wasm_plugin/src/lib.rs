static VENDOR: &str = "VendorX Experimental";

#[no_mangle]
pub extern "C" fn vendor_name_ptr() -> *const u8 {
    VENDOR.as_ptr()
}

#[no_mangle]
pub extern "C" fn vendor_name_len() -> usize {
    VENDOR.len()
}

#[no_mangle]
pub extern "C" fn capabilities_mask() -> u32 {
    const SUPPORTS_COMMIT: u32 = 1 << 0;
    const SUPPORTS_ROLLBACK: u32 = 1 << 1;
    const SUPPORTS_DIFF: u32 = 1 << 2;

    SUPPORTS_COMMIT | SUPPORTS_ROLLBACK | SUPPORTS_DIFF
}

