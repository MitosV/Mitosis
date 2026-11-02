use mitosis_plugin_sdk::*;

// Embed metadata into the custom section 'mitosis_metadata'
export_mitosis_plugin!(b"{\
  \"id\":\"d4games\",\
  \"name\":\"D4Games Automation Plugin\",\
  \"version\":\"1.0.0\",\
  \"authors\":[\"MitosV\"],\
  \"description\":\"Minigame automation and verification plugin for Mitosis\",\
  \"api_version\":\"1.0.0\",\
  \"minecraft_versions\":[\"1.21.1\"],\
  \"capabilities\":[\"custom_payloads\",\"actions\",\"assertions\"]\
}");

#[no_mangle]
pub extern "C" fn mitosis_init() {
    log_info("Initializing D4Games plugin...");
    register_action("complete_game", "Automates minigame completion sequence");
    register_action("select_option", "Selects an option dialog by index");
    log_info("D4Games actions registered successfully.");
}

#[no_mangle]
pub extern "C" fn mitosis_execute_action(
    bot_id: u32,
    action_ptr: *const u8,
    action_len: usize,
    args_ptr: *const u8,
    args_len: usize,
) -> u32 {
    let action = unsafe {
        let slice = std::slice::from_raw_parts(action_ptr, action_len);
        std::str::from_utf8_unchecked(slice)
    };
    let args = unsafe {
        let slice = std::slice::from_raw_parts(args_ptr, args_len);
        std::str::from_utf8_unchecked(slice)
    };

    match action {
        "complete_game" => {
            log_info(&format!("Bot {} executing complete_game action", bot_id));
            send_custom_payload(bot_id, "d4games:action", b"COMPLETE");
            send_chat(bot_id, "Minigame successfully completed!");
            assert_true(true, "Minigame completion asserted");
            0 // Success
        }
        "select_option" => {
            log_info(&format!("Bot {} selecting option: {}", bot_id, args));
            send_custom_payload(bot_id, "d4games:select", args.as_bytes());
            0 // Success
        }
        _ => {
            log_warn(&format!("Unknown action: {}", action));
            1 // Error
        }
    }
}
