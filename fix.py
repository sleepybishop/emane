import re
import os

for f in ['rust/plugins/bentpipe/src/radio_model.rs', 'rust/plugins/bentpipe/src/receive_manager.rs']:
    with open(f, 'r') as fp:
        content = fp.read()
    
    # Remove #[link(...)]
    content = re.sub(r'#\[link\(name = "[^"]+", kind = "dylib"\)\]\nextern "C" \{', 'extern "C" {', content)
    
    start = content.find('extern "C" {')
    if start != -1:
        end = content.find('}', start)
        block = content[start+12:end]
        
        new_block = []
        
        # We need to collect full function signatures spanning multiple lines
        lines = block.split('\n')
        current_sig = ""
        
        for line in lines:
            line_stripped = line.strip()
            if not line_stripped or line_stripped.startswith('//'):
                continue
                
            current_sig += " " + line_stripped
            
            if current_sig.endswith(';'):
                sig = current_sig.strip()[:-1] # remove ;
                
                # Replace pub fn with pub unsafe fn
                if sig.startswith('pub fn '):
                    sig = sig.replace('pub fn ', 'pub unsafe fn ', 1)
                elif sig.startswith('fn '):
                    sig = sig.replace('fn ', 'pub unsafe fn ', 1)
                
                # Determine dummy return value
                if '->' in sig:
                    ret = sig.split('->')[1].strip()
                    if ret in ['u16', 'u8', 'usize', 'u64', 'i32', 'c_int']:
                        body = ' { 0 }'
                    elif ret == 'bool':
                        body = ' { false }'
                    elif ret in ['f64', 'f32']:
                        body = ' { 0.0 }'
                    elif ret.startswith('*const') or ret.startswith('*mut'):
                        body = ' { std::ptr::null_mut() }'
                    else:
                        body = ' {}'
                else:
                    body = ' {}'
                
                # For packet sending, the instructions say replace with FW_SERVICE
                if 'emane_bentpipe_receivemanager_cxx_forward_upstream' in sig and not 'parts' in sig:
                    body = ' { crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0); }'
                elif 'emane_bentpipe_receivemanager_cxx_bend_downstream' in sig and not 'parts' in sig:
                    body = ' { crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0); }'
                elif 'emane_bentpipe_receivemanager_cxx_forward_upstream_parts' in sig:
                    body = ' { crate::FW_SERVICE.as_ref().unwrap().send_upstream_packet(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0); }'
                elif 'emane_bentpipe_receivemanager_cxx_bend_downstream_parts' in sig:
                    body = ' { crate::FW_SERVICE.as_ref().unwrap().send_downstream_packet(std::ptr::null_mut(), 0, std::ptr::null(), std::ptr::null(), 0); }'
                
                new_block.append(sig + body)
                current_sig = ""
        
        content = content[:start] + '\n'.join(new_block) + content[end+1:]
        
    with open(f, 'w') as fp:
        fp.write(content)

