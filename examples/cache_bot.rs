#[cfg(all(feature = "gateway", feature = "cache"))]
use async_trait::async_trait;
#[cfg(all(feature = "gateway", feature = "cache"))]
use discordrs::{gateway_intents, Client, Context, Event, EventHandler, Snowflake};

#[cfg(all(feature = "gateway", feature = "cache"))]
struct Handler;

#[cfg(all(feature = "gateway", feature = "cache"))]
#[async_trait]
impl EventHandler for Handler {
    async fn handle_event(&self, ctx: Context, event: Event) {
        match event {
            Event::Ready(ready) => {
                let cached_guilds = ctx.guilds().list_cached().await.len();
                println!(
                    "ready_user={} cached_guilds={cached_guilds}",
                    ready.data.user.username
                );
            }
            Event::MessageCreate(message) => {
                let fetched = ctx
                    .messages()
                    .get(
                        message.message.channel_id.clone(),
                        message.message.id.clone(),
                    )
                    .await;
                println!("cached_or_rest_message={}", fetched.is_ok());

                if let Some(guild_id) = message.message.guild_id {
                    let cached_member = ctx
                        .cache
                        .member(&guild_id, &Snowflake::from("0"))
                        .await
                        .is_some();
                    println!("cached_member_lookup={cached_member}");
                }
            }
            _ => {}
        }
    }
}

#[cfg(all(feature = "gateway", feature = "cache"))]
#[tokio::main]
async fn main() -> Result<(), discordrs::DiscordError> {
    let token = std::env::var("DISCORD_TOKEN")?;
    Client::builder(
        &token,
        gateway_intents::GUILDS | gateway_intents::GUILD_MESSAGES,
    )
    .event_handler(Handler)
    .start()
    .await
}

#[cfg(not(all(feature = "gateway", feature = "cache")))]
fn main() {}
