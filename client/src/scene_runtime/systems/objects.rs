use super::particles::particle_emitter_from_definition;
use crate::bevy_compat::*;
use crate::legacy_additive::{
    LegacyAdditiveMaterial, legacy_additive_from_standard, legacy_additive_intensity_from_extras,
};
use crate::scene_runtime::components::*;
use crate::scene_runtime::scene_loader::{SceneObjectsMetadata, SceneRotationEncoding};
use crate::scene_runtime::state::RuntimeSceneAssets;
use crate::scene_runtime::transforms::scene_object_rotation_to_quat;
use crate::scene_runtime::world_coordinates::{
    WorldMirrorAxis, mirror_map_position_with_axis, world_mirror_axis,
};
use bevy::ecs::system::EntityCommands;
use bevy::gltf::{Gltf, GltfMaterialExtras};
use bevy::math::primitives::Cuboid;
use bevy::prelude::*;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

const DEFAULT_SCENE_OBJECT_ANIMATION_SPEED: f32 = 0.16;
const DEFAULT_NPC_MONSTER_ANIMATION_SPEED: f32 = 0.25;
const DEFAULT_MU_SCENE_OBJECT_YAW_OFFSET_DEGREES: f32 = 0.0;
const SCENE_OBJECT_YAW_OFFSET_ENV: &str = "MU_SCENE_OBJECT_YAW_OFFSET_DEGREES";
const DEFAULT_SCENE_OBJECT_CULL_DISTANCE: f32 = 25000.0;
const SCENE_OBJECT_CULL_DISTANCE_ENV: &str = "MU_SCENE_OBJECT_CULL_DISTANCE";
const SCENE_OBJECTS_UNLIT_ENV: &str = "MU_SCENE_OBJECTS_UNLIT";

fn scene_objects_unlit() -> bool {
    static UNLIT: OnceLock<bool> = OnceLock::new();
    *UNLIT.get_or_init(|| {
        std::env::var(SCENE_OBJECTS_UNLIT_ENV)
            .ok()
            .map(|v| {
                !matches!(
                    v.trim().to_ascii_lowercase().as_str(),
                    "false" | "0" | "off" | "no"
                )
            })
            .unwrap_or(true) // default: unlit
    })
}

