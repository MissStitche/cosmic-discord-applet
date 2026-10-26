//! COSMIC applet for the Discord widget: a panel icon that opens a popup.

mod bridge;
mod view;

use bridge::State;
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Length, Rectangle, Subscription};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget::container;
use cosmic::Element;
use std::time::Duration;

const ID: &str = "fr.chloe.CosmicDiscordApplet";

#[derive(Default)]
struct Applet {
    core: Core,
    popup: Option<Id>,
    /// Last state sent by the bridge (None until it says something).
    state: Option<State>,
    /// Volume slider being dragged: (participant id, "" = my mic; value).
    dragged_volume: Option<(String, f32)>,
    /// Slider released: keep showing its value until the next bridge state (otherwise it "jumps").
    volume_released: bool,
    /// Participant whose volume slider is shown (click on their name).
    expanded_participant: Option<String>,
    /// Current time (seconds since 1970), for the call duration.
    now: i64,
}

#[derive(Clone, Debug)]
enum Message {
    PopupClosed(Id),
    Surface(cosmic::surface::Action<Message>),
    Bridge(Option<State>),
    /// Command for the bridge, e.g. ["set_mute", "true"].
    Command(Vec<String>),
    ToggleParticipant(String),
    Volume(String, f32),
    VolumeReleased,
    OpenInDiscord,
    LaunchDiscord,
    Tick,
    None,
}

fn now() -> i64 {
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
        (Applet { core, now: now(), ..Default::default() }, Task::none())
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
            Message::Bridge(state) => {
                self.state = state;
                if self.volume_released {
                    self.dragged_volume = None;
                    self.volume_released = false;
                }
            }
            Message::Command(arguments) => {
                return cosmic::task::future(async move {
                    bridge::command(arguments).await;
                    Message::None
                });
            }
            Message::ToggleParticipant(id) => {
                self.expanded_participant =
                    if self.expanded_participant.as_ref() == Some(&id) { None } else { Some(id) };
            }
            Message::Volume(id, value) => {
                self.dragged_volume = Some((id, value));
                self.volume_released = false;
            }
            // Only send the volume on release (otherwise one command per pixel)
            Message::VolumeReleased => {
                if let Some((id, value)) = self.dragged_volume.clone() {
                    self.volume_released = true;
                    let value = (value.round() as i64).to_string();
                    let arguments = if id.is_empty() {
                        vec!["set_mic_volume".into(), value]
                    } else {
                        vec!["set_user_volume".into(), id, value]
                    };
                    return self.update(Message::Command(arguments));
                }
            }
            Message::OpenInDiscord => {
                if let Some(channel) = self.state.as_ref().and_then(|s| s.channel.as_ref()) {
                    let guild = if channel.guild_id.is_empty() { "@me" } else { &channel.guild_id };
                    bridge::spawn("xdg-open", &[&format!("discord://-/channels/{guild}/{}", channel.id)]);
                }
            }
            // Flatpak Discord, otherwise the regular install
            Message::LaunchDiscord => {
                bridge::spawn(
                    "sh",
                    &["-c", "if flatpak info com.discordapp.Discord >/dev/null 2>&1; then flatpak run com.discordapp.Discord; else discord; fi"],
                );
            }
            Message::Tick => self.now = now(),
            Message::None => {}
        }
        Task::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        let in_call = self.state.as_ref().is_some_and(|s| s.channel.is_some());
        // The call duration only ticks while the popup is open
        let clock = if in_call && self.popup.is_some() {
            cosmic::iced::time::every(Duration::from_secs(1)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };
        Subscription::batch([Subscription::run(bridge::listen), clock])
    }

    fn view(&self) -> Element<'_, Message> {
        let popup = self.popup;
        let btn = self
            .core
            .applet
            .icon_button("audio-headset-symbolic")
            .on_press_with_rectangle(move |offset, bounds| match popup {
                // Popup already open: a click closes it.
                Some(id) => Message::Surface(destroy_popup(id)),
                None => Message::Surface(app_popup::<Applet>(
                    |_| Default::default(),
                    move |state: &mut Applet| {
                        let new_id = Id::unique();
                        state.popup = Some(new_id);
                        state.now = now();
                        let mut settings = state.core.applet.get_popup_settings(
                            state.core.main_window_id().unwrap(),
                            new_id,
                            None,
                            None,
                            None,
                        );
                        // Anchor the popup under the icon.
                        settings.positioner.anchor_rect = Rectangle {
                            x: (bounds.x - offset.x) as i32,
                            y: (bounds.y - offset.y) as i32,
                            width: bounds.width as i32,
                            height: bounds.height as i32,
                        };
                        settings
                    },
                    Some(Box::new(|state: &Applet| {
                        let content = container(view::panel(state)).width(Length::Fill).padding(8);
                        Element::from(transparent_background(content)).map(cosmic::Action::App)
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

static POPUP_ID: std::sync::LazyLock<cosmic::widget::Id> =
    std::sync::LazyLock::new(|| cosmic::widget::Id::new("discord-popup"));

/// Like libcosmic's popup_container, but always slightly transparent
/// (libcosmic only does it when blur is enabled in the theme).
fn transparent_background<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    use cosmic::iced::{Border, Color, Limits};
    let background = container(content).class(cosmic::theme::Container::custom(|theme| {
        let cosmic = theme.cosmic();
        let palette = cosmic.background(false);
        let mut color: Color = palette.base.into();
        color.a = 0.8;
        container::Style {
            text_color: Some(palette.on.into()),
            icon_color: Some(palette.on.into()),
            background: Some(color.into()),
            border: Border {
                radius: cosmic.corner_radii.radius_m.into(),
                width: 1.0,
                color: palette.divider.into(),
            },
            ..Default::default()
        }
    }));
    cosmic::widget::autosize::autosize(background, POPUP_ID.clone())
        .limits(Limits::NONE.min_height(1.0).min_width(360.0).max_width(360.0).max_height(1000.0))
        .into()
}

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<Applet>(())
}
