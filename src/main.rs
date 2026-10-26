//! Applet COSMIC du widget Discord : une icône dans le panneau qui ouvre une fenêtre.

mod pont;

use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Rectangle, Subscription};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::iced::widget::{Column, Row};
use cosmic::widget::{button, icon, text};
use cosmic::Element;
use pont::Etat;

const ID: &str = "fr.chloe.CosmicDiscordApplet";

#[derive(Default)]
struct Applet {
    core: Core,
    popup: Option<Id>,
    /// Dernier état envoyé par le pont (None tant qu'il n'a rien dit).
    etat: Option<Etat>,
}

#[derive(Clone, Debug)]
enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action<Message>),
    Pont(Option<Etat>),
    Autoriser,
    Rien,
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
        (Applet { core, ..Default::default() }, Task::none())
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
            Message::Pont(etat) => self.etat = etat,
            Message::Autoriser => {
                return cosmic::task::future(async {
                    pont::commande(&["authorize"]).await;
                    Message::Rien
                });
            }
            Message::Rien => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(pont::ecouter)
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
                        Element::from(state.core.applet.popup_container(state.contenu()))
                            .map(cosmic::Action::App)
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

impl Applet {
    /// Contenu de la fenêtre : un message d'état, ou le salon et ses participants.
    fn contenu(&self) -> Element<'_, Message> {
        let Some(etat) = &self.etat else {
            return text::body("Pont Discord introuvable ou arrêté").into();
        };
        let message = match etat.status.as_str() {
            "ready" => None,
            "discord_unavailable" => Some("Discord n'est pas lancé"),
            "auth_required" => Some("Autorise l'applet à lire ton vocal Discord"),
            "authorizing" => Some("Accepte la demande dans Discord…"),
            _ => Some("Connexion à Discord…"),
        };
        if let Some(message) = message {
            let mut colonne = Column::new().spacing(12).push(text::body(message));
            if etat.status == "auth_required" {
                colonne = colonne.push(button::suggested("Autoriser").on_press(Message::Autoriser));
            }
            return colonne.into();
        }
        let Some(salon) = &etat.salon else {
            return text::body("Tu n'es pas en vocal").into();
        };
        let mut colonne = Column::new()
            .spacing(8)
            .push(text::title4(&salon.nom))
            .push(text::caption(&salon.serveur));
        for personne in &etat.participants {
            // Icône : sourdine > muet > parle > rien
            let nom_icone = if personne.sourd {
                "audio-volume-muted-symbolic"
            } else if personne.muet {
                "microphone-sensitivity-muted-symbolic"
            } else if personne.parle {
                "audio-volume-high-symbolic"
            } else {
                "user-available-symbolic"
            };
            colonne = colonne.push(
                Row::new()
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .push(icon::from_name(nom_icone).size(16))
                    .push(text::body(&personne.nom)),
            );
        }
        colonne.into()
    }
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}
