mod state;

use std::env;
use std::sync::Arc;

use dotenv::dotenv;

use serenity::{
    all::{Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready},
    async_trait, Client,
};
use tokio::{signal, sync::RwLock};

use state::State;

struct Handler {
    state: Arc<RwLock<State>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        println!("Parser is ready!");

        let ref command_manager = self.state.read().await.command_manager;
        let test_guild_id = GuildId::new(
            env::var("PARSER_TEST_GUILD_ID")
                .unwrap()
                .parse::<u64>()
                .unwrap(),
        );
        test_guild_id
            .set_commands(&ctx.http, command_manager.create_commands())
            .await
            .expect("Failed to register commands");
    }
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(command) => {
                let ref command_manager = self.state.read().await.command_manager;
                command_manager
                    .process_command(ctx.clone(), command.clone())
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
    let _ = dotenv().map_err(|err| {
        println!("Can't find .env file!: {err:?}, using available environment variables.")
    });

    let token = env::var("PARSER_DISCORD_TOKEN").expect("Expected a token in the environment");

    let _ = env::var("PARSER_TEST_GUILD_ID")
        .expect("Expected a discord server id to test")
        .parse::<u64>()
        .expect("Expected discord server id to be integer type");

    let intents = GatewayIntents::non_privileged();

    let state = State::new();

    let mut client = Client::builder(&token, intents)
        .voice_manager_arc(state.voice_manager.clone())
        .event_handler(Handler {
            state: Arc::new(RwLock::new(state)),
        })
        .await
        .expect("Error creating client");

    let shard_manager = client.shard_manager.clone();

    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Could not register ctrl+c handler");
        println!("Shutting down...");
        shard_manager.shutdown_all().await;
    });

    if let Err(why) = client.start().await {
        eprintln!("Client error: {why:?}");
    }
}
