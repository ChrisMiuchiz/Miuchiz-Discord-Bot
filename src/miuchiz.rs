use serenity::model::channel::Message;
use serenity::model::gateway::Activity;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time;

const ONLINE_PLAYERS_CHANNEL: ChannelId = ChannelId(1011782827848777970);
const NOTIFICATIONS_CHANNEL: ChannelId = ChannelId(597636044464193577);

#[derive(Debug)]
struct PlayerInfo {
    last_active_time: time::SystemTime,
}

impl PlayerInfo {
    pub fn new() -> Self {
        Self {
            last_active_time: time::SystemTime::now(),
        }
    }

    pub fn time_since_activity(&self) -> Result<time::Duration, time::SystemTimeError> {
        time::SystemTime::now().duration_since(self.last_active_time)
    }

    pub fn update_time(&mut self) {
        self.last_active_time = time::SystemTime::now();
    }

    pub fn should_remove(&self) -> bool {
        if let Ok(since) = self.time_since_activity() {
            since > time::Duration::from_secs(15 * 60)
        } else {
            true
        }
    }
}

pub struct MiuchizBot {
    players: HashMap<String, PlayerInfo>,
    user_file: PathBuf,
    ctx: Context,
    banner: Message,
}

impl MiuchizBot {
    pub async fn new(user_file: PathBuf, ctx: Context) -> Self {
        let banner = Self::send_initial_message(&ctx).await;

        Self {
            players: HashMap::new(),
            user_file,
            ctx,
            banner,
        }
    }

    pub async fn send_initial_message(ctx: &Context) -> Message {
        loop {
            let message = ONLINE_PLAYERS_CHANNEL
                .send_message(&ctx, |m| m.content("..."))
                .await;
            match message {
                Ok(message) => return message,
                Err(why) => eprintln!("Error sending message: {:?}", why),
            }
        }
    }

    pub async fn main_loop(&mut self) {
        loop {
            let online_players = self.get_online_player_names();
            let new_players = self.update_player_infos(&online_players);

            self.send_online_notification(&online_players).await;
            self.send_login_notifications(&new_players).await;
            self.update_status(online_players.len()).await;

            tokio::time::sleep(time::Duration::from_secs(5)).await;
        }
    }

    pub fn update_player_infos(&mut self, online_players: &[String]) -> Vec<String> {
        let mut new_players = Vec::<String>::new();

        for online_player in online_players {
            if let Some(info) = self.players.get_mut(online_player) {
                // Update last active time if this player is already known
                info.update_time();
            } else {
                // Store new player info if not already known
                self.players
                    .insert(online_player.clone(), PlayerInfo::new());
                new_players.push(online_player.clone());
            }
        }

        self.players.retain(|_, info| !info.should_remove());

        new_players
    }

    pub fn get_online_player_names(&self) -> Vec<String> {
        if let Ok(txt) = std::fs::read_to_string(&self.user_file) {
            txt.lines()
                .filter(|x| !x.is_empty())
                .map(|x| x.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub async fn send_login_notifications(&self, new_players: &[String]) {
        for p in new_players {
            let text = format!("{p} has entered Planet Mion!");

            if let Err(why) = NOTIFICATIONS_CHANNEL
                .send_message(&self.ctx, |m| m.content(&text))
                .await
            {
                eprintln!("Error sending message: {:?}", why);
            }
        }
    }

    pub async fn send_online_notification(&mut self, online_players: &[String]) {
        // If there are no players, it looks better to put a single space
        let player_list_text = match online_players.is_empty() {
            true => " ".to_string(),
            false => online_players.join("\n"),
        };

        let banner_text = format!("Online players:\n```\n{}\n```", player_list_text);

        if let Err(why) = self
            .banner
            .edit(&self.ctx, |x| x.content(&banner_text))
            .await
        {
            eprintln!("Failed to update banner {why:?}");
        }
    }

    pub async fn update_status(&self, online_count: usize) {
        let status = if online_count > 0 {
            let plural = if online_count > 1 { "s" } else { "" };
            format!("{online_count} player{plural} ingame")
        } else {
            "No players ingame 😢".to_string()
        };

        self.ctx.set_activity(Activity::playing(&status)).await;
    }
}
