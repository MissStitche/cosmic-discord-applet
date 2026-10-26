//! Contenu de la fenêtre, minimaliste comme les applets COSMIC (ex. Voxtype) :
//! en-tête icône + titre + sous-titre, puis une liste à séparateurs.

use crate::pont::{Etat, Personne, Salon};
use crate::{Applet, Message};
use cosmic::iced::widget::{Column, Row};
use cosmic::iced::{Alignment, Border, Length};
use cosmic::widget::{button, container, icon, image, list, list_column, slider, text};
use cosmic::{theme, Element};

/// Une commande pour le pont, ex. commande(&["set_mute", "true"]).
fn commande(arguments: &[&str]) -> Message {
    Message::Commande(arguments.iter().map(|a| a.to_string()).collect())
}

fn ligne<'a>() -> Row<'a, Message, cosmic::Theme> {
    Row::new().spacing(8).align_y(Alignment::Center)
}

/// Liste à séparateurs, avec un fond semi-transparent (celui de COSMIC est opaque
/// et cacherait la transparence de la fenêtre).
fn liste<'a>() -> list::ListColumn<'a, Message> {
    list_column().style(theme::Container::custom(|t| {
        let cosmic = t.cosmic();
        let composant = &cosmic.background(false).component;
        let mut fond: cosmic::iced::Color = composant.base.into();
        fond.a = 0.5;
        container::Style {
            icon_color: Some(composant.on.into()),
            text_color: Some(composant.on.into()),
            background: Some(fond.into()),
            border: Border { radius: cosmic.corner_radii.radius_s.into(), ..Default::default() },
            ..Default::default()
        }
    }))
}

/// Durée « m:ss » ou « h:mm:ss ».
fn duree(secondes: i64) -> String {
    let s = secondes.max(0);
    let (h, m, s) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

fn nom_serveur(salon: &Salon) -> &str {
    if salon.serveur.is_empty() { "Message privé" } else { &salon.serveur }
}

/// En-tête : icône, titre en gras, sous-titre discret, et une action à droite.
fn en_tete<'a>(
    nom_icone: &'static str,
    accent: bool,
    titre: String,
    sous_titre: String,
    action: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut icone = icon::from_name(nom_icone).size(20).icon();
    if accent {
        icone = icone.class(theme::Svg::custom(|t| cosmic::iced::widget::svg::Style {
            color: Some(t.cosmic().accent_color().into()),
        }));
    }
    let mut rangee = ligne()
        .spacing(12)
        .padding([4, 8])
        .push(icone)
        .push(Column::new().width(Length::Fill).push(text::heading(titre)).push(text::caption(sous_titre)));
    if let Some(action) = action {
        rangee = rangee.push(action);
    }
    rangee.into()
}

pub fn panneau(applet: &Applet) -> Element<'_, Message> {
    let Some(etat) = &applet.etat else {
        return en_tete("audio-headset-symbolic", false, "Discord".into(), "Pont introuvable ou arrêté".into(), None);
    };
    if !etat.authentifie {
        return non_connecte(etat);
    }
    match &etat.salon {
        Some(salon) => en_appel(applet, etat, salon),
        None => hors_appel(etat),
    }
}

/// Discord fermé, pas encore autorisé ou en cours de connexion.
fn non_connecte(etat: &Etat) -> Element<'_, Message> {
    let lance = etat.status != "discord_unavailable" && etat.status != "starting";
    let (sous_titre, action) = if !lance {
        ("Discord n'est pas lancé", Some(button::suggested("Lancer").on_press(Message::LancerDiscord)))
    } else if etat.status == "authenticating" || etat.status == "connecting" {
        ("Connexion…", None)
    } else if etat.status == "authorizing" {
        ("Accepte la demande dans Discord", Some(button::standard("Réessayer").on_press(commande(&["authorize"]))))
    } else {
        ("Pas encore autorisé", Some(button::suggested("Autoriser").on_press(commande(&["authorize"]))))
    };
    en_tete("audio-headset-symbolic", false, "Discord".into(), sous_titre.into(), action.map(Into::into))
}

/// En appel : salon + infos, micro / casque, puis les participants.
fn en_appel<'a>(applet: &'a Applet, etat: &'a Etat, salon: &'a Salon) -> Element<'a, Message> {
    let infos = format!(
        "{} · {} · {} ms",
        nom_serveur(salon),
        duree(applet.maintenant - etat.arrivee),
        etat.ping.round()
    );
    let raccrocher = button::icon(icon::from_name("call-stop-symbolic"))
        .class(theme::Button::Destructive)
        .on_press(commande(&["hang_up"]));
    let ouvrir = button::icon(icon::from_name("send-to-symbolic")).on_press(Message::OuvrirDansDiscord);
    let actions = ligne().spacing(4).push(ouvrir).push(raccrocher);

    // Bouton accentué = coupé (comme dans Quickshell)
    let bascule = |coupe: bool, nom: &'static str, allume: &'static str, eteint: &'static str, cmd: &'static str| {
        let b = if coupe { button::suggested(nom) } else { button::standard(nom) };
        b.leading_icon(icon::from_name(if coupe { eteint } else { allume }))
            .width(Length::Fill)
            .on_press(commande(&[cmd, if coupe { "false" } else { "true" }]))
    };
    let micro_casque = Row::new()
        .spacing(8)
        .push(bascule(
            etat.micro_coupe,
            "Micro",
            "audio-input-microphone-symbolic",
            "microphone-sensitivity-muted-symbolic",
            "set_mute",
        ))
        .push(bascule(etat.sourdine, "Casque", "audio-headphones-symbolic", "audio-volume-muted-symbolic", "set_deaf"));

    let mut personnes = liste();
    for p in &etat.participants {
        personnes = personnes.add(ligne_personne(applet, etat, p));
        if applet.personne_ouverte.as_deref() == Some(p.id.as_str()) {
            personnes = personnes.add(ligne_volume(applet, etat, p));
        }
    }

    Column::new()
        .spacing(12)
        .push(en_tete("audio-headset-symbolic", true, salon.nom.clone(), infos, Some(actions.into())))
        .push(micro_casque)
        .push(container(cosmic::widget::scrollable(personnes)).max_height(6.0 * 52.0))
        .into()
}

