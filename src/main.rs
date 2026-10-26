//! Applet COSMIC du widget Discord : une icône dans le panneau qui ouvre une fenêtre.

mod pont;
mod vue;

use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Length, Rectangle, Subscription};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget::container;
use cosmic::Element;
use pont::Etat;
use std::time::Duration;

const ID: &str = "fr.chloe.CosmicDiscordApplet";

#[derive(Default)]
struct Applet {
    core: Core,
    popup: Option<Id>,
    /// Dernier état envoyé par le pont (None tant qu'il n'a rien dit).
    etat: Option<Etat>,
    /// Curseur de volume en cours de glissement : (id de la personne, "" = mon micro ; valeur).
    volume_en_cours: Option<(String, f32)>,
    /// Curseur lâché : on garde sa valeur affichée jusqu'au prochain état du pont (sinon il « saute »).
    volume_relache: bool,
    /// Personne dont on affiche le curseur de volume (clic sur son nom).
    personne_ouverte: Option<String>,
    /// Heure actuelle (secondes depuis 1970), pour la durée de l'appel.
    maintenant: i64,
}

#[derive(Clone, Debug)]
enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action<Message>),
    Pont(Option<Etat>),
    /// Commande pour le pont, ex. ["set_mute", "true"].
    Commande(Vec<String>),
    OuvrirPersonne(String),
    Volume(String, f32),
    VolumeRelache,
    OuvrirDansDiscord,
    LancerDiscord,
    Seconde,
    Rien,
}

fn maintenant() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl cosmic::Application for Applet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: ()) -> (Self, Task<Message>) {
        (Applet { core, maintenant: maintenant(), ..Default::default() }, Task::none())
    }

    fn on_close_requested(&self, id: Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::Surface(a) => return cosmic::task::message(cosmic::Action::Surface(a)),
            Message::Pont(etat) => {
                self.etat = etat;
                if self.volume_relache {
                    self.volume_en_cours = None;
                    self.volume_relache = false;
                }
            }
            Message::Commande(arguments) => {
                return cosmic::task::future(async move {
                    pont::commande(arguments).await;
                    Message::Rien
                });
            }
            Message::OuvrirPersonne(id) => {
                self.personne_ouverte = if self.personne_ouverte.as_ref() == Some(&id) { None } else { Some(id) };
            }
            Message::Volume(id, valeur) => {
                self.volume_en_cours = Some((id, valeur));
                self.volume_relache = false;
            }
            // On n'envoie le volume qu'au lâcher (sinon une commande à chaque pixel)
            Message::VolumeRelache => {
                if let Some((id, valeur)) = self.volume_en_cours.clone() {
                    self.volume_relache = true;
                    let valeur = (valeur.round() as i64).to_string();
                    let arguments = if id.is_empty() {
                        vec!["set_mic_volume".into(), valeur]
                    } else {
                        vec!["set_user_volume".into(), id, valeur]
                    };
                    return self.update(Message::Commande(arguments));
                }
            }
            Message::OuvrirDansDiscord => {
                if let Some(salon) = self.etat.as_ref().and_then(|e| e.salon.as_ref()) {
                    let serveur = if salon.serveur_id.is_empty() { "@me" } else { &salon.serveur_id };
                    pont::lancer("xdg-open", &[&format!("discord://-/channels/{serveur}/{}", salon.id)]);
                }
            }
            // Discord en Flatpak, sinon installé normalement
            Message::LancerDiscord => {
                pont::lancer(
                    "sh",
                    &["-c", "if flatpak info com.discordapp.Discord >/dev/null 2>&1; then flatpak run com.discordapp.Discord; else discord; fi"],
                );
            }
            Message::Seconde => self.maintenant = maintenant(),
            Message::Rien => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let en_appel = self.etat.as_ref().is_some_and(|e| e.salon.is_some());
        // La durée de l'appel n'avance que si la fenêtre est ouverte
        let horloge = if en_appel && self.popup.is_some() {
            cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::Seconde)
        } else {
            Subscription::none()
        };
        Subscription::batch([Subscription::run(pont::ecouter), horloge])
    }

    fn view(&self) -> Element<'_, Message> {
        let popup = self.popup;
        let btn = self
            .core
            .applet
            .icon_button("audio-headset-symbolic")
            .on_press_with_rectangle(move |offset, bounds| match popup {
                // Fenêtre déjà ouverte : un clic la referme.
                Some(id) => Message::Surface(destroy_popup(id)),
                None => Message::Surface(app_popup::<Applet>(
                    |_| Default::default(),
                    move |state: &mut Applet| {
                        let new_id = Id::unique();
                        state.popup = Some(new_id);
                        state.maintenant = maintenant();
                        let mut settings = state.core.applet.get_popup_settings(
                            state.core.main_window_id().unwrap(),
                            new_id,
                            None,
                            None,
                            None,
                        );
                        // Accroche la fenêtre sous l'icône.
                        settings.positioner.anchor_rect = Rectangle {
                            x: (bounds.x - offset.x) as i32,
                            y: (bounds.y - offset.y) as i32,
                            width: bounds.width as i32,
                            height: bounds.height as i32,
                        };
                        settings
                    },
                    Some(Box::new(|state: &Applet| {
                        let contenu = container(vue::panneau(state)).width(Length::Fill).padding(8);
                        Element::from(state.core.applet.popup_container(contenu)).map(cosmic::Action::App)
                    })),
                )),
            });

        Element::from(self.core.applet.applet_tooltip::<Message>(
            btn,
            "Discord",
            self.popup.is_some(),
            Message::Surface,
            None,
        ))
    }

    fn view_window(&self, _id: Id) -> Element<'_, Message> {
        "".into()
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}
