import re

cxx_code = open("src/libemane/frameworkphy.cc").read()

# Let's write a simple Rust mock for framework_phy.rs
rust_code = """
use std::ffi::c_void;

pub struct FrameworkPhy {
    // fields
}

impl FrameworkPhy {
    pub fn new() -> Self {
        Self {}
    }
}

#[no_mangle]
pub extern "C" fn emane_rs_framework_phy_create() -> *mut c_void {
    Box::into_raw(Box::new(FrameworkPhy::new())) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn emane_rs_framework_phy_destroy(ptr: *mut c_void) {
    if !ptr.is_null() {
        drop(Box::from_raw(ptr as *mut FrameworkPhy));
    }
}
"""
open("rust/emane-core/src/framework_phy.rs", "w").write(rust_code)
