use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use std::env;
use std::path::PathBuf;
mod miuchiz;

struct Handler {
    user_file: PathBuf,
    bot_dispatched: RwLock<bool>,
}

impl Handler {
    pub fn new(user_file: PathBuf) -> Self {
        Self {
            user_file,
            bot_dispatched: RwLock::new(false),
        }
    }

    async fn enter_bot(&self) -> bool {
        let mut bot_dispatched = self.bot_dispatched.write().await;

        if *bot_dispatched {
            return false;
        }

        *bot_dispatched = true;

        true
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        // Allow only one instance of the bot, the current function could be
        // called again if there is a connection issue.
        if !self.enter_bot().await {
            return;
        }

        let mut bot = miuchiz::MiuchizBot::new(self.user_file.clone(), ctx.clone()).await;
        println!("{} is connected!", ready.user.name);
        tokio::spawn(async move {
            bot.main_loop().await;
        });
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
    let user_file = env::var("MIUCHIZ_USERLIST_FILE")
        .expect("Expected a miuchiz userlist file in the environment");

    let intents = GatewayIntents::empty();
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new(PathBuf::from(user_file)))
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}
