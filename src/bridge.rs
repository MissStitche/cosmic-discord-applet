//! Talks to the Discord bridge (`discord-bridge`, installed next to the applet):
//! we start it, and it prints a JSON "snapshot" line on every change.

use crate::Message;
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::futures::{SinkExt, Stream};
use cosmic::iced::stream;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Clone, Debug, Default)]
pub struct State {
    pub status: String,
    pub authenticated: bool,
    /// My Discord id (to show "(toi)" in the list).
    pub self_id: String,
    pub channel: Option<Channel>,
    pub participants: Vec<Participant>,
    pub mic_muted: bool,
    pub deafened: bool,
    pub mic_volume: f32,
    pub ping: f64,
    /// When I joined the channel (seconds since 1970).
    pub joined_at: i64,
    pub favorites: Vec<Channel>,
    pub recents: Vec<Channel>,
}

#[derive(Clone, Debug)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub guild_name: String,
    pub online_count: u64,
}

#[derive(Clone, Debug)]
pub struct Participant {
    pub id: String,
    pub name: String,
    pub avatar: String,
    pub volume: f32,
    pub speaking: bool,
    pub muted: bool,
    pub deafened: bool,
}

/// The bridge lives in the same folder as the applet.
fn bridge_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("discord-bridge")))
        .unwrap_or_else(|| "discord-bridge".into())
}

fn string(value: &Value) -> String {
    value.as_str().unwrap_or_default().to_string()
}

fn parse_channel(json: &Value) -> Channel {
    Channel {
        id: string(&json["id"]),
        name: string(&json["name"]),
        guild_name: string(&json["guild_name"]),
        online_count: json["participant_count"].as_u64().unwrap_or(0),
    }
}

fn parse_channels(json: &Value) -> Vec<Channel> {
    json.as_array().into_iter().flatten().map(parse_channel).collect()
}

fn parse_snapshot(line: &str) -> Option<State> {
    let json: Value = serde_json::from_str(line).ok()?;
    if json["type"] != "snapshot" {
        return None;
    }
    let participants = json["participants"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let flag = |key: &str| p[key].as_bool().unwrap_or(false);
            Participant {
                id: string(&p["id"]),
                name: Some(string(&p["nick"])).filter(|n| !n.is_empty()).unwrap_or(string(&p["username"])),
                avatar: string(&p["avatar_path"]),
                volume: p["volume"].as_f64().unwrap_or(100.0) as f32,
                speaking: flag("speaking"),
                muted: flag("mute") || flag("self_mute"),
                deafened: flag("deaf") || flag("self_deaf"),
            }
        })
        .collect();
    let voice = &json["voice"];
    Some(State {
        status: string(&json["status"]),
        authenticated: json["authenticated"].as_bool().unwrap_or(false),
        self_id: string(&json["user"]["id"]),
        channel: json["channel"].is_object().then(|| parse_channel(&json["channel"])),
        participants,
        mic_muted: voice["mute"].as_bool().unwrap_or(false),
        deafened: voice["deaf"].as_bool().unwrap_or(false),
        mic_volume: voice["input_volume"].as_f64().unwrap_or(100.0) as f32,
        ping: json["connection"]["average_ping"].as_f64().unwrap_or(0.0),
        joined_at: json["connected_since"].as_i64().unwrap_or(0),
        favorites: parse_channels(&json["favorite_channels"]),
        recents: parse_channels(&json["recent_channels"]),
    })
}

/// Runs the bridge and sends each new state to the applet; restarts it if it stops.
pub fn listen() -> impl Stream<Item = Message> {
    stream::channel(50, |mut output: mpsc::Sender<Message>| async move {
        loop {
            let bridge = Command::new(bridge_path())
                .arg("daemon")
                .stdout(Stdio::piped())
                .kill_on_drop(true) // applet closes → bridge too
                .spawn();
            if let Ok(mut bridge) = bridge {
                if let Some(stdout) = bridge.stdout.take() {
                    let mut lines = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = lines.next_line().await {
                        if let Some(state) = parse_snapshot(&line) {
                            let _ = output.send(Message::Bridge(Some(state))).await;
                        }
                    }
                }
                let _ = bridge.kill().await;
            }
            let _ = output.send(Message::Bridge(None)).await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    })
}

/// Sends a command to the bridge (like the Quickshell widget buttons).
pub async fn command(arguments: Vec<String>) {
    let _ = Command::new(bridge_path()).arg("command").args(arguments).status().await;
}

/// Starts a program without waiting (launch Discord).
pub fn spawn(program: &str, arguments: &[&str]) {
    if let Ok(mut child) = std::process::Command::new(program).args(arguments).spawn() {
        std::thread::spawn(move || child.wait()); // avoids a zombie process
    }
}
