//! Popup content, minimalist like COSMIC applets (e.g. Voxtype):
//! header with icon + title + subtitle, then a list with dividers.
//! Text shown to the user stays in French.

use crate::bridge::{Channel, Participant, State};
use crate::{Applet, Message};
use cosmic::iced::widget::{Column, Row};
use cosmic::iced::{Alignment, Border, Length};
use cosmic::widget::{button, container, icon, image, list, list_column, slider, text};
use cosmic::{theme, Element};

/// A command for the bridge, e.g. command(&["set_mute", "true"]).
fn command(arguments: &[&str]) -> Message {
    Message::Command(arguments.iter().map(|a| a.to_string()).collect())
}

fn row<'a>() -> Row<'a, Message, cosmic::Theme> {
    Row::new().spacing(8).align_y(Alignment::Center)
}

/// List with dividers and a semi-transparent background (COSMIC's is opaque
/// and would hide the popup transparency).
fn list<'a>() -> list::ListColumn<'a, Message> {
    list_column().style(theme::Container::custom(|t| {
        let cosmic = t.cosmic();
        let component = &cosmic.background(false).component;
        let mut background: cosmic::iced::Color = component.base.into();
        background.a = 0.5;
        container::Style {
            icon_color: Some(component.on.into()),
            text_color: Some(component.on.into()),
            background: Some(background.into()),
            border: Border { radius: cosmic.corner_radii.radius_s.into(), ..Default::default() },
            ..Default::default()
        }
    }))
}

/// Duration as "m:ss" or "h:mm:ss".
fn duration(seconds: i64) -> String {
    let s = seconds.max(0);
    let (h, m, s) = (s / 3600, (s % 3600) / 60, s % 60);
    if h > 0 { format!("{h}:{m:02}:{s:02}") } else { format!("{m}:{s:02}") }
}

fn guild_name(channel: &Channel) -> &str {
    if channel.guild_name.is_empty() { "Message privé" } else { &channel.guild_name }
}

/// Header: icon, bold title, muted subtitle, and an action on the right.
fn header<'a>(
    icon_name: &'static str,
    accent: bool,
    title: String,
    subtitle: String,
    action: Option<Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut header_icon = icon::from_name(icon_name).size(20).icon();
    if accent {
        header_icon = header_icon.class(theme::Svg::custom(|t| cosmic::iced::widget::svg::Style {
            color: Some(t.cosmic().accent_color().into()),
        }));
    }
    let mut header_row = row()
        .spacing(12)
        .padding([4, 8])
        .push(header_icon)
        .push(Column::new().width(Length::Fill).push(text::heading(title)).push(text::caption(subtitle)));
    if let Some(action) = action {
        header_row = header_row.push(action);
    }
    header_row.into()
}

pub fn panel(applet: &Applet) -> Element<'_, Message> {
    let Some(state) = &applet.state else {
        return header("audio-headset-symbolic", false, "Discord".into(), "Pont introuvable ou arrêté".into(), None);
    };
    if !state.authenticated {
        return not_connected(state);
    }
    match &state.channel {
        Some(channel) => in_call(applet, state, channel),
        None => out_of_call(state),
    }
}

/// Discord closed, not authorized yet, or connecting.
fn not_connected(state: &State) -> Element<'_, Message> {
    let running = state.status != "discord_unavailable" && state.status != "starting";
    let (subtitle, action) = if !running {
        ("Discord n'est pas lancé", Some(button::suggested("Lancer").on_press(Message::LaunchDiscord)))
    } else if state.status == "authenticating" || state.status == "connecting" {
        ("Connexion…", None)
    } else if state.status == "authorizing" {
        ("Accepte la demande dans Discord", Some(button::standard("Réessayer").on_press(command(&["authorize"]))))
    } else {
        ("Pas encore autorisé", Some(button::suggested("Autoriser").on_press(command(&["authorize"]))))
    };
    header("audio-headset-symbolic", false, "Discord".into(), subtitle.into(), action.map(Into::into))
}

/// In a call: channel + info, mic / headset, then participants.
fn in_call<'a>(applet: &'a Applet, state: &'a State, channel: &'a Channel) -> Element<'a, Message> {
    let info = format!(
        "{} · {} · {} ms",
        guild_name(channel),
        duration(applet.now - state.joined_at),
        state.ping.round()
    );
    let hang_up = button::icon(icon::from_name("call-stop-symbolic"))
        .class(theme::Button::Destructive)
        .on_press(command(&["hang_up"]));

    // Highlighted button = muted (like in Quickshell)
    let toggle = |muted: bool, label: &'static str, on_icon: &'static str, off_icon: &'static str, cmd: &'static str| {
        let b = if muted { button::suggested(label) } else { button::standard(label) };
        b.leading_icon(icon::from_name(if muted { off_icon } else { on_icon }))
            .on_press(command(&[cmd, if muted { "false" } else { "true" }]))
    };
    let mic_and_headset = Row::new()
        .spacing(8)
        .push(toggle(
            state.mic_muted,
            "Micro",
            "audio-input-microphone-symbolic",
            "microphone-sensitivity-muted-symbolic",
            "set_mute",
        ))
        .push(toggle(state.deafened, "Casque", "audio-headphones-symbolic", "audio-volume-muted-symbolic", "set_deaf"));
    let mic_and_headset = container(mic_and_headset).center_x(Length::Fill);

    let mut participants = list();
    for p in &state.participants {
        participants = participants.add(participant_row(applet, state, p));
        if applet.expanded_participant.as_deref() == Some(p.id.as_str()) {
            participants = participants.add(volume_row(applet, state, p));
        }
    }

    Column::new()
        .spacing(12)
        .push(header("audio-headset-symbolic", true, channel.name.clone(), info, Some(hang_up.into())))
        .push(mic_and_headset)
        .push(container(cosmic::widget::scrollable(participants)).max_height(6.0 * 52.0))
        .into()
}