/// Marker component to track if scene objects have been spawned### Iniciativa 1 — Dashboard Gerencial v2.1 (Refactoring, Migration, and Expansion)
//
// A iniciativa abrange o desenvolvimento do Dashboard Gerencial e a migração Snowflake→Redshift — trabalhos com timelines distintas. O direcionamento foi recebido sobre o que deveria ser feito, com autonomia para reescrever os relatórios da melhor forma, mas a estratégia da migração não foi definida pelo colaborador. Trabalho colaborativo com outros membros do time; os reports realizados tiveram pouca ou nenhuma ajuda.
//
// **Scope & Complexity (Performing at the next level)**
//
// Execução de migração com direcionamento recebido do EM. Avaliou-se a complexidade como mediana (muito mais trabalhosa do que complexa) — a dificuldade maior é aprender a nova tecnologia (Redshift) e garantir corretude dos resultados. Escopo de implementador com autonomia em tarefa bem definida. Destaca-se a utilização de IA como principal ferramenta de apoio na tradução ágil dos relatórios de Snowflake para Redshift, o que acelerou significativamente o processo de migração. No peer review, destaca-se "papel crucial na migração do Snowflake para o Redshift, com grande capacidade de execução e resiliência ao reescrever centenas de relatórios"; outro revisor atribui "total crédito ao Victor pela migração e refatoração dos dashboards" com mais de 100 relatórios refatorados.
//
// **Impact & Execution (Meets expectations)**
//
// Redução de runtime de múltiplos relatórios de >1 minuto para <10 segundos. Os stakeholders já usavam os relatórios no Snowflake; o principal impacto é mudança de tecnologia sem afetar o usuário final no dia a dia. A migração foi concluída preservando consistência de dados. Evidência: planilha de controle de migração ([link](https://docs.google.com/spreadsheets/d/1qoTFODHVfXp_VJxRzM5y6qkgLCAh4gJQEinDpHow82A/edit?gid=0#gid=0)).
//
// **Influence & Collaboration (Meets expectations) / Technical Vision & Articulation (Meets expectations)**
//
// Observou-se interação contínua com stakeholders (incluindo clientes diretos) para validar prioridades e usabilidade durante a migração. O trabalho de migração foi colaborativo com outros membros do time, com execução autônoma nos relatórios atribuídos. No peer review, atribui-se "total crédito ao Victor pela migração e refatoração dos dashboards" e destaca-se a parceria com os times de negócio. No aspecto técnico, registrou-se padronização de naming, filtros e lógica de segmentação durante a migração, além da tradução de tecnologia Snowflake→Redshift. A contribuição manteve-se no âmbito de seguir a visão técnica do time com autonomia na implementação.
//
// ---
//
// ### Iniciativa 2 — Black Friday 2025: Forecast & Pacing
//
// A complexidade dos relatórios foi muito maior do que se conseguiu absorver no tempo disponível. Demonstrou-se alto nível de compromisso (disponibilidade overnight, engajamento durante toda a madrugada), mas não foi possível implementar os reports. Foi necessário que todos os reports fossem refeitos pelo EM (Allan Batista) para atender aos requisitos e entregar no prazo. Considera-se que, com mais tempo, a entrega teria sido realizada. Este foi o caso de pressão mais extremo do ano.
//
// **Scope & Complexity (Meets expectations)**
//
// Demanda de urgência que excedeu a capacidade técnica atual sob restrição de tempo. Foi necessário que o EM auxiliasse na finalização dos reports para atender aos requisitos dentro do prazo. Esta iniciativa revela o limite atual de autonomia sob alta complexidade e pressão de tempo.
//
// **Impact & Execution (Meets expectations)**
//
// O impacto de decisões operacionais em tempo real no BF é real. A entrega técnica foi concluída com apoio significativo do EM. O mérito registrado é o comprometimento e disponibilidade — observou-se engajamento durante toda a madrugada. A proposta de melhoria de sazonalidade pós-BF (same weekday → same day-of-month) é positiva como iteração. Evidência: thread Slack ([link](https://vtex.slack.com/archives/C054BB5GQ05/p1764199108385019)).
//
// **Influence & Collaboration (Meets expectations) / Technical Vision & Articulation (Meets expectations)**
//
// Observou-se engajamento durante toda a madrugada em contexto de alta pressão, com disponibilidade e comprometimento demonstrados. A colaboração com o EM e suporte on-call foi mantida ao longo da noite. Registrou-se tentativa de implementação de abordagem de forecast com curva histórica horária, e proposta pós-evento de melhoria de sazonalidade (mesma data do mês em vez de mesmo dia da semana). Embora a entrega técnica final tenha sido realizada pelo EM, a iteração pós-BF demonstra capacidade de reflexão técnica e proposição de ajustes dentro do domínio.
//
// ---
//
// ### Iniciativa 3 — Buscador de Oportunidade (Opportunity Finder)
//
// A necessidade e os requisitos do dashboard foram definidos por João Gracioto e Henrique Sato (Casas Bahia). O desenvolvimento foi realizado de forma autônoma. O time de Performance utiliza o dashboard com frequência para tirar insights, conforme confirmação gerencial.
//
// **Scope & Complexity (Performing at the next level)**
//
// Execução autônoma de dashboard com especificação recebida de stakeholders. Provavelmente a iniciativa onde se demonstrou maior independência técnica. Foram implementados três pilares analíticos com métricas customizadas (ads penetration, market share, ROAS indicators). No peer review, destaca-se que o dashboard "transformou dados de vendas e Ads em uma visão única e acionável, melhorando a tomada de decisão dos times comerciais e de desempenho" e que "elevou a assertividade estratégica".
//
// **Impact & Execution (Performing at the next level)**
//
// Adoção real confirmada via validação com time de Performance. Documentação de 8 páginas para self-service onboarding. O impacto verificável é: dashboard funcional com adoção ativa que reduz time-to-insight para os times de Commercial e Performance. Evidência: dashboard Metabase ([link](https://metabase.newtail.com.br/dashboard/164-buscador-de-oportunidades?start_date=2025-01-01&end_date=2025-04-28&aggregate=month)), documentação ([link](https://docs.google.com/document/d/10o4XIZH1iaAim0pMVYxwaq9rVuOssGYjMqbPFhFRFs4/edit?tab=t.0)).
//
// **Technical Vision & Articulation (Performing at the next level) / Influence & Collaboration (Performing at the next level)**
//
// Registrou-se a criação de métricas customizadas (ads penetration por categoria, market share, indicadores derivados de ROAS) e o design de três pilares analíticos integrados, dentro da especificação recebida de stakeholders. A documentação de 8 páginas para onboarding self-service demonstra capacidade de estruturar e comunicar lógica analítica. No aspecto de colaboração, confirmou-se adoção ativa pelo time de Performance para geração de insights. No peer review, destaca-se que o dashboard "transformou dados de vendas e Ads em uma visão única e acionável, melhorando a tomada de decisão dos times comerciais e de desempenho" e que "elevou a assertividade estratégica".
//
// ---
//
// ### Iniciativa 4 — Mix de Produtos (Portfolio Segmentation with ABC / Pareto)
//
// Demanda trazida por João Gracioto, mas as visualizações foram formuladas pelo colaborador — um passo além de pura execução de spec. Complexidade mediana. Execução realizada de forma totalmente autônoma. Identificou-se adoção baixa — relatório utilizado de forma pontual.
//
// **Scope & Complexity (Performing at the next level)**
//
// Contribuição na formulação das visualizações além da execução de spec recebida. Aplicação de técnica analítica estruturada (ABC/Pareto) ao contexto de negócio, com SQL otimizado para segmentação dinâmica. Avaliou-se a complexidade como mediana. No peer review, destaca-se que o projeto "traz claramente quais produtos realmente geram resultados, substituindo decisões intuitivas por análises orientadas a dados".
//
// **Impact & Execution (Meets expectations)**
//
// Adoção baixa e uso pontual, conforme avaliação gerencial. Documentação técnica/funcional de 8 páginas. O valor verificável está na execução autônoma, na contribuição ao design das visualizações, e na documentação. Evidência: dashboard Metabase ([link](https://metabase.newtail.com.br/dashboard/180-mix-de-produtos?publisher_id=3b7bcd3a-fde6-42d1-8de4-47a6e41a415a&in%25C3%25ADcio=2025-08-01&fim=2025-08-19&category_level=1&categoria=TV%20E%20VIDEO&segmento=A)), documentação ([link](https://docs.google.com/document/d/1H9PbxXFwPF4e15TY-WxOHBPir0HV_XPqU9itLQ5KZ8U/edit?tab=t.0)).
//
// **Technical Vision & Articulation (Performing at the next level) / Influence & Collaboration (Meets expectations)**
//
// Registrou-se contribuição na formulação das visualizações além da execução da especificação recebida — avaliou-se como um passo além de pura execução de spec. Aplicou-se técnica analítica estruturada (ABC/Pareto) ao contexto de negócio com SQL otimizado para segmentação dinâmica, acompanhada de documentação técnica/funcional de 8 páginas com racional estatístico. No peer review, destaca-se que o projeto "traz claramente quais produtos realmente geram resultados, substituindo decisões intuitivas por análises orientadas a dados". A interação com o stakeholder (João Gracioto / Casas Bahia) ficou limitada ao escopo da demanda original, e a adoção baixa e pontual restringe a evidência de influência.
//
// ---
//
// ### Iniciativa 5 — Stakeholder Enablement & Rapid-Response Analytics
//
// O trabalho ajudou o time de CS e Performance a argumentar melhor com o cliente. A complexidade foi média para baixa. Todas foram demandas levantadas pelo time de Performance. Trata-se de uma iniciativa "guarda-chuva" que agrupa demandas pontuais sob um título único.
//
// **Scope & Complexity (Meets expectations)**
//
// Atenderam-se demandas de múltiplos stakeholders (Performance, CS, FP&A) em contextos diversificados — retenção de churn, habilitação de negociação comercial, e consolidação de métricas financeiras. Trata-se de uma iniciativa "guarda-chuva" que agrupa demandas pontuais, todas de complexidade média para baixa conforme avaliação gerencial. A versatilidade de contextos demonstra amplitude de atuação, embora cada demanda individual tenha sido bem-escopada e recebida do time de Performance.
//
// **Impact & Execution (Meets expectations)**
//
// Capacidade de responder rapidamente a demandas urgentes de múltiplos stakeholders (Performance, CS, FP&A). RFC aprovada para FP&A demonstra capacidade de documentação estruturada ([link](https://docs.google.com/document/d/15eiTvuO2sffWvrB6Vz3MEeU44F0of43YSi8KgPTCi6s/edit?tab=t.0#heading=h.b38oaof1w0iw)). Impacto verificável: reports que serviram como evidência para argumentação de times de negócio em retenção e negociação comercial. No peer review, nota-se a capacidade de "entender o que o cliente realmente precisa, além da demanda imediata" e de buscar "resolver problemas reais dos usuários".
//
// **Influence & Collaboration (Meets expectations) / Technical Vision & Articulation (Meets expectations)**
//
// Observou-se suporte efetivo a múltiplos stakeholders não-técnicos, com reports utilizados como insumo direto para argumentação em negociações comerciais e retenção de clientes. No peer review, nota-se a capacidade de "entender o que o cliente realmente precisa, além da demanda imediata" e de buscar "resolver problemas reais dos usuários". Registrou-se a autoria de RFC aprovada para FP&A com métricas padronizadas e regras de segmentação auditáveis ([link](https://docs.google.com/document/d/15eiTvuO2sffWvrB6Vz3MEeU44F0of43YSi8KgPTCi6s/edit?tab=t.0#heading=h.b38oaof1w0iw)), demonstrando capacidade de documentação estruturada quando direcionado.
//
// ---
//
// ### Iniciativa 6 — Analytics Platform Discipline: Governance & Performance Roadmap
//
// Trata-se de pedido do EM para melhorar a organização dos relatórios do Metabase. A investigação ficou sob responsabilidade do colaborador, incluindo documentação e proposta de organização. A entrega foi feita com a qualidade esperada de um L2. Propostas não foram implementadas.
//
// **Technical Vision & Articulation (Performing at the next level)**
//
// Investigação e documentação de qualidade condizente com o esperado para o nível seguinte, conforme avaliação gerencial. Proposta de Governance do Metabase com ownership boundaries e naming conventions ([link](https://docs.google.com/document/d/1Us-EMqnvij_FP2xcGA6tC8Dfvwco8lzKZj9LdueSKY4/edit?tab=t.0#heading=h.tglo77yl0lf5)). Proposta de arquitetura modular de MVs para performance ([link](https://docs.google.com/document/d/14fBlSy6Yn5eT26c5yH0dG3vytnMZTwR3A4ZDS1kADcU/edit?tab=t.0)). Propostas em fase de documentação, sem implementação no ciclo. O valor está na capacidade de investigar, estruturar problema e documentar quando direcionado.
//
// **Influence & Collaboration (Meets expectations) / Discipline Contribution (Meets expectations)**
//
// Observou-se socialização das propostas com stakeholders e discussão em andamento sobre a reorganização do Metabase. Os artefatos produzidos incluem proposta de ownership boundaries e naming conventions que, se implementados, contribuiriam para práticas do time. Entretanto, nenhuma proposta foi implementada e não se identificou evidência de contribuição para padrões adotados, mentoria, hiring, ou melhoria de práticas em funcionamento — elementos esperados em Discipline Contribution conforme o career ladder. O valor registrado limita-se à investigação e documentação de qualidade condizente com o esperado para o nível seguinte, conforme avaliação gerencial.
#[derive(Component)]
pub struct SceneObjectsSpawned;

