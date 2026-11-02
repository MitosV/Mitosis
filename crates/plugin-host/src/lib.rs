use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use bytes::Bytes;
use mitosis_plugin_sdk::PluginMetadata;
use thiserror::Error;
use tracing::{error, info, warn};
use wasmtime::*;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("Wasmtime engine error: {0}")]
    Wasmtime(#[from] wasmtime::Error),
    #[error("Memory access error: {0}")]
    MemoryAccess(#[from] wasmtime::MemoryAccessError),
    #[error("Plugin metadata not found in custom section 'mitosis_metadata'")]
    MissingMetadata,
    #[error("Metadata parse error: {0}")]
    MetadataParse(#[from] serde_json::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Plugin '{0}' not found")]
    NotFound(String),
    #[error("Action execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone, Default)]
pub struct HostState {
    pub registered_actions: Arc<Mutex<HashMap<String, String>>>, // action_name -> description
    pub sent_payloads: Arc<Mutex<Vec<(u32, String, Bytes)>>>,    // (bot_id, channel, data)
    pub assertion_failures: Arc<Mutex<Vec<String>>>,
}

pub struct LoadedPlugin {
    pub metadata: PluginMetadata,
    pub path: PathBuf,
    pub module: Module,
}

pub struct PluginHost {
    engine: Engine,
    linker: Linker<HostState>,
    plugins: HashMap<String, LoadedPlugin>,
    host_state: HostState,
}

impl PluginHost {
    pub fn new() -> Result<Self, PluginError> {
        let mut config = Config::new();
        config.cranelift_opt_level(OptLevel::Speed);
        config.consume_fuel(true);

        let engine = Engine::new(&config)?;
        let mut linker = Linker::new(&engine);
        let host_state = HostState::default();

        // Register host functions
        Self::bind_host_functions(&mut linker)?;

        Ok(Self {
            engine,
            linker,
            plugins: HashMap::new(),
            host_state,
        })
    }

    fn bind_host_functions(linker: &mut Linker<HostState>) -> Result<(), wasmtime::Error> {
        // mitosis_host_log(level: u32, msg_ptr: *const u8, msg_len: usize)
        linker.func_wrap(
            "env",
            "mitosis_host_log",
            |mut caller: Caller<'_, HostState>, level: u32, ptr: u32, len: u32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return,
                };
                let data = mem.data(&caller);
                let start = ptr as usize;
                let end = start + len as usize;
                if end <= data.len() {
                    let msg = String::from_utf8_lossy(&data[start..end]);
                    match level {
                        1 => info!("[WASM Plugin] {}", msg),
                        2 => warn!("[WASM Plugin] {}", msg),
                        3 => error!("[WASM Plugin] {}", msg),
                        _ => info!("[WASM Plugin] {}", msg),
                    }
                }
            },
        )?;

        // mitosis_host_register_action(action_ptr: *const u8, action_len: usize, desc_ptr: *const u8, desc_len: usize)
        linker.func_wrap(
            "env",
            "mitosis_host_register_action",
            |mut caller: Caller<'_, HostState>, a_ptr: u32, a_len: u32, d_ptr: u32, d_len: u32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return,
                };
                let data = mem.data(&caller);
                let a_end = (a_ptr + a_len) as usize;
                let d_end = (d_ptr + d_len) as usize;
                if a_end <= data.len() && d_end <= data.len() {
                    let action = String::from_utf8_lossy(&data[a_ptr as usize..a_end]).to_string();
                    let desc = String::from_utf8_lossy(&data[d_ptr as usize..d_end]).to_string();
                    caller.data().registered_actions.lock().unwrap().insert(action, desc);
                }
            },
        )?;

        // mitosis_host_send_custom_payload(bot_id: u32, channel_ptr, channel_len, data_ptr, data_len)
        linker.func_wrap(
            "env",
            "mitosis_host_send_custom_payload",
            |mut caller: Caller<'_, HostState>,
             bot_id: u32,
             c_ptr: u32,
             c_len: u32,
             d_ptr: u32,
             d_len: u32| {
                let mem = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => return,
                };
                let data = mem.data(&caller);
                let c_end = (c_ptr + c_len) as usize;
                let d_end = (d_ptr + d_len) as usize;
                if c_end <= data.len() && d_end <= data.len() {
                    let channel = String::from_utf8_lossy(&data[c_ptr as usize..c_end]).to_string();
                    let payload = Bytes::copy_from_slice(&data[d_ptr as usize..d_end]);
                    caller
                        .data()
                        .sent_payloads
                        .lock()
                        .unwrap()
                        .push((bot_id, channel, payload));
                }
            },
        )?;

        // mitosis_host_assert(cond: u32, msg_ptr, msg_len)
        linker.func_wrap(
            "env",
            "mitosis_host_assert",
            |mut caller: Caller<'_, HostState>, cond: u32, ptr: u32, len: u32| {
                if cond == 0 {
                    let mem = match caller.get_export("memory") {
                        Some(Extern::Memory(m)) => m,
                        _ => return,
                    };
                    let data = mem.data(&caller);
                    let end = (ptr + len) as usize;
                    if end <= data.len() {
                        let msg = String::from_utf8_lossy(&data[ptr as usize..end]).to_string();
                        error!("[WASM Assertion Failed] {}", msg);
                        caller.data().assertion_failures.lock().unwrap().push(msg);
                    }
                }
            },
        )?;

        // mitosis_host_send_chat(bot_id: u32, msg_ptr, msg_len)
        linker.func_wrap(
            "env",
            "mitosis_host_send_chat",
            |_caller: Caller<'_, HostState>, _bot_id: u32, _ptr: u32, _len: u32| {
                // Sent to bot runtime via channels
            },
        )?;

        Ok(())
    }

    /// Loads a plugin from a .wasm file path.
    pub fn load_plugin(&mut self, path: &Path) -> Result<PluginMetadata, PluginError> {
        let bytes = fs::read(path)?;
        let metadata = Self::extract_metadata(&bytes)?;

        let module = Module::new(&self.engine, &bytes)?;

        // Instantiate to run initialization & register actions
        let mut store = Store::new(&self.engine, self.host_state.clone());
        store.set_fuel(1_000_000)?;

        let instance = self.linker.instantiate(&mut store, &module)?;
        if let Ok(init_fn) = instance.get_typed_func::<(), ()>(&mut store, "mitosis_init") {
            let _ = init_fn.call(&mut store, ());
        }

        let meta_clone = metadata.clone();
        self.plugins.insert(
            metadata.id.clone(),
            LoadedPlugin {
                metadata,
                path: path.to_path_buf(),
                module,
            },
        );

        Ok(meta_clone)
    }

    /// Extracts `mitosis_metadata` custom section from WASM binary.
    pub fn extract_metadata(wasm_bytes: &[u8]) -> Result<PluginMetadata, PluginError> {
        // Wasm binary custom section search:
        // A section begins with section ID (0 for custom), VarUint32 size, name string, payload.
        let mut i = 8; // Skip 4-byte magic + 4-byte version
        while i < wasm_bytes.len() {
            let sec_id = wasm_bytes[i];
            i += 1;
            let (sec_len, len_bytes) = Self::decode_u32_leb128(&wasm_bytes[i..])?;
            i += len_bytes;
            let sec_end = i + sec_len as usize;

            if sec_id == 0 && sec_end <= wasm_bytes.len() {
                // Custom section: read name
                let (name_len, name_len_bytes) = Self::decode_u32_leb128(&wasm_bytes[i..])?;
                let name_start = i + name_len_bytes;
                let name_end = name_start + name_len as usize;
                if name_end <= sec_end {
                    let name = String::from_utf8_lossy(&wasm_bytes[name_start..name_end]);
                    if name == "mitosis_metadata" {
                        let payload = &wasm_bytes[name_end..sec_end];
                        let meta: PluginMetadata = serde_json::from_slice(payload)?;
                        return Ok(meta);
                    }
                }
            }
            i = sec_end;
        }

        Err(PluginError::MissingMetadata)
    }

    fn decode_u32_leb128(bytes: &[u8]) -> Result<(u32, usize), PluginError> {
        let mut result = 0u32;
        let mut shift = 0;
        let mut count = 0;
        for &b in bytes {
            count += 1;
            result |= ((b & 0x7F) as u32) << shift;
            if (b & 0x80) == 0 {
                return Ok((result, count));
            }
            shift += 7;
            if shift >= 35 {
                break;
            }
        }
        Err(PluginError::MissingMetadata)
    }

    pub fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.values().map(|p| p.metadata.clone()).collect()
    }

    pub fn get_registered_actions(&self) -> HashMap<String, String> {
        self.host_state.registered_actions.lock().unwrap().clone()
    }

    /// Executes an action registered by a plugin for a specific bot.
    pub fn execute_action(
        &self,
        plugin_id: &str,
        bot_id: u32,
        action: &str,
        args: &str,
    ) -> Result<u32, PluginError> {
        let plugin = self
            .plugins
            .get(plugin_id)
            .ok_or_else(|| PluginError::NotFound(plugin_id.to_string()))?;

        let mut store = Store::new(&self.engine, self.host_state.clone());
        store.set_fuel(5_000_000)?;

        let instance = self.linker.instantiate(&mut store, &plugin.module)?;

        let alloc_fn = instance
            .get_typed_func::<u32, u32>(&mut store, "mitosis_alloc")
            .map_err(|e| PluginError::ExecutionFailed(format!("Missing mitosis_alloc: {}", e)))?;

        // Allocate action name and args in WASM memory
        let action_bytes = action.as_bytes();
        let args_bytes = args.as_bytes();

        let action_ptr = alloc_fn.call(&mut store, action_bytes.len() as u32)?;
        let args_ptr = alloc_fn.call(&mut store, args_bytes.len() as u32)?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| PluginError::ExecutionFailed("Missing memory".into()))?;

        memory.write(&mut store, action_ptr as usize, action_bytes)?;
        memory.write(&mut store, args_ptr as usize, args_bytes)?;

        let exec_fn = instance
            .get_typed_func::<(u32, u32, u32, u32, u32), u32>(&mut store, "mitosis_execute_action")
            .map_err(|e| PluginError::ExecutionFailed(format!("Missing mitosis_execute_action: {}", e)))?;

        let result = exec_fn.call(
            &mut store,
            (
                bot_id,
                action_ptr,
                action_bytes.len() as u32,
                args_ptr,
                args_bytes.len() as u32,
            ),
        )?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_execute_plugin() {
        let wasm_path = Path::new("../../plugins/d4games.wasm");
        if !wasm_path.exists() {
            return;
        }

        let mut host = PluginHost::new().unwrap();
        let meta = host.load_plugin(wasm_path).unwrap();

        assert_eq!(meta.id, "d4games");
        assert_eq!(meta.name, "D4Games Automation Plugin");
        assert_eq!(meta.version, "1.0.0");

        let actions = host.get_registered_actions();
        assert!(actions.contains_key("complete_game"));
        assert!(actions.contains_key("select_option"));

        // Execute action
        let res = host.execute_action("d4games", 1, "complete_game", "").unwrap();
        assert_eq!(res, 0);

        let payloads = host.host_state.sent_payloads.lock().unwrap();
        assert_eq!(payloads.len(), 1);
        assert_eq!(payloads[0].0, 1);
        assert_eq!(payloads[0].1, "d4games:action");
    }
}
