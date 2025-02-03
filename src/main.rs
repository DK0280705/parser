mod commands;
mod state;

use std::env;
use std::sync::Arc;

use tokio::signal;
use serenity::all::GuildId;
use serenity::async_trait;
use serenity::model::application::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use songbird::{driver::DecodeMode, Config, SerenityInit};
use state::State;

const TEST_GUILD_ID: u64 = 1116375728343228438;

struct Handler {
    state: Arc<State>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        println!("Parser is ready!");

        let test_guild_id = GuildId::new(TEST_GUILD_ID);
        test_guild_id
            .set_commands(&ctx.http, commands::create_all())
            .await
            .expect("Failed to register commands");
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                commands::process_command(self.state.as_ref(), &ctx, &command)
                    .await
                    .err()
                    .map(|err| {
                        eprintln!(
                            "Error running command {}: {err:?}",
                            command.data.name.as_str()
                        )
                    });
            }
            _ => {}
        }
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("PARSER_DISCORD_TOKEN").expect("Expected a token in the environment");
    let intents = GatewayIntents::non_privileged();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            state: Arc::new(State::new()),
        })
        .register_songbird_from_config(Config::default().decode_mode(DecodeMode::Decode))
        .await
        .expect("Error creating client");
    
    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        signal::ctrl_c().await.expect("Could not register ctrl+c handler");
        println!("Shutting down...");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}
