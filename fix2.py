import re

for f in ['rust/plugins/bentpipe/src/receive_manager.rs']:
    with open(f, 'r') as fp:
        content = fp.read()
    
    content = content.replace('(crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet))(std::ptr::null_mut()', '(crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet)(std::ptr::null_mut()')
    content = content.replace('(crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet))(std::ptr::null_mut()', '(crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet)(std::ptr::null_mut()')
    
    # Also if there's any other cases:
    content = content.replace('crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet(std::ptr::null_mut()', '(crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet)(std::ptr::null_mut()')
    content = content.replace('crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet(std::ptr::null_mut()', '(crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet)(std::ptr::null_mut()')

    with open(f, 'w') as fp:
        fp.write(content)

