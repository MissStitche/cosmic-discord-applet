//! COSMIC applet for the Discord widget: a panel icon that opens a popup.

mod bridge;
mod view;

use bridge::State;
use cosmic::app::{Core, Task};
use cosmic::iced::window::Id;
use cosmic::iced::{Length, Rectangle, Subscription};
use cosmic::surface::action::{app_popup, destroy_popup};
use cosmic::widget::{container, image};
use cosmic::Element;
use std::collections::HashMap;
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
    /// Avatars for the panel, by file path: (color, grayscale).
    avatars: HashMap<String, (image::Handle, image::Handle)>,
    /// People who just stopped speaking, still shown in color for a moment: id → hold generation.
    speaking_hold: HashMap<String, u64>,
    /// Who was speaking in the previous bridge state.
    was_speaking: Vec<String>,
    /// Bumped on every new hold, so an old hold timer can't end a newer one.
    hold_generation: u64,
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
    LaunchDiscord,
    Tick,
    SpeakingHoldEnded(String, u64),
    None,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

/// Most people in a call shown on the panel; the rest becomes "+N".
const MAX_PANEL_AVATARS: usize = 5;

/// Discord logo for the panel when not in a call (tinted like the other panel icons).
const DISCORD_ICON: &[u8] = include_bytes!("../assets/discord-symbolic.svg");

/// Loads an avatar in color and in grayscale (grayscale = not speaking).
fn load_avatar(path: &str) -> Option<(image::Handle, image::Handle)> {
    let color = ::image::open(path).ok()?.to_rgba8();
    let (width, height) = color.dimensions();
    let mut gray = color.clone();
    for pixel in gray.pixels_mut() {
        let [r, g, b, a] = pixel.0;
        let luma = (0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32) as u8;
        pixel.0 = [luma, luma, luma, a];
    }
    Some((
        image::Handle::from_rgba(width, height, color.into_raw()),
        image::Handle::from_rgba(width, height, gray.into_raw()),
    ))
}

impl Applet {
    fn load_avatars(&mut self) {
        let Some(state) = &self.state else { return };
        for p in &state.participants {
            if !p.avatar.is_empty() && !self.avatars.contains_key(&p.avatar) {
                if let Some(handles) = load_avatar(&p.avatar) {
                    self.avatars.insert(p.avatar.clone(), handles);
                }
            }
        }
    }

    /// Someone who stops speaking stays in color a moment, otherwise they would flicker between words.
    fn update_speaking(&mut self) -> Task<Message> {
        let speaking: Vec<String> = self
            .state
            .iter()
            .flat_map(|s| s.participants.iter())
            .filter(|p| p.speaking)
            .map(|p| p.id.clone())
            .collect();
        let mut tasks = Vec::new();
        for id in &self.was_speaking {
            if !speaking.contains(id) {
                self.hold_generation += 1;
                let (id, generation) = (id.clone(), self.hold_generation);
                self.speaking_hold.insert(id.clone(), generation);
                tasks.push(cosmic::task::future(async move {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    Message::SpeakingHoldEnded(id, generation)
                }));
            }
        }
        for id in &speaking {
            self.speaking_hold.remove(id);
        }
        self.was_speaking = speaking;
        Task::batch(tasks)
    }

    /// Panel button: Discord logo out of a call; in a call, everyone's avatar,
    /// gray when silent and in color with an accent ring while speaking.
    fn panel_button(&self) -> cosmic::widget::Button<'_, Message> {
        use cosmic::iced::widget::{Column, Row};
        use cosmic::iced::{Alignment, Border, Color};
        use cosmic::widget::{icon, image, text};
        let size = self.core.applet.suggested_size(true).0 as f32;
        let call = self.state.as_ref().filter(|s| s.channel.is_some());
        let Some(state) = call.filter(|s| !s.participants.is_empty()) else {
            let logo = icon::from_svg_bytes(DISCORD_ICON)
                .symbolic(true)
                .icon()
                .class(cosmic::theme::Svg::custom(|t| cosmic::iced::widget::svg::Style {
                    color: Some(Color::from(t.cosmic().background(false).on)),
                }))
                .width(Length::Fixed(size))
                .height(Length::Fixed(size));
            return self.core.applet.button_from_element(logo, true);
        };

        let inner = size - 4.0;
        let mut avatars: Vec<Element<'_, Message>> = Vec::new();
        for p in state.participants.iter().take(MAX_PANEL_AVATARS) {
            let colored = p.speaking || self.speaking_hold.contains_key(&p.id);
            let picture: Element<'_, Message> = match self.avatars.get(&p.avatar) {
                Some((color, gray)) => image(if colored { color.clone() } else { gray.clone() })
                    .width(Length::Fixed(inner))
                    .height(Length::Fixed(inner))
                    .border_radius(inner / 2.0)
                    .into(),
                None => icon::from_name("avatar-default-symbolic").size(inner as u16).into(),
            };
            avatars.push(container(picture).padding(2).class(cosmic::theme::Container::custom(
                move |t| container::Style {
                    border: Border {
                        color: if colored { t.cosmic().accent_color().into() } else { Color::TRANSPARENT },
                        width: 2.0,
                        radius: (size / 2.0).into(),
                    },
                    ..Default::default()
                },
            )).into());
        }
        let hidden = state.participants.len().saturating_sub(MAX_PANEL_AVATARS);
        if hidden > 0 {
            avatars.push(text::caption(format!("+{hidden}")).into());
        }
        // Like libcosmic's button_from_element, but grows with the number of avatars
        let applet = &self.core.applet;
        let (major_padding, minor_padding) = applet.suggested_padding(true);
        let thickness = Length::Fixed(size + 2.0 * minor_padding as f32);
        let content: Element<'_, Message> = if applet.is_horizontal() {
            Row::with_children(avatars).spacing(4).align_y(Alignment::Center).into()
        } else {
            Column::with_children(avatars).spacing(4).align_x(Alignment::Center).into()
        };
        let button = cosmic::widget::button::custom(cosmic::widget::layer_container(content).center(Length::Shrink))
            .class(cosmic::theme::Button::AppletIcon);
        if applet.is_horizontal() {
            button.height(thickness).padding([0, major_padding])
        } else {
            button.width(thickness).padding([major_padding, 0])
        }
    }
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
                self.load_avatars();
                return self.update_speaking();
            }
            Message::SpeakingHoldEnded(id, generation) => {
                if self.speaking_hold.get(&id) == Some(&generation) {
                    self.speaking_hold.remove(&id);
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
            .panel_button()
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
