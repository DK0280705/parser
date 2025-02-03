use crate::state::{State, VoiceHandler};
use serenity::all::{
    ChannelType, CommandInteraction, CommandOptionType, Context, CreateCommand,
    CreateCommandOption, ResolvedValue
};
use songbird::CoreEvent;

use super::reply;

pub async fn run(
    state: &State,
    ctx: &Context,
    itr: &CommandInteraction,
) -> Result<(), serenity::Error> {
    let opts = itr.data.options();
    let first_opt = opts.first().unwrap();
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird voice client placed in at initialization")
        .clone();

    let guild_id = itr.guild_id.unwrap();

    match first_opt.name {
        "start" => {
            if manager.get(guild_id).is_some() {
                return reply(ctx, itr, "You need to stop the recording first").await;
            }

            let ResolvedValue::SubCommand(ref sub_command) = first_opt.value else {
                panic!("Boom");
            };
            let ResolvedValue::Channel(channel) = sub_command.first().unwrap().value else {
                panic!("Boom");
            };
            assert_eq!(channel.kind, ChannelType::Voice);

            let Ok(call_lock) = manager.join(guild_id, channel.id).await else {
                return reply(ctx, itr, "Can't connect to voice channel!").await;
            };

            {
                let mut call = call_lock.lock().await;
                let vc_entry = state
                    .record_channels
                    .entry(guild_id)
                    .insert_entry(VoiceHandler::new(format!("{}.pcm", channel.id)));
                call.add_global_event(CoreEvent::VoiceTick.into(), vc_entry.get().clone());
            }

            reply(ctx, itr, "Recording started!").await
        }
        "stop" => {
            if manager.get(guild_id).is_none() {
                return reply(ctx, itr, "Already stopped").await;
            }

            manager
                .remove(guild_id)
                .await
                .expect("The songbird is doomed");

            state.record_channels.remove(&guild_id);
            reply(ctx, itr, "Recording stopped").await
        }
        _ => unreachable!("Ain't no way unless i'm dumb"),
    }
}

pub fn create() -> CreateCommand {
    CreateCommand::new("record")
        .description("Join a voice channel")
        .add_option(
            CreateCommandOption::new(CommandOptionType::SubCommand, "start", "Start recording")
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "Channel to join",
                    )
                    .channel_types(vec![ChannelType::Voice])
                    .required(true),
                ),
        )
        .add_option(CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "stop",
            "Stop recording",
        ))
}
