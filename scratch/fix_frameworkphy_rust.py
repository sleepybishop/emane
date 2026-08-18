import re

with open("rust/emane-core/src/framework_phy.rs", "r") as f:
    text = f.read()

text = text.replace("pub struct FrameworkPhy {", "pub struct FrameworkPhy {\n    pub cpp_this: *mut c_void,")
text = text.replace("pub fn new() -> Self {\n        Self {", "pub fn new() -> Self {\n        Self {\n            cpp_this: std::ptr::null_mut(),")
text = text.replace("self as *mut _ as *mut c_void", "self.cpp_this")

text = text.replace('pub extern "C" fn emane_rs_framework_phy_create() -> *mut c_void {', 'pub extern "C" fn emane_rs_framework_phy_create(cpp_this: *mut c_void) -> *mut c_void {')
text = text.replace('Box::into_raw(Box::new(FrameworkPhy::new()))', 'Box::into_raw(Box::new({ let mut phy = FrameworkPhy::new(); phy.cpp_this = cpp_this; phy }))')

with open("rust/emane-core/src/framework_phy.rs", "w") as f:
    f.write(text)

with open("rust/emane-core/src/framework_phy_downstream.rs", "r") as f:
    text = f.read()
text = text.replace("phy as *mut _ as *mut c_void", "phy.cpp_this")
with open("rust/emane-core/src/framework_phy_downstream.rs", "w") as f:
    f.write(text)
