use std::sync::Arc;

use parser::{CommandManager, CommandManagerCreateOption, CommandManagerPlugin};
use songbird::{driver::DecodeMode, Config, Songbird};

pub struct State {
    pub command_manager: Arc<dyn CommandManager>,
    pub voice_manager: Arc<Songbird>,
    _command_manager_plugin: CommandManagerPlugin,
}

impl State {
    pub fn new() -> Self {
        let plugin = CommandManagerPlugin::new();

        let vm_config = Config::default()
            .decode_mode(DecodeMode::Decode);
        let voice_manager = Songbird::serenity_from_config(vm_config);

        let cm_create_option = CommandManagerCreateOption {
            voice_manager: voice_manager.clone(),
        };

        Self {
            command_manager: Arc::from(plugin.create(cm_create_option).unwrap()),
            voice_manager: voice_manager,
            _command_manager_plugin: plugin,
        }
    }
}
