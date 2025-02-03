mod ping;
mod record;

use crate::state::State;
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};

pub async fn process_command(
    state: &State,
    context: &Context,
    interaction: &CommandInteraction,
) -> Result<(), serenity::Error> {
    match interaction.data.name.as_str() {
        "ping" => ping::run(state, context, interaction).await,
        "record" => record::run(state, context, interaction).await,
        _ => not_found_reply(state, context, interaction).await,
    }
}

pub fn create_all() -> Vec<CreateCommand> {
    vec![ping::create(), record::create()]
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
