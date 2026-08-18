import re

with open('src/libemane/commonlayerstatistics.cc', 'r') as f:
    cc_content = f.read()

# Extract numeric vars and table vars
numeric_vars = re.findall(r'StatisticNumeric<Counter> \* (pNum[a-zA-Z_0-9]+);', cc_content)
numeric_vars.append('pStatisticUpstreamProcessingDelay_')
numeric_vars.append('pStatisticDownstreamProcessingDelay_')
table_vars = re.findall(r'StatisticTable<NEMId> \* (pStatistic[a-zA-Z_0-9]+Table_);', cc_content)

ffi_fields = "\n".join([f"    void* {v};" for v in numeric_vars + table_vars])
rust_fields = "\n".join([f"    pub {v.lower()}: *mut c_void," for v in numeric_vars + table_vars])

rust_init = "\n".join([f"        {v.lower()}: std::ptr::null_mut()," for v in numeric_vars + table_vars])

set_pointers_body = "\n".join([f"    state->{v.lower()} = ptrs->{v};" for v in numeric_vars + table_vars])

rust_code = f"""
use std::os::raw::c_void;

#[repr(C)]
pub struct ClsPointers {{
{ffi_fields}
}}

pub struct CommonLayerStatisticsState {{
{rust_fields}
    // Drop codes
    pub unicast_drop_map: std::collections::HashMap<u16, Vec<u64>>,
    pub broadcast_drop_map: std::collections::HashMap<u16, Vec<u64>>,
    
    // Accept maps
    pub unicast_accept_map: std::collections::HashMap<u16, [u64; 4]>,
    pub broadcast_accept_map: std::collections::HashMap<u16, [u64; 4]>,
}}

#[no_mangle]
pub extern "C" fn emane_rs_cls_new() -> *mut c_void {{
    let state = Box::new(CommonLayerStatisticsState {{
{rust_init}
        unicast_drop_map: std::collections::HashMap::new(),
        broadcast_drop_map: std::collections::HashMap::new(),
        unicast_accept_map: std::collections::HashMap::new(),
        broadcast_accept_map: std::collections::HashMap::new(),
    }});
    Box::into_raw(state) as *mut c_void
}}

#[no_mangle]
pub extern "C" fn emane_rs_cls_destroy(ptr: *mut c_void) {{
    if !ptr.is_null() {{
        unsafe {{ drop(Box::from_raw(ptr as *mut CommonLayerStatisticsState)); }}
    }}
}}

#[no_mangle]
pub extern "C" fn emane_rs_cls_set_pointers(ptr: *mut c_void, ptrs: *const ClsPointers) {{
    let state = unsafe {{ &mut *(ptr as *mut CommonLayerStatisticsState) }};
    let ptrs = unsafe {{ &*ptrs }};
{set_pointers_body}
}}
"""

with open('rust/emane-core/src/common_layer_statistics.rs', 'w') as f:
    f.write(rust_code)
