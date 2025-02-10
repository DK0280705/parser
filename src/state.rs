use std::sync::Arc;
use libloading::{Library, Symbol};
use serenity::{all::{CommandInteraction, Context, CreateCommand}, async_trait};

#[cfg(target_os = "windows")]
const LIB_EXT: &str = ".dll";
#[cfg(target_os = "linux")]
const LIB_EXT: &str = ".so";

#[async_trait]
pub trait CommandManager: Sync + Send {
    async fn process_command(&self, ctx: &Context, itr: &CommandInteraction) -> Result<(), serenity::Error>;
    fn create_commands(&self) -> Vec<CreateCommand>;
}

type CommandManagerBuilder = fn() -> Box<dyn CommandManager>;
struct CommandManagerPlugin {
    library: Library,
}

impl CommandManagerPlugin {
    pub fn new() -> Self {
        Self {
            library: unsafe { Library::new(format!("parser_commands{}", LIB_EXT)).unwrap() }
        }
    }
    pub fn create(&self) -> Result<Box<dyn CommandManager>, libloading::Error> {
        let create: Symbol<CommandManagerBuilder> = unsafe { self.library.get(b"create_command_manager")? };
        Ok(create())
    }
}

pub struct State {
    pub command_manager: Arc<dyn CommandManager>,
    _command_manager_plugin: CommandManagerPlugin,
}

impl State {
    pub fn new() -> Self {
        let plugin = CommandManagerPlugin::new();
        Self {
            command_manager: Arc::from(plugin.create().unwrap()),
            _command_manager_plugin: plugin
        }
    }
}