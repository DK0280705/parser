mod record;
mod ping;

use record::VoiceHandler;
use dashmap::DashMap;
use serenity::{all::{CommandInteraction, Context, CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, GuildId}, async_trait};

pub (crate) struct State {
    pub record_channels: DashMap<GuildId, VoiceHandler>,
}

impl State {
    pub fn new() -> Self {
        State {
            record_channels: DashMap::new()
        }
    }
}

#[async_trait]
pub trait CommandManager: Sync + Send {
    async fn process_command(&self, ctx: &Context, itr: &CommandInteraction) -> Result<(), serenity::Error>;
    fn create_commands(&self) -> Vec<CreateCommand>;
}

#[async_trait]
impl CommandManager for State {
    async fn process_command(&self, ctx: &Context, itr: &CommandInteraction) -> Result<(), serenity::Error> {
        match itr.data.name.as_str() {
            "ping" => ping::run(self, &ctx, &itr).await,
            "record" => record::run(self, &ctx, &itr).await,
            _ => not_found_reply(self, &ctx, &itr).await,
        }
    }

    fn create_commands(&self) -> Vec<CreateCommand> {
        vec![ping::create(), record::create()]
    }
}

pub async fn reply(
    ctx: &Context,
    itr: &CommandInteraction,
    message: &str,
) -> Result<(), serenity::Error> {
    itr.create_response(
        &ctx.http,
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(message),
        ),
    )
    .await
}

async fn not_found_reply(
    _: &State,
    ctx: &Context,
    itr: &CommandInteraction,
) -> Result<(), serenity::Error> {
    reply(ctx, itr, "Command not found!").await
}

#[no_mangle]
pub extern "Rust" fn create_command_manager() -> Box<dyn CommandManager> {
    Box::new(State::new())
}