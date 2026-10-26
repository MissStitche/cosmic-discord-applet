//! Applet COSMIC du widget Discord : une icône dans le panneau qui ouvre une fenêtre.

use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::Rectangle;
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget::text;
use cosmic::Element;

const ID: &str = "fr.chloe.CosmicDiscordApplet";

#[derive(Default)]
struct Applet {
    core: Core,
    popup: Option<Id>,
}

#[derive(Clone, Debug)]
enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action<Message>),
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
        (Applet { core, popup: None }, Task::none())
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
        }
        Task::none()
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
                        Element::from(state.core.applet.popup_container(text::body(
                            "Coucou ! L'applet Discord fonctionne 🎧",
                        )))
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

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}
