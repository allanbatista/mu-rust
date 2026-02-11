use crate::settings::{GameSettings, SettingsResource};
use crate::{AppState, settings};
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::state::prelude::{NextState, OnEnter, OnExit};
use bevy::ui::{IsDefaultUiCamera, UiTargetCamera};

const COLOR_TEXT: Color = Color::srgb(0.95, 0.95, 0.95);
const COLOR_TEXT_MUTED: Color = Color::srgb(0.74, 0.75, 0.78);
const COLOR_MODAL_BG: Color = Color::srgba(0.06, 0.07, 0.09, 0.96);
const COLOR_MODAL_SCRIM: Color = Color::srgba(0.0, 0.0, 0.0, 0.55);
const COLOR_BUTTON_NORMAL: Color = Color::srgba(0.12, 0.14, 0.18, 0.95);
const COLOR_BUTTON_HOVER: Color = Color::srgba(0.2, 0.22, 0.26, 0.98);
const COLOR_BUTTON_PRESSED: Color = Color::srgba(0.3, 0.34, 0.4, 0.98);
const COLOR_BUTTON_PRIMARY: Color = Color::srgba(0.16, 0.43, 0.83, 0.95);
const COLOR_BUTTON_PRIMARY_HOVER: Color = Color::srgba(0.2, 0.49, 0.9, 0.98);
const COLOR_BUTTON_PRIMARY_PRESSED: Color = Color::srgba(0.12, 0.37, 0.74, 1.0);
const COLOR_BAR_BG: Color = Color::srgba(0.0, 0.0, 0.0, 0.5);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SettingsModalState>()
            .add_systems(Startup, setup_hud_ui_camera)
            .add_systems(
                OnEnter(AppState::Login),
                (reset_modal_state, spawn_login_hud),
            )
            .add_systems(OnExit(AppState::Login), cleanup_login_hud)
            .add_systems(
                OnEnter(AppState::Gameplay),
                (reset_modal_state, spawn_gameplay_hud),
            )
            .add_systems(OnExit(AppState::Gameplay), cleanup_gameplay_hud)
            .add_systems(
                Update,
                (
                    toggle_settings_modal_with_escape,
                    handle_settings_open_buttons,
                    handle_settings_cycle_buttons,
                    handle_settings_modal_action_buttons,
                    handle_login_submit_button,
                    sync_settings_modal_visibility,
                    sync_settings_field_labels,
                    sync_hud_ui_camera_clear_color,
                    update_button_visual_feedback,
                )
                    .run_if(hud_states_active),
            );
    }
}

fn hud_states_active(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Login | AppState::Gameplay)
}

#[derive(Resource)]
struct SettingsModalState {
    open: bool,
    draft: GameSettings,
}

impl Default for SettingsModalState {
    fn default() -> Self {
        Self {
            open: false,
            draft: GameSettings::default(),
        }
    }
}

#[derive(Component)]
struct LoginHudRoot;

#[derive(Component)]
struct GameplayHudRoot;

#[derive(Component)]
struct SettingsModalRoot;

#[derive(Component)]
struct SettingsOpenButton;

#[derive(Component, Clone, Copy)]
enum SettingsModalAction {
    Save,
    Cancel,
}

#[derive(Component)]
struct SettingsModalActionButton(SettingsModalAction);

#[derive(Component)]
struct LoginSubmitButton;

#[derive(Component)]
struct PrimaryActionButton;

#[derive(Component, Clone, Copy)]
enum SettingsCycleField {
    WindowMode,
    Resolution,
    ShadowQuality,
    Vsync,
    FpsLimit,
    RenderDistance,
    AmbientSound,
    EffectsSound,
}

#[derive(Component)]
struct SettingsCycleButton(SettingsCycleField);

#[derive(Component)]
struct SettingsCycleLabel(SettingsCycleField);

#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudUiCamera;

fn reset_modal_state(
    mut modal: ResMut<SettingsModalState>,
    settings_resource: Res<SettingsResource>,
) {
    modal.open = false;
    modal.draft = settings_resource.current.clone();
}

