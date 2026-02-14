use crate::AppState;
use crate::settings::{
    self, FpsLimitSetting, GameSettings, RenderDistanceSetting, ResolutionSetting,
    SettingsResource, ShadowQualitySetting, WindowModeSetting,
};
use bevy::camera::ClearColorConfig;
use bevy::prelude::*;
use bevy::state::prelude::OnEnter;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, EguiTextureHandle, egui};

const LOGIN_BACKGROUND: Color = Color::srgb(0.42, 0.42, 0.42);
const GAMEPLAY_BACKGROUND: Color = Color::srgb(0.1, 0.1, 0.15);

pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<HudUiState>()
            .init_resource::<HudAssets>()
            .add_systems(Startup, setup_hud_assets)
            .add_systems(OnEnter(AppState::Login), reset_login_hud_state)
            .add_systems(OnEnter(AppState::Gameplay), reset_gameplay_hud_state)
            .add_systems(
                Update,
                (
                    toggle_settings_modal_with_escape,
                    sync_world_camera_clear_color,
                )
                    .run_if(hud_states_active),
            )
            .add_systems(
                EguiPrimaryContextPass,
                draw_hud_egui.run_if(hud_states_active),
            );
    }
}

fn hud_states_active(state: Res<State<AppState>>) -> bool {
    matches!(state.get(), AppState::Login | AppState::Gameplay)
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum SettingsTab {
    #[default]
    Graphics,
    Audio,
}

#[derive(Resource)]
struct HudUiState {
    settings_open: bool,
    settings_tab: SettingsTab,
    draft: GameSettings,
    login_username: String,
    login_password: String,
}

impl Default for HudUiState {
    fn default() -> Self {
        Self {
            settings_open: false,
            settings_tab: SettingsTab::Graphics,
            draft: GameSettings::default(),
            login_username: String::new(),
            login_password: String::new(),
        }
    }
}

#[derive(Resource, Default)]
struct HudAssets {
    settings_icon: Handle<Image>,
    settings_icon_id: Option<egui::TextureId>,
}

fn setup_hud_assets(mut hud_assets: ResMut<HudAssets>, asset_server: Res<AssetServer>) {
    hud_assets.settings_icon = asset_server.load("ui/icons/tabler_settings_outline.png");
}

fn reset_login_hud_state(mut hud_state: ResMut<HudUiState>, settings: Res<SettingsResource>) {
    hud_state.settings_open = false;
    hud_state.settings_tab = SettingsTab::Graphics;
    hud_state.draft = settings.current.clone();
    hud_state.login_username.clear();
    hud_state.login_password.clear();
}

fn reset_gameplay_hud_state(mut hud_state: ResMut<HudUiState>, settings: Res<SettingsResource>) {
    hud_state.settings_open = false;
    hud_state.settings_tab = SettingsTab::Graphics;
    hud_state.draft = settings.current.clone();
}

fn sync_world_camera_clear_color(
    app_state: Res<State<AppState>>,
    mut world_cameras: Query<&mut Camera, With<Camera3d>>,
) {
    for mut camera in &mut world_cameras {
        camera.clear_color = match app_state.get() {
            AppState::Login | AppState::Loading => ClearColorConfig::Custom(LOGIN_BACKGROUND),
            AppState::Gameplay => ClearColorConfig::Custom(GAMEPLAY_BACKGROUND),
        };
    }
}

fn toggle_settings_modal_with_escape(
    keys: Res<ButtonInput<KeyCode>>,
    settings_resource: Res<SettingsResource>,
    mut hud_state: ResMut<HudUiState>,
) {
    if !keys.just_pressed(KeyCode::Escape) {
        return;
    }

    if !hud_state.settings_open {
        hud_state.draft = settings_resource.current.clone();
        hud_state.settings_tab = SettingsTab::Graphics;
    }

    hud_state.settings_open = !hud_state.settings_open;
}

fn draw_hud_egui(
    mut contexts: EguiContexts,
    mut hud_state: ResMut<HudUiState>,
    mut hud_assets: ResMut<HudAssets>,
    mut settings_resource: ResMut<SettingsResource>,
    app_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
    mut theme_initialized: Local<bool>,
) {
    if hud_assets.settings_icon_id.is_none() {
        hud_assets.settings_icon_id =
            Some(contexts.add_image(EguiTextureHandle::Strong(hud_assets.settings_icon.clone())));
    }
    let settings_icon_id = hud_assets.settings_icon_id;

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    if !*theme_initialized {
        apply_modern_hud_theme(ctx);
        *theme_initialized = true;
    }

    if matches!(app_state.get(), AppState::Login) {
        draw_login_form(&mut hud_state, ctx, &mut next_state);
    }

    draw_bottom_bar(&mut hud_state, &settings_resource, ctx, settings_icon_id);

    if hud_state.settings_open {
        draw_settings_modal(
            &mut hud_state,
            &mut settings_resource,
            app_state.get(),
            &mut next_state,
            ctx,
        );
    }
}

fn apply_modern_hud_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(14);
    style.visuals.window_corner_radius = egui::CornerRadius::same(12);
    style.visuals.menu_corner_radius = egui::CornerRadius::same(10);
    style.visuals.widgets.active.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(8);
    style.visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(8);
    ctx.set_style(style);
}

