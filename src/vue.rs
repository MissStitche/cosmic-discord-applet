//! Contenu de la fenêtre, copié du panneau Discord de Quickshell (onglet Vocal), en style COSMIC.

use crate::pont::{Etat, Personne, Salon};
use crate::{Applet, Message};
use cosmic::iced::widget::{Column, Row};
use cosmic::iced::{Alignment, Border, Length};
use cosmic::widget::{button, container, icon, image, scrollable, slider, text};
use cosmic::{theme, Element};

/// Une commande pour le pont, ex. commande(&["set_mute", "true"]).
fn commande(arguments: &[&str]) -> Message {
    Message::Commande(arguments.iter().map(|a| a.to_string()).collect())
}

fn carte<'a>(contenu: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(contenu)
        .class(theme::Container::Card)
        .padding(12)
        .width(Length::Fill)
        .into()
}

fn ligne<'a>() -> Row<'a, Message, cosmic::Theme> {
    Row::new().spacing(8).align_y(Alignment::Center)
}

/// Durée « m:ss » ou « h:mm:ss ».
fn duree(secondes: i64) -> String {
    let s = secondes.max(0);
    let (h, m, s) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

fn etat_personne(p: &Personne) -> &'static str {
    if p.sourd {
        "En sourdine"
    } else if p.muet {
        "Micro coupé"
    } else if p.parle {
        "Parle"
    } else {
        "Écoute"
    }
}

fn est_favori(etat: &Etat, id: &str) -> bool {
    etat.favoris.iter().any(|s| s.id == id)
}

fn bouton_favori<'a>(etat: &Etat, id: &str) -> Element<'a, Message> {
    let favori = est_favori(etat, id);
    let action = if favori { "unfavorite_channel" } else { "favorite_channel" };
    button::icon(icon::from_name(if favori { "starred-symbolic" } else { "non-starred-symbolic" }))
        .selected(favori)
        .on_press(commande(&[action, id]))
        .into()
}

pub fn panneau(applet: &Applet) -> Element<'_, Message> {
    let mut colonne = Column::new().spacing(12).push(text::title4("Discord vocal"));
    let Some(etat) = &applet.etat else {
        return colonne.push(carte(text::body("Pont Discord introuvable ou arrêté"))).into();
    };
    if !etat.authentifie {
        return colonne.push(non_connecte(etat)).into();
    }
    match &etat.salon {
        Some(salon) => {
            colonne = colonne
                .push(carte_salon(applet, etat, salon))
                .push(boutons_appel(etat))
                .push(text::heading(format!("Participants ({})", etat.participants.len())));
            let mut liste = Column::new().spacing(8);
            for personne in &etat.participants {
                liste = liste.push(carte_personne(applet, etat, personne));
            }
            // Au-delà de 5 personnes, la liste défile
            colonne.push(container(scrollable(liste)).max_height(5.0 * 72.0)).into()
        }
        None => colonne
            .push(text::body("Tu n'es dans aucun salon vocal.").class(theme::Text::Default))
            .push(salons_enregistres(etat))
            .into(),
    }
}

/// Discord fermé, pas encore autorisé ou en cours de connexion.
fn non_connecte(etat: &Etat) -> Element<'_, Message> {
    let lance = etat.status != "discord_unavailable" && etat.status != "starting";
    let (titre, detail) = if !lance {
        ("Discord n'est pas lancé", "Lance l'appli Discord (le site web ne suffit pas).")
    } else if etat.status == "authenticating" || etat.status == "connecting" {
        ("Connexion à Discord…", "")
    } else if etat.status == "authorizing" {
        ("Discord n'est pas encore autorisé", "Accepte la demande dans Discord.")
    } else {
        ("Discord n'est pas encore autorisé", "Clique sur « Autoriser », puis accepte la demande dans Discord.")
    };
    let mut textes = Column::new().spacing(2).width(Length::Fill).push(text::heading(titre));
    if !detail.is_empty() {
        textes = textes.push(text::caption(detail));
    }
    let mut contenu = Column::new()
        .spacing(10)
        .push(ligne().spacing(10).push(icon::from_name("dialog-warning-symbolic").size(24)).push(textes));
    if !lance {
        contenu = contenu.push(button::suggested("Lancer Discord").on_press(Message::LancerDiscord));
    } else {
        contenu = contenu.push(button::suggested("Autoriser Discord").on_press(commande(&["authorize"])));
    }
    carte(contenu)
}

/// Serveur ⭐ ↗, nom du salon, durée · ping · nombre de personnes.
fn carte_salon<'a>(applet: &Applet, etat: &'a Etat, salon: &'a Salon) -> Element<'a, Message> {
    let serveur = if salon.serveur.is_empty() { "Message privé" } else { &salon.serveur };
    let en_tete = ligne()
        .push(icon::from_name("network-server-symbolic").size(18))
        .push(text::heading(serveur).width(Length::Fill))
        .push(bouton_favori(etat, &salon.id))
        .push(button::icon(icon::from_name("send-to-symbolic")).on_press(Message::OuvrirDansDiscord));
    let nom = ligne()
        .push(icon::from_name("audio-speakers-symbolic").size(18))
        .push(text::title4(&salon.nom).class(theme::Text::Accent));
    let infos = ligne()
        .spacing(14)
        .push(ligne().spacing(4).push(icon::from_name("preferences-system-time-symbolic").size(15)).push(
            text::body(duree(applet.maintenant - etat.arrivee)),
        ))
        .push(
            ligne()
                .spacing(4)
                .push(icon::from_name("network-cellular-signal-excellent-symbolic").size(14))
                .push(text::body(format!("{} ms", etat.ping.round())).class(theme::Text::Accent)),
        )
        .push(
            ligne()
                .spacing(4)
                .push(icon::from_name("system-users-symbolic").size(15))
                .push(text::body(etat.participants.len().to_string())),
        );
    carte(Column::new().spacing(8).push(en_tete).push(nom).push(infos))
}