fn spawn_login_hud(mut commands: Commands, hud_camera_query: Query<Entity, With<HudUiCamera>>) {
    info!("Spawning login HUD");

    let root = commands
        .spawn((
            HudRoot,
            LoginHudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                ..default()
            },
            Visibility::Visible,
            GlobalZIndex(10),
        ))
        .id();

    if let Ok(hud_camera) = hud_camera_query.single() {
        commands.entity(root).insert(UiTargetCamera(hud_camera));
    }

    commands.entity(root).with_children(|parent| {
        parent
            .spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                Visibility::Visible,
            ))
            .with_children(|center| {
                center
                    .spawn((
                        Node {
                            width: px(440),
                            padding: UiRect::all(px(20)),
                            row_gap: px(12),
                            flex_direction: FlexDirection::Column,
                            border_radius: BorderRadius::all(px(14)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.06, 0.08, 0.1, 0.88)),
                    ))
                    .with_children(|form| {
                        form.spawn((
                            Text::new("Login"),
                            TextFont {
                                font_size: 30.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT),
                        ));
                        form.spawn((
                            Text::new("Acesse com sua conta para entrar no gameplay."),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT_MUTED),
                        ));
                        spawn_login_input_like_field(form, "Usuario");
                        spawn_login_input_like_field(form, "Senha");
                        form.spawn((
                            Button,
                            LoginSubmitButton,
                            PrimaryActionButton,
                            Node {
                                width: percent(100),
                                height: px(42),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                border_radius: BorderRadius::all(px(10)),
                                ..default()
                            },
                            BackgroundColor(COLOR_BUTTON_PRIMARY),
                        ))
                        .with_children(|button| {
                            button.spawn((
                                Text::new("Entrar"),
                                TextFont {
                                    font_size: 17.0,
                                    ..default()
                                },
                                TextColor(COLOR_TEXT),
                            ));
                        });
                    });
            });

        spawn_bottom_bar(parent);
        spawn_settings_modal(parent);
    });
}

fn spawn_gameplay_hud(
    mut commands: Commands,
    hud_camera_query: Query<Entity, With<HudUiCamera>>,
) {
    info!("Spawning gameplay HUD");

    let root = commands
        .spawn((
            HudRoot,
            GameplayHudRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                top: px(0),
                left: px(0),
                right: px(0),
                bottom: px(0),
                ..default()
            },
            Visibility::Visible,
            GlobalZIndex(10),
        ))
        .id();

    if let Ok(hud_camera) = hud_camera_query.single() {
        commands.entity(root).insert(UiTargetCamera(hud_camera));
    }

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            Text::new("Gameplay"),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(COLOR_TEXT_MUTED),
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                left: px(16),
                ..default()
            },
        ));

        spawn_bottom_bar(parent);
        spawn_settings_modal(parent);
    });
}

fn setup_hud_ui_camera(mut commands: Commands) {
    commands.spawn((
        HudUiCamera,
        Camera2d,
        IsDefaultUiCamera,
        Camera {
            order: 100,
            clear_color: ClearColorConfig::None,
            ..default()
        },
    ));
}

fn spawn_login_input_like_field(parent: &mut ChildSpawnerCommands, label: &str) {
    parent
        .spawn((
            Node {
                width: percent(100),
                height: px(42),
                border: UiRect::all(px(1)),
                padding: UiRect::axes(px(12), px(8)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.85, 0.88, 0.94, 0.28)),
            BackgroundColor(Color::srgba(0.05, 0.06, 0.08, 0.92)),
        ))
        .with_children(|input| {
            input.spawn((
                Text::new(label),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(COLOR_TEXT_MUTED),
            ));
        });
}

