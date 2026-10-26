//! Discussion avec le pont Discord (`discord-bridge`, installé à côté de l'applet) :
//! on le lance, il écrit une ligne JSON « snapshot » à chaque changement.

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
pub struct Etat {
    pub status: String,
    pub salon: Option<Salon>,
    pub participants: Vec<Personne>,
}

#[derive(Clone, Debug)]
pub struct Salon {
    pub nom: String,
    pub serveur: String,
}

#[derive(Clone, Debug)]
pub struct Personne {
    pub nom: String,
    pub parle: bool,
    pub muet: bool,
    pub sourd: bool,
}

/// Le pont est rangé dans le même dossier que l'applet.
fn chemin_pont() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|d| d.join("discord-bridge")))
        .unwrap_or_else(|| "discord-bridge".into())
}

fn texte(valeur: &Value) -> String {
    valeur.as_str().unwrap_or_default().to_string()
}

fn lire_snapshot(ligne: &str) -> Option<Etat> {
    let json: Value = serde_json::from_str(ligne).ok()?;
    if json["type"] != "snapshot" {
        return None;
    }
    let salon = json["channel"].is_object().then(|| Salon {
        nom: texte(&json["channel"]["name"]),
        serveur: texte(&json["channel"]["guild_name"]),
    });
    let participants = json["participants"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let vrai = |cle: &str| p[cle].as_bool().unwrap_or(false);
            Personne {
                nom: Some(texte(&p["nick"])).filter(|n| !n.is_empty()).unwrap_or(texte(&p["username"])),
                parle: vrai("speaking"),
                muet: vrai("mute") || vrai("self_mute"),
                sourd: vrai("deaf") || vrai("self_deaf"),
            }
        })
        .collect();
    Some(Etat { status: texte(&json["status"]), salon, participants })
}

/// Fait tourner le pont et envoie chaque nouvel état à l'applet ; le relance s'il s'arrête.
pub fn ecouter() -> impl Stream<Item = Message> {
    stream::channel(50, |mut sortie: mpsc::Sender<Message>| async move {
        loop {
            let pont = Command::new(chemin_pont())
                .arg("daemon")
                .stdout(Stdio::piped())
                .kill_on_drop(true) // l'applet se ferme → le pont aussi
                .spawn();
            if let Ok(mut pont) = pont {
                if let Some(stdout) = pont.stdout.take() {
                    let mut lignes = BufReader::new(stdout).lines();
                    while let Ok(Some(ligne)) = lignes.next_line().await {
                        if let Some(etat) = lire_snapshot(&ligne) {
                            let _ = sortie.send(Message::Pont(Some(etat))).await;
                        }
                    }
                }
                let _ = pont.kill().await;
            }
            let _ = sortie.send(Message::Pont(None)).await;
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    })
}

/// Envoie une commande au pont (comme les boutons du widget Quickshell).
pub async fn commande(arguments: &[&str]) {
    let _ = Command::new(chemin_pont()).arg("command").args(arguments).status().await;
}