fn draw_login_form(
    hud_state: &mut HudUiState,
    ctx: &egui::Context,
    next_state: &mut ResMut<NextState<AppState>>,
) {
    egui::Window::new("Login")
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -30.0))
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.heading("Acesso ao Jogo");
            ui.label("Entre para carregar o World1.");
            ui.add_space(6.0);

            ui.label("Usuario");
            ui.add(
                egui::TextEdit::singleline(&mut hud_state.login_username)
                    .desired_width(320.0)
                    .hint_text("Digite seu usuario"),
            );

            ui.label("Senha");
            ui.add(
                egui::TextEdit::singleline(&mut hud_state.login_password)
                    .password(true)
                    .desired_width(320.0)
                    .hint_text("Digite sua senha"),
            );

            ui.add_space(8.0);
            if ui
                .add_sized(egui::vec2(320.0, 34.0), egui::Button::new("Login"))
                .clicked()
            {
                next_state.set(AppState::Gameplay);
            }
        });
}

fn draw_bottom_bar(
    hud_state: &mut HudUiState,
    settings_resource: &SettingsResource,
    ctx: &egui::Context,
    settings_icon_id: Option<egui::TextureId>,
) {
    egui::TopBottomPanel::bottom("hud_bottom_bar")
        .resizable(false)
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal_centered(|ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128))
                    .corner_radius(egui::CornerRadius::same(12))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let open_settings = if let Some(texture_id) = settings_icon_id {
                                let image = egui::Image::new((texture_id, egui::vec2(18.0, 18.0)));
                                ui.add(
                                    egui::Button::image(image)
                                        .min_size(egui::vec2(42.0, 42.0))
                                        .frame(true),
                                )
                                .clicked()
                            } else {
                                ui.add_sized(egui::vec2(42.0, 42.0), egui::Button::new("Menu"))
                                    .clicked()
                            };

                            if open_settings {
                                hud_state.settings_open = true;
                                hud_state.settings_tab = SettingsTab::Graphics;
                                hud_state.draft = settings_resource.current.clone();
                            }
                        });
                    });
            });
            ui.add_space(8.0);
        });
}

