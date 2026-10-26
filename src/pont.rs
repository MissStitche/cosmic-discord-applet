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
    pub authentifie: bool,
    /// Mon identifiant Discord (pour reconnaître « (toi) » dans la liste).
    pub moi: String,
    pub salon: Option<Salon>,
    pub participants: Vec<Personne>,
    pub micro_coupe: bool,
    pub sourdine: bool,
    pub volume_micro: f32,
    pub ping: f64,
    /// Heure d'arrivée dans le salon (secondes depuis 1970).
    pub arrivee: i64,
    pub favoris: Vec<Salon>,
    pub recents: Vec<Salon>,
}

#[derive(Clone, Debug)]
pub struct Salon {
    pub id: String,
    pub nom: String,
    pub serveur_id: String,
    pub serveur: String,
    pub en_ligne: u64,
}

#[derive(Clone, Debug)]
pub struct Personne {
    pub id: String,
    pub nom: String,
    pub avatar: String,
    pub volume: f32,
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

fn lire_salon(json: &Value) -> Salon {
    Salon {
        id: texte(&json["id"]),
        nom: texte(&json["name"]),
        serveur_id: texte(&json["guild_id"]),
        serveur: texte(&json["guild_name"]),
        en_ligne: json["participant_count"].as_u64().unwrap_or(0),
    }
}

fn lire_salons(json: &Value) -> Vec<Salon> {
    json.as_array().into_iter().flatten().map(lire_salon).collect()
}

fn lire_snapshot(ligne: &str) -> Option<Etat> {
    let json: Value = serde_json::from_str(ligne).ok()?;
    if json["type"] != "snapshot" {
        return None;
    }
    let participants = json["participants"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|p| {
            let vrai = |cle: &str| p[cle].as_bool().unwrap_or(false);
            Personne {
                id: texte(&p["id"]),
                nom: Some(texte(&p["nick"])).filter(|n| !n.is_empty()).unwrap_or(texte(&p["username"])),
                avatar: texte(&p["avatar_path"]),
                volume: p["volume"].as_f64().unwrap_or(100.0) as f32,
                parle: vrai("speaking"),
                muet: vrai("mute") || vrai("self_mute"),
                sourd: vrai("deaf") || vrai("self_deaf"),
            }
        })
        .collect();
    let voix = &json["voice"];
    Some(Etat {
        status: texte(&json["status"]),
        authentifie: json["authenticated"].as_bool().unwrap_or(false),
        moi: texte(&json["user"]["id"]),
        salon: json["channel"].is_object().then(|| lire_salon(&json["channel"])),
        participants,
        micro_coupe: voix["mute"].as_bool().unwrap_or(false),
        sourdine: voix["deaf"].as_bool().unwrap_or(false),
        volume_micro: voix["input_volume"].as_f64().unwrap_or(100.0) as f32,
        ping: json["connection"]["average_ping"].as_f64().unwrap_or(0.0),
        arrivee: json["connected_since"].as_i64().unwrap_or(0),
        favoris: lire_salons(&json["favorite_channels"]),
        recents: lire_salons(&json["recent_channels"]),
    })
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
pub async fn commande(arguments: Vec<String>) {
    let _ = Command::new(chemin_pont()).arg("command").args(arguments).status().await;
}

/// Lance un programme sans attendre (ouvrir un lien, lancer Discord).
pub fn lancer(programme: &str, arguments: &[&str]) {
    if let Ok(mut enfant) = std::process::Command::new(programme).args(arguments).spawn() {
        std::thread::spawn(move || enfant.wait()); // évite un processus zombie
    }
}
