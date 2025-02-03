use serenity::all::{CommandInteraction, Context, CreateCommand};

use crate::state::State;

use super::reply;

pub async fn run(
    _: &State,
    ctx: &Context,
    itr: &CommandInteraction,
) -> Result<(), serenity::Error> {
    reply(ctx, itr, "Pong!").await
}

pub fn create() -> CreateCommand {
    CreateCommand::new("ping").description("Ping!")
}