fn draw_settings_modal(
    hud_state: &mut HudUiState,
    settings_resource: &mut SettingsResource,
    app_state: &AppState,
    next_state: &mut ResMut<NextState<AppState>>,
    ctx: &egui::Context,
) {
    let was_open = hud_state.settings_open;
    let mut window_open = hud_state.settings_open;
    let mut should_apply = false;
    let mut should_close = false;
    let mut should_logout = false;

    egui::Window::new("Settings")
        .open(&mut window_open)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .default_width(600.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut hud_state.settings_tab,
                    SettingsTab::Graphics,
                    "Grafico",
                );
                ui.selectable_value(&mut hud_state.settings_tab, SettingsTab::Audio, "Som");
            });

            ui.separator();

            match hud_state.settings_tab {
                SettingsTab::Graphics => {
                    draw_graphics_settings_tab(ui, &mut hud_state.draft);
                }
                SettingsTab::Audio => {
                    draw_audio_settings_tab(ui, &mut hud_state.draft);
                }
            }

            ui.separator();
            ui.horizontal(|ui| {
                should_logout = ui
                    .add(egui::Button::new("Sair").fill(egui::Color32::from_rgb(121, 42, 42)))
                    .clicked();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    should_apply = ui.button("Aplicar").clicked();
                    should_close = ui.button("Fechar").clicked();
                });
            });
        });

    if should_apply {
        settings_resource.current = hud_state.draft.clone();
        if let Err(error) = settings_resource.save_to_disk() {
            warn!(
                "Failed to save settings file '{}': {}",
                settings::SETTINGS_FILE_PATH,
                error
            );
        }
    }

    if should_close {
        window_open = false;
        hud_state.draft = settings_resource.current.clone();
    }

    if should_logout {
        window_open = false;
        hud_state.draft = settings_resource.current.clone();
        if matches!(app_state, AppState::Gameplay) {
            next_state.set(AppState::Login);
        }
    }

    hud_state.settings_open = window_open;

    if was_open && !hud_state.settings_open {
        hud_state.draft = settings_resource.current.clone();
    }
}

fn draw_graphics_settings_tab(ui: &mut egui::Ui, draft: &mut GameSettings) {
    egui::ComboBox::from_label("Tipo de janela")
        .selected_text(draft.graphics.window_mode.label())
        .show_ui(ui, |ui| {
            for option in WindowModeSetting::ALL {
                ui.selectable_value(&mut draft.graphics.window_mode, option, option.label());
            }
        });

    egui::ComboBox::from_label("Resolucao")
        .selected_text(draft.graphics.resolution.label())
        .show_ui(ui, |ui| {
            for option in ResolutionSetting::presets() {
                ui.selectable_value(&mut draft.graphics.resolution, *option, option.label());
            }
        });

    egui::ComboBox::from_label("Sombras")
        .selected_text(draft.graphics.shadow_quality.label())
        .show_ui(ui, |ui| {
            for option in ShadowQualitySetting::ALL {
                ui.selectable_value(&mut draft.graphics.shadow_quality, option, option.label());
            }
        });

    egui::ComboBox::from_label("Limite de FPS")
        .selected_text(draft.graphics.fps_limit.label())
        .show_ui(ui, |ui| {
            for option in FpsLimitSetting::ALL {
                ui.selectable_value(&mut draft.graphics.fps_limit, option, option.label());
            }
        });

    egui::ComboBox::from_label("Distancia de render")
        .selected_text(draft.graphics.render_distance.label())
        .show_ui(ui, |ui| {
            for option in RenderDistanceSetting::ALL {
                ui.selectable_value(&mut draft.graphics.render_distance, option, option.label());
            }
        });

    ui.checkbox(&mut draft.graphics.vsync, "VSync");
    ui.checkbox(&mut draft.graphics.show_grass, "Mostrar grama");
    ui.checkbox(
        &mut draft.graphics.use_remaster_assets,
        "Usar assets remaster (F10)",
    );
}

fn draw_audio_settings_tab(ui: &mut egui::Ui, draft: &mut GameSettings) {
    ui.checkbox(&mut draft.audio.ambient_enabled, "Som ambiente");
    ui.checkbox(&mut draft.audio.effects_enabled, "Outros sons");
}