fn spawn_bottom_bar(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                width: percent(100),
                position_type: PositionType::Absolute,
                bottom: px(16),
                justify_content: JustifyContent::Center,
                ..default()
            },
            Visibility::Visible,
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    padding: UiRect::axes(px(16), px(10)),
                    column_gap: px(10),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius: BorderRadius::all(px(12)),
                    ..default()
                },
                BackgroundColor(COLOR_BAR_BG),
            ))
            .with_children(|items| {
                items
                    .spawn((
                        Button,
                        SettingsOpenButton,
                        Node {
                            width: px(46),
                            height: px(46),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            border_radius: BorderRadius::all(px(10)),
                            ..default()
                        },
                        BackgroundColor(COLOR_BUTTON_NORMAL),
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new("⚙"),
                            TextFont {
                                font_size: 13.0,
                                ..default()
                            },
                            TextColor(COLOR_TEXT),
                        ));
                    });
            });
        });
}

fn spawn_settings_modal(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            SettingsModalRoot,
            Node {
                width: percent(100),
                height: percent(100),
                position_type: PositionType::Absolute,
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(COLOR_MODAL_SCRIM),
        ))
        .with_children(|overlay| {
            overlay
                .spawn((
                    Node {
                        width: px(620),
                        max_height: percent(88),
                        flex_direction: FlexDirection::Column,
                        row_gap: px(10),
                        padding: UiRect::all(px(20)),
                        overflow: Overflow::clip(),
                        border_radius: BorderRadius::all(px(14)),
                        ..default()
                    },
                    BackgroundColor(COLOR_MODAL_BG),
                ))
                .with_children(|modal| {
                    modal.spawn((
                        Text::new("Settings"),
                        TextFont {
                            font_size: 28.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                    ));

                    modal.spawn((
                        Text::new("Grafico"),
                        TextFont {
                            font_size: 19.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                    ));

                    spawn_settings_cycle_row(modal, SettingsCycleField::WindowMode);
                    spawn_settings_cycle_row(modal, SettingsCycleField::Resolution);
                    spawn_settings_cycle_row(modal, SettingsCycleField::ShadowQuality);
                    spawn_settings_cycle_row(modal, SettingsCycleField::Vsync);
                    spawn_settings_cycle_row(modal, SettingsCycleField::FpsLimit);
                    spawn_settings_cycle_row(modal, SettingsCycleField::RenderDistance);

                    modal.spawn((
                        Text::new("Som"),
                        TextFont {
                            font_size: 19.0,
                            ..default()
                        },
                        TextColor(COLOR_TEXT),
                        Node {
                            margin: UiRect::top(px(8)),
                            ..default()
                        },
                    ));

                    spawn_settings_cycle_row(modal, SettingsCycleField::AmbientSound);
                    spawn_settings_cycle_row(modal, SettingsCycleField::EffectsSound);

                    modal
                        .spawn((
                            Node {
                                width: percent(100),
                                justify_content: JustifyContent::FlexEnd,
                                column_gap: px(10),
                                margin: UiRect::top(px(8)),
                                ..default()
                            },
                            Visibility::Visible,
                        ))
                        .with_children(|footer| {
                            footer
                                .spawn((
                                    Button,
                                    SettingsModalActionButton(SettingsModalAction::Cancel),
                                    Node {
                                        width: px(120),
                                        height: px(40),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border_radius: BorderRadius::all(px(10)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_BUTTON_NORMAL),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Cancelar"),
                                        TextFont {
                                            font_size: 15.0,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT),
                                    ));
                                });

                            footer
                                .spawn((
                                    Button,
                                    SettingsModalActionButton(SettingsModalAction::Save),
                                    PrimaryActionButton,
                                    Node {
                                        width: px(170),
                                        height: px(40),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        border_radius: BorderRadius::all(px(10)),
                                        ..default()
                                    },
                                    BackgroundColor(COLOR_BUTTON_PRIMARY),
                                ))
                                .with_children(|button| {
                                    button.spawn((
                                        Text::new("Salvar e Aplicar"),
                                        TextFont {
                                            font_size: 15.0,
                                            ..default()
                                        },
                                        TextColor(COLOR_TEXT),
                                    ));
                                });
                        });
                });
        });
}