#[derive(Default)]
pub(crate) struct ModelValidationCache {
    by_model: HashMap<String, bool>,
    warned_models: HashSet<String>,
}

#[derive(Default)]
pub(crate) struct ProxyAssetCache {
    mesh: Option<Handle<Mesh>>,
    materials: HashMap<u32, Handle<StandardMaterial>>,
}

#[derive(Resource, Clone, Debug)]
pub struct SceneObjectDistanceCullingConfig {
    pub enabled: bool,
    pub max_distance: f32,
    pub(crate) max_distance_squared: f32,
}

impl Default for SceneObjectDistanceCullingConfig {
    fn default() -> Self {
        let max_distance = std::env::var(SCENE_OBJECT_CULL_DISTANCE_ENV)
            .ok()
            .and_then(|raw| raw.trim().parse::<f32>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(DEFAULT_SCENE_OBJECT_CULL_DISTANCE);
        let enabled = max_distance > 0.0;
        let max_distance = if enabled {
            max_distance
        } else {
            DEFAULT_SCENE_OBJECT_CULL_DISTANCE
        };

        Self {
            enabled,
            max_distance,
            max_distance_squared: max_distance * max_distance,
        }
    }
}

/// System to spawn scene objects once assets are loaded
pub fn spawn_scene_objects_when_ready(
    mut commands: Commands,
    assets: Res<RuntimeSceneAssets>,
    terrain_configs: Res<Assets<TerrainConfig>>,
    scene_objects_data: Res<Assets<SceneObjectsData>>,
    particle_defs: Res<Assets<ParticleDefinitions>>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut model_validation_cache: Local<ModelValidationCache>,
    mut proxy_assets: Local<ProxyAssetCache>,
    spawned_query: Query<&SceneObjectsSpawned>,
) {
    // Only spawn once
    if !spawned_query.is_empty() {
        return;
    }

    // Wait for assets to be loaded
    if !assets.loaded {
        return;
    }

    let Some(world) = assets.world.as_ref() else {
        return;
    };

    let Some(scene_data) = scene_objects_data.get(&world.scene_objects) else {
        return;
    };

    let Some(terrain_config) = terrain_configs.get(&world.terrain_config) else {
        return;
    };

    let Some(particle_definitions) = particle_defs.get(&assets.particle_defs) else {
        return;
    };

    let mirror_axis = world_mirror_axis();
    let map_max_x =
        (terrain_config.size.width.saturating_sub(1) as f32) * terrain_config.size.scale;
    let map_max_z =
        (terrain_config.size.depth.saturating_sub(1) as f32) * terrain_config.size.scale;

    let (object_defs, rotation_encoding, rotation_yaw_offset_degrees) = if scene_data
        .objects
        .is_empty()
    {
        warn!(
            "Scene object list is empty; falling back to placeholder login objects. For parity with C++ scene, provide EncTerrain<world>.obj and regenerate scene_objects.json"
        );
        (
            fallback_scene_objects(),
            SceneRotationEncoding::LegacySwizzledDegrees,
            0.0,
        )
    } else {
        let rotation_encoding = scene_data.metadata.rotation_encoding;
        let yaw_offset_degrees =
            scene_object_rotation_yaw_offset(rotation_encoding, &scene_data.metadata);
        (
            scene_data.objects.clone(),
            rotation_encoding,
            yaw_offset_degrees,
        )
    };

    if rotation_yaw_offset_degrees != 0.0 {
        info!(
            "Applying scene-object yaw offset of {:.1}° (encoding={:?})",
            rotation_yaw_offset_degrees, rotation_encoding
        );
    }

    info!("Spawning {} scene objects", object_defs.len());
    let spawn_started_at = Instant::now();

    // Spawn each object
    for object in &object_defs {
        spawn_scene_object(
            &mut commands,
            &asset_server,
            &mut meshes,
            &mut materials,
            &mut model_validation_cache,
            &mut proxy_assets,
            object,
            particle_definitions,
            rotation_encoding,
            rotation_yaw_offset_degrees,
            map_max_x,
            map_max_z,
            mirror_axis,
        );
    }

    // Mark as spawned
    commands.spawn((SceneObjectsSpawned, RuntimeSceneEntity));

    info!(
        "Scene objects spawned successfully in {} ms",
        spawn_started_at.elapsed().as_millis()
    );
}

/// Additional distance-based culling on top of Bevy frustum culling.
///
/// Controlled by `MU_SCENE_OBJECT_CULL_DISTANCE` (world units):
/// - `> 0`: enabled with provided distance
/// - `<= 0`: disabled
/// Threshold (squared) below which camera movement is ignored for culling recalc.
const CULLING_CAMERA_MOVE_THRESHOLD_SQ: f32 = 100.0; // 10 world units

pub fn apply_scene_object_distance_culling(
    config: Res<SceneObjectDistanceCullingConfig>,
    camera_query: Query<&Transform, With<Camera3d>>,
    mut scene_objects: Query<(&Transform, &mut Visibility), With<SceneObject>>,
    mut last_camera_pos: Local<Option<Vec3>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let camera_position = camera_transform.translation;

    if !config.enabled {
        for (_, mut visibility) in &mut scene_objects {
            if *visibility == Visibility::Hidden {
                *visibility = Visibility::Inherited;
            }
        }
        *last_camera_pos = Some(camera_position);
        return;
    }

    // Skip recalculation if camera hasn't moved significantly
    if let Some(prev) = *last_camera_pos {
        if prev.distance_squared(camera_position) < CULLING_CAMERA_MOVE_THRESHOLD_SQ {
            return;
        }
    }
    *last_camera_pos = Some(camera_position);

    for (object_transform, mut visibility) in &mut scene_objects {
        let distance_squared = object_transform
            .translation
            .distance_squared(camera_position);
        let target_visibility = if distance_squared <= config.max_distance_squared {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        if *visibility != target_visibility {
            *visibility = target_visibility;
        }
    }
}

/// Spawn a single scene object
fn spawn_scene_object(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    model_validation_cache: &mut ModelValidationCache,
    proxy_assets: &mut ProxyAssetCache,
    object_def: &SceneObjectDef,
    particle_defs: &ParticleDefinitions,
    rotation_encoding: SceneRotationEncoding,
    rotation_yaw_offset_degrees: f32,
    map_max_x: f32,
    map_max_z: f32,
    mirror_axis: WorldMirrorAxis,
) {
    let position = mirror_map_position_with_axis(
        Vec3::from(object_def.position),
        map_max_x,
        map_max_z,
        mirror_axis,
    );
    let rotation = scene_object_rotation_to_quat(
        apply_scene_object_yaw_offset(
            object_def.rotation,
            rotation_encoding,
            rotation_yaw_offset_degrees,
        ),
        rotation_encoding,
    );
    let scale = Vec3::from(object_def.scale);

    let mut entity_cmd = commands.spawn((
        RuntimeSceneEntity,
        SceneObject,
        SceneObjectKind(object_def.object_type),
        SpatialBundle {
            transform: Transform {
                translation: position,
                rotation,
                scale,
            },
            ..default()
        },
    ));

    if object_def.model.is_empty() {
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    } else if matches!(object_def.properties.model_renderable, Some(false)) {
        if model_validation_cache
            .warned_models
            .insert(object_def.model.clone())
        {
            let reason = object_def
                .properties
                .model_validation_reason
                .as_deref()
                .unwrap_or("precomputed conversion validation failed");
            warn!(
                "Model '{}' marked as non-renderable by conversion pipeline ({}). Using proxy mesh.",
                object_def.model, reason
            );
        }
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    } else if matches!(object_def.properties.model_renderable, Some(true))
        || is_renderable_model(&object_def.model, model_validation_cache)
    {
        let scene_path = normalize_scene_path(&object_def.model);
        let animation_speed =
            scene_object_animation_speed(&object_def.model, object_def.properties.animation_speed);
        let animation_source = glb_asset_path_from_scene_path(&scene_path).map(|glb_asset_path| {
            let gltf_handle: Handle<Gltf> = asset_server.load(glb_asset_path.clone());
            SceneObjectAnimationSource {
                glb_asset_path,
                gltf_handle,
                playback_speed: animation_speed,
            }
        });
        if let Some(source) = animation_source.clone() {
            entity_cmd.insert(source);
        }
        let scene: Handle<Scene> = asset_server.load(scene_path);
        entity_cmd.with_children(|parent| {
            parent.spawn(SceneBundle {
                scene: SceneRoot(scene),
                ..default()
            });
        });
    } else {
        spawn_model_proxy(
            &mut entity_cmd,
            meshes,
            materials,
            proxy_assets,
            object_def.object_type,
        );
    }

    // Add particle emitter if specified
    if let Some(emitter_type) = &object_def.properties.particle_emitter {
        if let Some(emitter_def) = particle_defs.emitters.get(emitter_type) {
            add_particle_emitter(&mut entity_cmd, emitter_def);
        } else {
            warn!(
                "Particle emitter '{}' not found for object '{}'",
                emitter_type, object_def.id
            );
        }
    }

    // Add dynamic light if specified
    if let Some(light_color) = object_def.properties.light_color {
        add_dynamic_light(&mut entity_cmd, &object_def.properties, light_color);
    }

    // Add boid spawner if object type is 62 (eagle spawn point)
    if object_def.object_type == 62 {
        spawn_boid(commands, position, &object_def.properties);
    }
}

fn scene_object_rotation_yaw_offset(
    rotation_encoding: SceneRotationEncoding,
    metadata: &SceneObjectsMetadata,
) -> f32 {
    if rotation_encoding != SceneRotationEncoding::MuAnglesDegrees {
        return 0.0;
    }

    if let Some(explicit_offset) = metadata.rotation_yaw_offset_degrees {
        if explicit_offset.is_finite() {
            return explicit_offset;
        }
    }

    if metadata.generated_placeholder || metadata.reason.is_some() {
        return 0.0;
    }

    std::env::var(SCENE_OBJECT_YAW_OFFSET_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(DEFAULT_MU_SCENE_OBJECT_YAW_OFFSET_DEGREES)
}

fn apply_scene_object_yaw_offset(
    mut rotation: [f32; 3],
    rotation_encoding: SceneRotationEncoding,
    yaw_offset_degrees: f32,
) -> [f32; 3] {
    if rotation_encoding == SceneRotationEncoding::MuAnglesDegrees
        && yaw_offset_degrees.is_finite()
        && yaw_offset_degrees != 0.0
    {
        rotation[2] += yaw_offset_degrees;
    }
    rotation
}

fn scene_object_animation_speed(model_path: &str, configured_speed: Option<f32>) -> f32 {
    if let Some(speed) = configured_speed {
        if speed.is_finite() && speed > 0.0 {
            return speed;
        }
    }

    let normalized = model_path.to_ascii_lowercase();
    if normalized.contains("/monster") || normalized.contains("/npc") {
        return DEFAULT_NPC_MONSTER_ANIMATION_SPEED;
    }

    DEFAULT_SCENE_OBJECT_ANIMATION_SPEED
}

fn is_renderable_model(model_path: &str, cache: &mut ModelValidationCache) -> bool {
    if let Some(is_renderable) = cache.by_model.get(model_path) {
        return *is_renderable;
    }

    let validation = validate_model_asset(model_path);
    let is_renderable = validation.is_ok();
    if !is_renderable && cache.warned_models.insert(model_path.to_string()) {
        if let Err(reason) = &validation {
            warn!(
                "Model '{}' is not renderable ({}). Using proxy mesh.",
                model_path, reason
            );
        } else {
            warn!(
                "Model '{}' is not renderable. Using proxy mesh.",
                model_path
            );
        }
    }
    cache.by_model.insert(model_path.to_string(), is_renderable);
    is_renderable
}

fn validate_model_asset(model_path: &str) -> Result<(), String> {
    let normalized_model_path = model_path.split('#').next().unwrap_or(model_path);
    let full_path = asset_disk_path(normalized_model_path);
    if !full_path.exists() {
        return Err(format!("asset path not found: {}", full_path.display()));
    }

    match full_path
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some(ext) if ext.eq_ignore_ascii_case("glb") => validate_glb_asset(&full_path),
        Some(ext) if ext.eq_ignore_ascii_case("gltf") => {
            Err("gltf is no longer supported; use glb".to_string())
        }
        _ => Ok(()),
    }
}

fn asset_disk_path(asset_path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("assets")
        .join(asset_path)
}

fn validate_glb_asset(path: &Path) -> Result<(), String> {
    let size = fs::metadata(path)
        .map_err(|error| format!("failed to stat GLB '{}': {}", path.display(), error))?
        .len();
    if size < 128 {
        return Err(format!("GLB payload too small ({} bytes)", size));
    }
    Ok(())
}

fn spawn_model_proxy(
    entity_cmd: &mut EntityCommands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    proxy_assets: &mut ProxyAssetCache,
    object_type: u32,
) {
    let mesh_handle = proxy_assets
        .mesh
        .get_or_insert_with(|| meshes.add(Mesh::from(Cuboid::new(260.0, 420.0, 260.0))))
        .clone();
    let material_handle = proxy_assets
        .materials
        .entry(object_type)
        .or_insert_with(|| {
            let hue = (object_type % 360) as f32;
            materials.add(StandardMaterial {
                base_color: Color::hsl(hue, 0.75, 0.62),
                perceptual_roughness: 0.9,
                metallic: 0.0,
                unlit: true,
                ..default()
            })
        })
        .clone();

    entity_cmd.with_children(|parent| {
        parent.spawn(PbrBundle {
            mesh: Mesh3d(mesh_handle),
            material: MeshMaterial3d(material_handle),
            transform: Transform::from_xyz(0.0, 210.0, 0.0),
            ..default()
        });
    });
}

/// Add particle emitter component to entity
fn add_particle_emitter(entity_cmd: &mut EntityCommands, emitter_def: &ParticleEmitterDef) {
    if let Some(emitter) = particle_emitter_from_definition(emitter_def) {
        entity_cmd.insert(emitter);
    }
}

/// Add dynamic light component to entity
fn add_dynamic_light(
    entity_cmd: &mut EntityCommands,
    properties: &ObjectProperties,
    light_color: [f32; 3],
) {
    entity_cmd.insert(DynamicLight {
        color: Color::srgb(light_color[0], light_color[1], light_color[2]),
        intensity: properties.light_intensity.unwrap_or(1.0),
        range: properties.light_range.unwrap_or(5.0),
        flicker: Some(FlickerParams {
            min_intensity: 0.3,
            max_intensity: 0.7,
            speed: 2.0,
        }),
    });
}

/// Spawn a boid (eagle) at the object location
fn spawn_boid(commands: &mut Commands, spawn_point: Vec3, properties: &ObjectProperties) {
    let flight_radius = properties.flight_radius.unwrap_or(30.0);

    commands.spawn((
        RuntimeSceneEntity,
        SpatialBundle {
            transform: Transform::from_translation(spawn_point),
            ..default()
        },
        Boid {
            spawn_point,
            animation_timer: Timer::from_seconds(0.1, TimerMode::Repeating),
        },
        BoidFlightPattern {
            pattern_type: FlightPattern::Circular {
                radius: flight_radius,
                speed: 0.3,
            },
            time: 0.0,
        },
    ));
}

fn fallback_scene_objects() -> Vec<SceneObjectDef> {
    vec![
        SceneObjectDef {
            id: "fallback_gate_1".to_string(),
            object_type: 113,
            model: "data/Object74/Object114.glb".to_string(),
            position: [24_000.0, 170.0, 2_600.0],
            rotation: [0.0, 125.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_tower_1".to_string(),
            object_type: 122,
            model: "data/Object74/Object123.glb".to_string(),
            position: [23_200.0, 170.0, 3_200.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_wall_1".to_string(),
            object_type: 126,
            model: "data/Object74/Object127.glb".to_string(),
            position: [22_100.0, 170.0, 4_300.0],
            rotation: [0.0, 180.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_hall_2".to_string(),
            object_type: 139,
            model: "data/Object74/Object140.glb".to_string(),
            position: [20_900.0, 170.0, 4_900.0],
            rotation: [0.0, 210.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_hall_3".to_string(),
            object_type: 145,
            model: "data/Object74/Object146.glb".to_string(),
            position: [19_900.0, 170.0, 5_100.0],
            rotation: [0.0, 235.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_wall_2".to_string(),
            object_type: 148,
            model: "data/Object74/Object149.glb".to_string(),
            position: [20_600.0, 170.0, 2_300.0],
            rotation: [0.0, 30.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_arch_1".to_string(),
            object_type: 70,
            model: "data/Object74/Object71.glb".to_string(),
            position: [23_100.0, 170.0, 1_900.0],
            rotation: [0.0, 95.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties::default(),
        },
        SceneObjectDef {
            id: "fallback_fire_1".to_string(),
            object_type: 103,
            model: "data/Object74/Object104.glb".to_string(),
            position: [21_100.0, 170.0, 2_700.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties {
                particle_emitter: Some("fire_orange".to_string()),
                light_color: Some([1.0, 0.6, 0.2]),
                light_intensity: Some(300.0),
                light_range: Some(350.0),
                ..Default::default()
            },
        },
        SceneObjectDef {
            id: "fallback_cloud_1".to_string(),
            object_type: 60,
            model: "data/Object74/Object63.glb".to_string(),
            position: [20_900.0, 260.0, 3_000.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [0.7, 0.7, 0.7],
            properties: ObjectProperties {
                particle_emitter: Some("cloud".to_string()),
                ..Default::default()
            },
        },
        SceneObjectDef {
            id: "fallback_eagle_spawn".to_string(),
            object_type: 62,
            model: "data/Object74/Object63.glb".to_string(),
            position: [20_800.0, 300.0, 3_400.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
            properties: ObjectProperties {
                flight_radius: Some(900.0),
                flight_height: Some(250.0),
                ..Default::default()
            },
        },
    ]
}

pub fn apply_legacy_gltf_material_overrides(
    mut commands: Commands,
    mut legacy_materials: ResMut<Assets<LegacyAdditiveMaterial>>,
    materials: Res<Assets<StandardMaterial>>,
    query: Query<
        (
            Entity,
            &MeshMaterial3d<StandardMaterial>,
            &GltfMaterialExtras,
        ),
        Added<GltfMaterialExtras>,
    >,
) {
    for (entity, material_handle, extras) in &query {
        let parsed = serde_json::from_str::<Value>(&extras.value);
        let Ok(payload) = parsed else {
            continue;
        };

        let Some(blend_mode) = payload.get("mu_legacy_blend_mode").and_then(Value::as_str) else {
            continue;
        };

        if blend_mode != "additive" {
            continue;
        }

        let Some(material) = materials.get(&material_handle.0) else {
            continue;
        };

        let mut legacy_material = legacy_additive_from_standard(material);
        let intensity = legacy_additive_intensity_from_extras(&payload);
        legacy_material.params.intensity = intensity;
        let has_texture = legacy_material.color_texture.is_some();
        let legacy_material_handle = legacy_materials.add(legacy_material);

        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(legacy_material_handle));

        debug!(
            "Applied legacy additive override (object={:?}/{:?}) texture={} intensity={:.2} material=LegacyAdditiveMaterial",
            payload
                .get("mu_legacy_object_dir")
                .and_then(|value| value.as_i64()),
            payload
                .get("mu_legacy_object_model")
                .and_then(|value| value.as_i64()),
            has_texture,
            intensity,
        );
    }
}

/// Fix materials for scene objects loaded from GLB files.
///
/// MU BMD models are converted to GLB with single-sided geometry. When the camera
/// views a wall from the "back" side, standard backface culling discards those
/// triangles, making the wall invisible. This system sets `double_sided: true` on
/// all StandardMaterials that belong to scene object entities (descendants of
/// entities with the `SceneObject` component).
///
/// Additionally, when `MU_SCENE_OBJECTS_UNLIT` is enabled (default: true), sets
/// `unlit: true` so textures display at full brightness — matching the original
/// MU Online rendering where objects were not affected by PBR lighting.
pub fn fix_scene_object_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    new_material_query: Query<
        (Entity, &MeshMaterial3d<StandardMaterial>),
        Added<MeshMaterial3d<StandardMaterial>>,
    >,
    parent_query: Query<&ChildOf>,
    scene_object_query: Query<(), With<SceneObject>>,
) {
    let unlit = scene_objects_unlit();

    for (entity, mat_handle) in &new_material_query {
        // Walk up the parent chain to check if this entity is a descendant of a SceneObject
        let mut current = entity;
        let mut is_scene_object_descendant = false;
        for _ in 0..10 {
            if scene_object_query.get(current).is_ok() {
                is_scene_object_descendant = true;
                break;
            }
            match parent_query.get(current) {
                Ok(parent) => current = parent.parent(),
                Err(_) => break,
            }
        }

        if !is_scene_object_descendant {
            continue;
        }

        let Some(material) = materials.get_mut(&mat_handle.0) else {
            continue;
        };

        if !material.double_sided {
            material.double_sided = true;
            material.cull_mode = None;
        }
        if unlit && !material.unlit {
            material.unlit = true;
        }
    }
}

fn normalize_scene_path(model_path: &str) -> String {
    if model_path.contains('#') {
        return model_path.to_string();
    }

    if model_path.ends_with(".glb") {
        format!("{model_path}#Scene0")
    } else {
        model_path.to_string()
    }
}

fn glb_asset_path_from_scene_path(scene_path: &str) -> Option<String> {
    let base_path = scene_path.split('#').next().unwrap_or(scene_path).trim();
    if base_path.to_ascii_lowercase().ends_with(".glb") {
        Some(base_path.to_string())
    } else {
        None
    }
}