/// Round avatar (accent ring while speaking), name, status icon only when muted.
fn participant_row<'a>(applet: &Applet, state: &State, p: &'a Participant) -> list::ListButton<'a, Message> {
    let is_self = p.id == state.self_id;
    let avatar_image: Element<'a, Message> = if p.avatar.is_empty() {
        container(icon::from_name("avatar-default-symbolic").size(18)).center(Length::Fixed(28.0)).into()
    } else {
        image(image::Handle::from_path(&p.avatar)).width(28).height(28).border_radius(14.0).into()
    };
    let speaking = p.speaking;
    let avatar = container(avatar_image).padding(2).class(theme::Container::custom(move |t| container::Style {
        border: Border {
            color: if speaking { t.cosmic().accent_color().into() } else { cosmic::iced::Color::TRANSPARENT },
            width: 2.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }));
    let name = text::body(if is_self { format!("{} (toi)", p.name) } else { p.name.clone() })
        .width(Length::Fill)
        .class(if speaking { theme::Text::Accent } else { theme::Text::Default });
    let mut participant = row().spacing(10).push(avatar).push(name);
    let status_icon = if p.locally_muted {
        Some("audio-volume-muted-symbolic")
    } else if p.deafened {
        Some("audio-volume-muted-symbolic")
    } else if p.muted {
        Some("microphone-sensitivity-muted-symbolic")
    } else {
        None
    };
    if let Some(icon_name) = status_icon {
        participant = participant.push(icon::from_name(icon_name).size(16));
    }
    // Click on someone: show / hide their volume
    let expanded = applet.expanded_participant.as_deref() == Some(p.id.as_str());
    list::button(participant).selected(expanded).on_press(Message::ToggleParticipant(p.id.clone()))
}

/// Volume under the participant: my mic (0-100 %) or how loud I hear them (0-200 %).
fn volume_row<'a>(applet: &Applet, state: &State, p: &Participant) -> Element<'a, Message> {
    let is_self = p.id == state.self_id;
    let key = if is_self { String::new() } else { p.id.clone() };
    let value = match &applet.dragged_volume {
        Some((id, v)) if *id == key => *v,
        _ if is_self => state.mic_volume,
        _ => p.volume,
    };
    let max = if is_self { 100.0 } else { 200.0 };
    // Mute button: my mic for me, local mute (only I stop hearing them) for others
    let (muted, mute_command) = if is_self {
        (state.mic_muted, command(&["set_mute", if state.mic_muted { "false" } else { "true" }]))
    } else {
        (p.locally_muted, command(&["set_user_mute", &p.id, if p.locally_muted { "false" } else { "true" }]))
    };
    let mute_icon = match (is_self, muted) {
        (true, true) => "microphone-sensitivity-muted-symbolic",
        (true, false) => "audio-input-microphone-symbolic",
        (false, true) => "audio-volume-muted-symbolic",
        (false, false) => "audio-volume-high-symbolic",
    };
    let mute_button = button::icon(icon::from_name(mute_icon)).selected(muted).on_press(mute_command);
    row()
        .padding([0, 4])
        .push(mute_button)
        .push(slider(0.0..=max, value, move |v| Message::Volume(key.clone(), v)).on_release(Message::VolumeReleased))
        .push(text::caption(format!("{} %", value.round())).width(Length::Fixed(42.0)).align_x(Alignment::End))
        .into()
}

/// Out of a call: favorites then recents, click = join.
fn out_of_call(state: &State) -> Element<'_, Message> {
    let is_favorite = |id: &str| state.favorites.iter().any(|c| c.id == id);
    let channels: Vec<&Channel> = state
        .favorites
        .iter()
        .chain(state.recents.iter().filter(|c| !is_favorite(&c.id)))
        .collect();
    let mut column = Column::new()
        .spacing(12)
        .push(header("audio-headset-symbolic", false, "Discord".into(), "Pas en vocal".into(), None));
    if channels.is_empty() {
        return column.into();
    }
    let mut channel_list = list();
    for channel in channels {
        let detail = if channel.online_count > 0 {
            format!("{} · {} en ligne", guild_name(channel), channel.online_count)
        } else {
            guild_name(channel).to_string()
        };
        let favorite = is_favorite(&channel.id);
        let action = if favorite { "unfavorite_channel" } else { "favorite_channel" };
        let star = button::icon(icon::from_name(if favorite { "starred-symbolic" } else { "non-starred-symbolic" }))
            .selected(favorite)
            .on_press(command(&[action, &channel.id]));
        let channel_row = row()
            .push(Column::new().width(Length::Fill).push(text::body(&channel.name)).push(text::caption(detail)))
            .push(star);
        channel_list = channel_list.add(list::button(channel_row).on_press(command(&["join_channel", &channel.id])));
    }
    column = column.push(container(cosmic::widget::scrollable(channel_list)).max_height(400.0));
    column.into()
}
