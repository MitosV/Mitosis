use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub authors: Vec<String>,
    pub description: String,
    pub api_version: String,
    pub minecraft_versions: Vec<String>,
    pub capabilities: Vec<String>,
}

#[macro_export]
macro_rules! export_mitosis_plugin {
    ($meta:expr) => {
        #[link_section = "mitosis_metadata"]
        #[no_mangle]
        pub static MITOSIS_METADATA: [u8; $meta.len()] = *$meta;
    };
}

// Host functions imported by WebAssembly plugins
extern "C" {
    fn mitosis_host_log(level: u32, msg_ptr: *const u8, msg_len: usize);
    fn mitosis_host_send_custom_payload(
        bot_id: u32,
        channel_ptr: *const u8,
        channel_len: usize,
        data_ptr: *const u8,
        data_len: usize,
    );
    fn mitosis_host_register_action(
        action_ptr: *const u8,
        action_len: usize,
        desc_ptr: *const u8,
        desc_len: usize,
    );
    fn mitosis_host_send_chat(bot_id: u32, msg_ptr: *const u8, msg_len: usize);
    fn mitosis_host_assert(condition: u32, msg_ptr: *const u8, msg_len: usize);
}

pub fn log_info(msg: &str) {
    unsafe {
        mitosis_host_log(1, msg.as_ptr(), msg.len());
    }
}

pub fn log_warn(msg: &str) {
    unsafe {
        mitosis_host_log(2, msg.as_ptr(), msg.len());
    }
}

pub fn log_error(msg: &str) {
    unsafe {
        mitosis_host_log(3, msg.as_ptr(), msg.len());
    }
}

pub fn register_action(name: &str, description: &str) {
    unsafe {
        mitosis_host_register_action(
            name.as_ptr(),
            name.len(),
            description.as_ptr(),
            description.len(),
        );
    }
}

pub fn send_custom_payload(bot_id: u32, channel: &str, data: &[u8]) {
    unsafe {
        mitosis_host_send_custom_payload(
            bot_id,
            channel.as_ptr(),
            channel.len(),
            data.as_ptr(),
            data.len(),
        );
    }
}

pub fn send_chat(bot_id: u32, message: &str) {
    unsafe {
        mitosis_host_send_chat(bot_id, message.as_ptr(), message.len());
    }
}

pub fn assert_true(condition: bool, message: &str) {
    unsafe {
        mitosis_host_assert(
            if condition { 1 } else { 0 },
            message.as_ptr(),
            message.len(),
        );
    }
}

// Memory allocator for host to allocate strings/buffers in WASM memory
#[no_mangle]
pub extern "C" fn mitosis_alloc(size: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub unsafe extern "C" fn mitosis_dealloc(ptr: *mut u8, size: usize) {
    let _ = Vec::from_raw_parts(ptr, 0, size);
}