/// Muet / Sourdine / Raccrocher (bouton accentué = actif, comme dans Quickshell).
fn boutons_appel(etat: &Etat) -> Element<'_, Message> {
    let bascule = |actif: bool, oui: &'static str, non: &'static str, cmd: &'static str| {
        let b = if actif { button::suggested(oui) } else { button::standard(non) };
        b.width(Length::Fill).on_press(commande(&[cmd, if actif { "false" } else { "true" }]))
    };
    Row::new()
        .spacing(8)
        .push(bascule(etat.micro_coupe, "Micro coupé", "Couper le micro", "set_mute"))
        .push(bascule(etat.sourdine, "En sourdine", "Sourdine", "set_deaf"))
        .push(button::destructive("Raccrocher").width(Length::Fill).on_press(commande(&["hang_up"])))
        .into()
}

/// Avatar rond (cerclé d'accent quand la personne parle), nom, état, volume.
fn carte_personne<'a>(applet: &Applet, etat: &Etat, p: &'a Personne) -> Element<'a, Message> {
    let moi = p.id == etat.moi;
    let image_avatar: Element<'a, Message> = if p.avatar.is_empty() {
        container(icon::from_name("avatar-default-symbolic").size(24))
            .center(Length::Fixed(40.0))
            .into()
    } else {
        image(image::Handle::from_path(&p.avatar))
            .width(40)
            .height(40)
            .border_radius(20.0)
            .into()
    };
    let parle = p.parle;
    let avatar = container(image_avatar).padding(3).class(theme::Container::custom(move |t| {
        container::Style {
            border: Border {
                color: if parle { t.cosmic().accent_color().into() } else { cosmic::iced::Color::TRANSPARENT },
                width: 3.0,
                radius: 23.0.into(),
            },
            ..Default::default()
        }
    }));

    let nom = text::heading(if moi { format!("{} (toi)", p.nom) } else { p.nom.clone() })
        .width(Length::Fill)
        .class(if parle { theme::Text::Accent } else { theme::Text::Default });
    let en_tete = ligne().push(nom).push(text::caption(etat_personne(p)));

    // Mon micro (0-100 %) ou le volume auquel j'entends la personne (0-200 %)
    let cle = if moi { String::new() } else { p.id.clone() };
    let valeur = match &applet.volume_en_cours {
        Some((id, v)) if *id == cle => *v,
        _ if moi => etat.volume_micro,
        _ => p.volume,
    };
    let (max, nom_icone) = if moi {
        (100.0, if p.muet { "microphone-sensitivity-muted-symbolic" } else { "audio-input-microphone-symbolic" })
    } else {
        (200.0, if p.muet { "audio-volume-muted-symbolic" } else { "audio-volume-high-symbolic" })
    };
    let volume = ligne()
        .spacing(6)
        .push(icon::from_name(nom_icone).size(15))
        .push(slider(0.0..=max, valeur, move |v| Message::Volume(cle.clone(), v)).on_release(Message::VolumeRelache))
        .push(text::caption(format!("{} %", valeur.round())).width(Length::Fixed(42.0)).align_x(Alignment::End));

    carte(
        ligne()
            .spacing(10)
            .push(avatar)
            .push(Column::new().spacing(2).width(Length::Fill).push(en_tete).push(volume)),
    )
}

/// Hors salon : favoris d'abord, puis les récents qui ne sont pas déjà en favori. Clic = rejoindre.
fn salons_enregistres(etat: &Etat) -> Element<'_, Message> {
    let salons = etat
        .favoris
        .iter()
        .chain(etat.recents.iter().filter(|s| !est_favori(etat, &s.id)));
    let mut liste = Column::new().spacing(8);
    for salon in salons {
        let serveur = if salon.serveur.is_empty() { "Message privé" } else { &salon.serveur };
        let detail = if salon.en_ligne > 0 {
            format!("{serveur} · {} en ligne", salon.en_ligne)
        } else {
            serveur.to_string()
        };
        let contenu = ligne()
            .push(icon::from_name("audio-speakers-symbolic").size(17))
            .push(
                Column::new()
                    .width(Length::Fill)
                    .push(text::heading(&salon.nom))
                    .push(text::caption(detail)),
            )
            .push(bouton_favori(etat, &salon.id))
            .push(icon::from_name("go-next-symbolic").size(18));
        liste = liste.push(
            button::custom(contenu)
                .class(theme::Button::MenuItem)
                .padding([6, 10])
                .width(Length::Fill)
                .on_press(commande(&["join_channel", &salon.id])),
        );
    }
    container(scrollable(liste)).max_height(400.0).into()
}