fn spawn_settings_cycle_row(parent: &mut ChildSpawnerCommands, field: SettingsCycleField) {
    parent
        .spawn((
            Button,
            SettingsCycleButton(field),
            Node {
                width: percent(100),
                min_height: px(40),
                border: UiRect::all(px(1)),
                padding: UiRect::axes(px(12), px(8)),
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(px(10)),
                ..default()
            },
            BorderColor::all(Color::srgba(0.85, 0.88, 0.94, 0.18)),
            BackgroundColor(COLOR_BUTTON_NORMAL),
        ))
        .with_children(|button| {
            button.spawn((
                Text::new(""),
                SettingsCycleLabel(field),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
                TextColor(COLOR_TEXT),
            ));
        });
}

fn cleanup_login_hud(mut commands: Commands, query: Query<Entity, With<LoginHudRoot>>) {
    for entity in &query {
        commands.entity(entity).try_despawn();
    }
}

fn cleanup_gameplay_hud(mut commands: Commands, query: Query<Entity, With<GameplayHudRoot>>) {
    for entity in &query {
        commands.entity(entity).try_despawn();
    }
}

fn sync_hud_ui_camera_clear_color(
    app_state: Res<State<AppState>>,
    mut hud_camera_query: Query<&mut Camera, With<HudUiCamera>>,
) {
    let Ok(mut camera) = hud_camera_query.single_mut() else {
        return;
    };

    let desired_clear = match app_state.get() {
        AppState::Login => ClearColorConfig::Custom(Color::srgb(0.42, 0.42, 0.42)),
        AppState::Gameplay => ClearColorConfig::None,
        AppState::Loading => ClearColorConfig::Custom(Color::srgb(0.42, 0.42, 0.42)),
    };

    camera.clear_color = desired_clear;
}

fn toggle_settings_modal_with_escape(
    keys: Res<ButtonInput<KeyCode>>,
    settings_resource: Res<SettingsResource>,
    mut modal: ResMut<SettingsModalState>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    if !modal.open {
        modal.draft = settings_resource.current.clone();
    }
    modal.open = !modal.open;
}

fn handle_settings_open_buttons(
    settings_resource: Res<SettingsResource>,
    mut modal: ResMut<SettingsModalState>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<SettingsOpenButton>)>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            modal.open = true;
            modal.draft = settings_resource.current.clone();
        }
    }
}

fn handle_settings_cycle_buttons(
    mut modal: ResMut<SettingsModalState>,
    interactions: Query<(&Interaction, &SettingsCycleButton), Changed<Interaction>>,
) {
    for (interaction, button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match button.0 {
            SettingsCycleField::WindowMode => {
                modal.draft.graphics.window_mode = modal.draft.graphics.window_mode.next();
            }
            SettingsCycleField::Resolution => {
                modal.draft.graphics.resolution = modal.draft.graphics.resolution.next();
            }
            SettingsCycleField::ShadowQuality => {
                modal.draft.graphics.shadow_quality = modal.draft.graphics.shadow_quality.next();
            }
            SettingsCycleField::Vsync => {
                modal.draft.graphics.vsync = !modal.draft.graphics.vsync;
            }
            SettingsCycleField::FpsLimit => {
                modal.draft.graphics.fps_limit = modal.draft.graphics.fps_limit.next();
            }
            SettingsCycleField::RenderDistance => {
                modal.draft.graphics.render_distance = modal.draft.graphics.render_distance.next();
            }
            SettingsCycleField::AmbientSound => {
                modal.draft.audio.ambient_enabled = !modal.draft.audio.ambient_enabled;
            }
            SettingsCycleField::EffectsSound => {
                modal.draft.audio.effects_enabled = !modal.draft.audio.effects_enabled;
            }
        }
    }
}