/// Avatar rond (cerclé d'accent quand la personne parle), nom, icône d'état seulement si coupé.
fn ligne_personne<'a>(applet: &Applet, etat: &Etat, p: &'a Personne) -> list::ListButton<'a, Message> {
    let moi = p.id == etat.moi;
    let image_avatar: Element<'a, Message> = if p.avatar.is_empty() {
        container(icon::from_name("avatar-default-symbolic").size(18)).center(Length::Fixed(28.0)).into()
    } else {
        image(image::Handle::from_path(&p.avatar)).width(28).height(28).border_radius(14.0).into()
    };
    let parle = p.parle;
    let avatar = container(image_avatar).padding(2).class(theme::Container::custom(move |t| container::Style {
        border: Border {
            color: if parle { t.cosmic().accent_color().into() } else { cosmic::iced::Color::TRANSPARENT },
            width: 2.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }));
    let nom = text::body(if moi { format!("{} (toi)", p.nom) } else { p.nom.clone() })
        .width(Length::Fill)
        .class(if parle { theme::Text::Accent } else { theme::Text::Default });
    let mut rangee = ligne().spacing(10).push(avatar).push(nom);
    let etat_icone = if p.sourd {
        Some("audio-volume-muted-symbolic")
    } else if p.muet {
        Some("microphone-sensitivity-muted-symbolic")
    } else {
        None
    };
    if let Some(nom_icone) = etat_icone {
        rangee = rangee.push(icon::from_name(nom_icone).size(16));
    }
    // Clic sur quelqu'un : affiche / cache son volume
    let ouverte = applet.personne_ouverte.as_deref() == Some(p.id.as_str());
    list::button(rangee).selected(ouverte).on_press(Message::OuvrirPersonne(p.id.clone()))
}

/// Volume sous la personne : mon micro (0-100 %) ou le volume auquel je l'entends (0-200 %).
fn ligne_volume<'a>(applet: &Applet, etat: &Etat, p: &Personne) -> Element<'a, Message> {
    let moi = p.id == etat.moi;
    let cle = if moi { String::new() } else { p.id.clone() };
    let valeur = match &applet.volume_en_cours {
        Some((id, v)) if *id == cle => *v,
        _ if moi => etat.volume_micro,
        _ => p.volume,
    };
    let max = if moi { 100.0 } else { 200.0 };
    ligne()
        .padding([0, 4])
        .push(slider(0.0..=max, valeur, move |v| Message::Volume(cle.clone(), v)).on_release(Message::VolumeRelache))
        .push(text::caption(format!("{} %", valeur.round())).width(Length::Fixed(42.0)).align_x(Alignment::End))
        .into()
}

/// Hors appel : favoris puis récents, clic = rejoindre.
fn hors_appel(etat: &Etat) -> Element<'_, Message> {
    let est_favori = |id: &str| etat.favoris.iter().any(|s| s.id == id);
    let salons: Vec<&Salon> = etat
        .favoris
        .iter()
        .chain(etat.recents.iter().filter(|s| !est_favori(&s.id)))
        .collect();
    let mut colonne = Column::new()
        .spacing(12)
        .push(en_tete("audio-headset-symbolic", false, "Discord".into(), "Pas en vocal".into(), None));
    if salons.is_empty() {
        return colonne.into();
    }
    let mut salons_liste = liste();
    for salon in salons {
        let detail = if salon.en_ligne > 0 {
            format!("{} · {} en ligne", nom_serveur(salon), salon.en_ligne)
        } else {
            nom_serveur(salon).to_string()
        };
        let favori = est_favori(&salon.id);
        let action = if favori { "unfavorite_channel" } else { "favorite_channel" };
        let etoile = button::icon(icon::from_name(if favori { "starred-symbolic" } else { "non-starred-symbolic" }))
            .selected(favori)
            .on_press(commande(&[action, &salon.id]));
        let rangee = ligne()
            .push(Column::new().width(Length::Fill).push(text::body(&salon.nom)).push(text::caption(detail)))
            .push(etoile);
        salons_liste = salons_liste.add(list::button(rangee).on_press(commande(&["join_channel", &salon.id])));
    }
    colonne = colonne.push(container(cosmic::widget::scrollable(salons_liste)).max_height(400.0));
    colonne.into()
}