fn handle_settings_modal_action_buttons(
    mut modal: ResMut<SettingsModalState>,
    mut settings_resource: ResMut<SettingsResource>,
    interactions: Query<
        (&Interaction, &SettingsModalActionButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, action_button) in &interactions {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action_button.0 {
            SettingsModalAction::Save => {
                settings_resource.current = modal.draft.clone();
                if let Err(error) = settings_resource.save_to_disk() {
                    warn!(
                        "Failed to save settings file '{}': {}",
                        settings::SETTINGS_FILE_PATH,
                        error
                    );
                }
                modal.open = false;
            }
            SettingsModalAction::Cancel => {
                modal.open = false;
                modal.draft = settings_resource.current.clone();
            }
        }
    }
}

fn handle_login_submit_button(
    mut next_state: ResMut<NextState<AppState>>,
    interactions: Query<&Interaction, (Changed<Interaction>, With<LoginSubmitButton>)>,
) {
    for interaction in &interactions {
        if *interaction == Interaction::Pressed {
            next_state.set(AppState::Gameplay);
        }
    }
}

fn sync_settings_modal_visibility(
    modal: Res<SettingsModalState>,
    mut query: Query<&mut Node, With<SettingsModalRoot>>,
) {
    if !modal.is_changed() {
        return;
    }

    for mut node in &mut query {
        node.display = if modal.open {
            Display::Flex
        } else {
            Display::None
        };
    }
}

fn sync_settings_field_labels(
    modal: Res<SettingsModalState>,
    mut labels: Query<(&SettingsCycleLabel, &mut Text)>,
) {
    if !modal.is_changed() {
        return;
    }

    for (label, mut text) in &mut labels {
        text.0 = settings_field_text(label.0, &modal.draft);
    }
}

fn settings_field_text(field: SettingsCycleField, settings: &GameSettings) -> String {
    match field {
        SettingsCycleField::WindowMode => {
            format!("Window Mode: {}", settings.graphics.window_mode.label())
        }
        SettingsCycleField::Resolution => {
            format!("Resolution: {}", settings.graphics.resolution.label())
        }
        SettingsCycleField::ShadowQuality => {
            format!("Shadows: {}", settings.graphics.shadow_quality.label())
        }
        SettingsCycleField::Vsync => {
            format!("VSync: {}", yes_no(settings.graphics.vsync))
        }
        SettingsCycleField::FpsLimit => {
            format!("FPS Limit: {}", settings.graphics.fps_limit.label())
        }
        SettingsCycleField::RenderDistance => {
            format!(
                "Render Distance: {}",
                settings.graphics.render_distance.label()
            )
        }
        SettingsCycleField::AmbientSound => {
            format!("Ambient Sound: {}", yes_no(settings.audio.ambient_enabled))
        }
        SettingsCycleField::EffectsSound => {
            format!("Effects Sound: {}", yes_no(settings.audio.effects_enabled))
        }
    }
}

fn yes_no(value: bool) -> &'static str {
    if value { "Enabled" } else { "Disabled" }
}

fn update_button_visual_feedback(
    mut query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Has<PrimaryActionButton>,
            Has<SettingsModalActionButton>,
            Has<SettingsCycleButton>,
            Has<SettingsOpenButton>,
            Has<LoginSubmitButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (
        interaction,
        mut background,
        is_primary,
        _is_modal_action,
        _is_cycle,
        _is_open,
        _is_login_submit,
    ) in &mut query
    {
        match *interaction {
            Interaction::Pressed => {
                *background = if is_primary {
                    COLOR_BUTTON_PRIMARY_PRESSED.into()
                } else {
                    COLOR_BUTTON_PRESSED.into()
                };
            }
            Interaction::Hovered => {
                *background = if is_primary {
                    COLOR_BUTTON_PRIMARY_HOVER.into()
                } else {
                    COLOR_BUTTON_HOVER.into()
                };
            }
            Interaction::None => {
                *background = if is_primary {
                    COLOR_BUTTON_PRIMARY.into()
                } else {
                    COLOR_BUTTON_NORMAL.into()
                };
            }
        }
    }
}
