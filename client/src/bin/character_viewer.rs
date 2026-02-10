use bevy::asset::AssetPlugin;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::gltf::Gltf;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::pbr::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, ShadowFilteringMethod};
use bevy::render::texture::{ImageAddressMode, ImageSampler, ImageSamplerDescriptor};
use bevy::window::WindowResolution;
use bevy_egui::{EguiContexts, EguiPlugin, egui};
use std::collections::HashMap;
use std::time::Duration;

// Import character modules from the client crate.
// Since this is a binary in the same package, we access via `client::`.
mod character_imports {
    // Re-export what we need — we inline the types to avoid crate linkage issues
    // in a bin target within the same package.
}

// ============================================================================
// Character system types (inlined for bin target)
// ============================================================================

mod character {
    pub mod types {
        use bevy::prelude::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum CharacterClass {
            DarkKnight,
            DarkWizard,
            FairyElf,
            MagicGladiator,
            DarkLord,
            Summoner,
            RageFighter,
        }

        impl CharacterClass {
            pub const ALL: &'static [CharacterClass] = &[
                CharacterClass::DarkKnight,
                CharacterClass::DarkWizard,
                CharacterClass::FairyElf,
                CharacterClass::MagicGladiator,
                CharacterClass::DarkLord,
                CharacterClass::Summoner,
                CharacterClass::RageFighter,
            ];

            pub fn name(&self) -> &'static str {
                match self {
                    CharacterClass::DarkKnight => "DarkKnight",
                    CharacterClass::DarkWizard => "DarkWizard",
                    CharacterClass::FairyElf => "FairyElf",
                    CharacterClass::MagicGladiator => "MagicGladiator",
                    CharacterClass::DarkLord => "DarkLord",
                    CharacterClass::Summoner => "Summoner",
                    CharacterClass::RageFighter => "RageFighter",
                }
            }

            pub fn class_id(&self) -> u8 {
                match self {
                    CharacterClass::DarkWizard => 1,
                    CharacterClass::DarkKnight => 2,
                    CharacterClass::FairyElf => 3,
                    CharacterClass::MagicGladiator => 4,
                    CharacterClass::DarkLord => 5,
                    CharacterClass::Summoner => 6,
                    CharacterClass::RageFighter => 7,
                }
            }

            pub fn body_type(&self) -> BodyType {
                match self {
                    CharacterClass::DarkKnight
                    | CharacterClass::DarkWizard
                    | CharacterClass::MagicGladiator
                    | CharacterClass::DarkLord => BodyType::Male,
                    CharacterClass::FairyElf | CharacterClass::Summoner => BodyType::Elf,
                    CharacterClass::RageFighter => BodyType::Monk,
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BodyType {
            Male,
            Elf,
            Monk,
        }

        impl BodyType {
            pub fn slug(&self) -> &'static str {
                match self {
                    BodyType::Male => "male",
                    BodyType::Elf => "elf",
                    BodyType::Monk => "monk",
                }
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum BodySlot {
            Helm,
            Armor,
            Pants,
            Gloves,
            Boots,
        }

        impl BodySlot {
            pub fn prefix(&self) -> &'static str {
                match self {
                    BodySlot::Helm => "helm",
                    BodySlot::Armor => "armor",
                    BodySlot::Pants => "pant",
                    BodySlot::Gloves => "glove",
                    BodySlot::Boots => "boot",
                }
            }

            pub fn slots_for(body_type: BodyType) -> &'static [BodySlot] {
                match body_type {
                    BodyType::Monk => &[
                        BodySlot::Helm,
                        BodySlot::Armor,
                        BodySlot::Pants,
                        BodySlot::Boots,
                    ],
                    _ => &[
                        BodySlot::Helm,
                        BodySlot::Armor,
                        BodySlot::Pants,
                        BodySlot::Gloves,
                        BodySlot::Boots,
                    ],
                }
            }
        }

        #[derive(Component)]
        pub struct BodyPartMarker {
            pub slot: BodySlot,
        }

        #[derive(Component)]
        pub struct CharacterRoot;
    }

    pub mod controller {
        use bevy::prelude::*;

        #[derive(Component)]
        pub struct CharacterController {
            pub class: super::types::CharacterClass,
            pub state: CharacterState,
        }

        #[derive(Component)]
        pub struct CharacterAnimState {
            pub current_action: usize,
            pub playback_speed: f32,
        }

        #[derive(Debug, Clone)]
        pub enum CharacterState {
            Idle,
            Walking { target: Vec3 },
            Running { target: Vec3 },
        }
    }

    pub mod animations {
        /// Player animation action indices matching the C++ `_enum.h` PLAYER_* constants.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(u16)]
        pub enum PlayerAction {
            Set = 0,
            StopMale = 1,
            StopFemale = 2,
            StopSummoner = 3,
            StopSword = 4,
            StopTwoHandSword = 5,
            StopSpear = 6,
            StopScythe = 7,
            StopBow = 8,
            StopCrossbow = 9,
            StopWand = 10,
            StopFly = 11,
            StopFlyCrossbow = 12,
            StopRide = 13,
            StopRideWeapon = 14,
            WalkMale = 15,
            WalkFemale = 16,
            WalkSword = 17,
            WalkTwoHandSword = 18,
            WalkSpear = 19,
            WalkScythe = 20,
            WalkBow = 21,
            WalkCrossbow = 22,
            WalkWand = 23,
            WalkSwim = 24,
            Run = 25,
            RunSword = 26,
            RunTwoSword = 27,
            RunTwoHandSword = 28,
            RunSpear = 29,
            RunBow = 30,
            RunCrossbow = 31,
            RunWand = 32,
            RunSwim = 33,
            Fly = 34,
            FlyCrossbow = 35,
            RunRide = 36,
            RunRideWeapon = 37,
            AttackFist = 38,
            AttackSwordRight1 = 39,
            AttackSwordRight2 = 40,
            AttackSwordLeft1 = 41,
            AttackSwordLeft2 = 42,
            AttackTwoHandSword1 = 43,
            AttackTwoHandSword2 = 44,
            AttackTwoHandSword3 = 45,
            AttackSpear1 = 46,
            AttackScythe1 = 47,
            AttackScythe2 = 48,
            AttackScythe3 = 49,
            AttackBow = 50,
            AttackCrossbow = 51,
            AttackFlyBow = 52,
            AttackFlyCrossbow = 53,
            AttackRideSword = 54,
            AttackRideTwoHandSword = 55,
            AttackRideSpear = 56,
            AttackRideScythe = 57,
            AttackRideBow = 58,
            AttackRideCrossbow = 59,
            AttackSkillSword1 = 60,
            AttackSkillSword2 = 61,
            AttackSkillSword3 = 62,
            AttackSkillSword4 = 63,
            AttackSkillSword5 = 64,
            AttackSkillWheel = 65,
            AttackSkillFuryStrike = 66,
            SkillVitality = 67,
            SkillRider = 68,
            SkillRiderFly = 69,
            AttackSkillSpear = 70,
            AttackOneToOne = 71,
            SkillHellBegin = 72,
            SkillHellStart = 73,
            FlyRide = 74,
            FlyRideWeapon = 75,
            DarklordStand = 76,
            DarklordWalk = 77,
            StopRideHorse = 78,
            RunRideHorse = 79,
            AttackStrike = 80,
            AttackTeleport = 81,
            AttackRideStrike = 82,
            AttackRideTeleport = 83,
            AttackRideHorseSword = 84,
            AttackRideAttackFlash = 85,
            AttackRideAttackMagic = 86,
            AttackDarkhorse = 87,
            Idle1Darkhorse = 88,
            Idle2Darkhorse = 89,
            FenrirAttack = 90,
            FenrirAttackDarklordAqua = 91,
            FenrirAttackDarklordStrike = 92,
            FenrirAttackDarklordSword = 93,
            FenrirAttackDarklordTeleport = 94,
            FenrirAttackDarklordFlash = 95,
            FenrirAttackTwoSword = 96,
            FenrirAttackMagic = 97,
            FenrirAttackCrossbow = 98,
            FenrirAttackSpear = 99,
            FenrirAttackOneSword = 100,
            FenrirAttackBow = 101,
            FenrirSkill = 102,
            FenrirSkillTwoSword = 103,
            FenrirSkillOneRight = 104,
            FenrirSkillOneLeft = 105,
            FenrirDamage = 106,
            FenrirDamageTwoSword = 107,
            FenrirDamageOneRight = 108,
            FenrirDamageOneLeft = 109,
            FenrirRun = 110,
            FenrirRunTwoSword = 111,
            FenrirRunOneRight = 112,
            FenrirRunOneLeft = 113,
            FenrirRunMagom = 114,
            FenrirRunTwoSwordMagom = 115,
            FenrirRunOneRightMagom = 116,
            FenrirRunOneLeftMagom = 117,
            FenrirRunElf = 118,
            FenrirRunTwoSwordElf = 119,
            FenrirRunOneRightElf = 120,
            FenrirRunOneLeftElf = 121,
            FenrirStand = 122,
            FenrirStandTwoSword = 123,
            FenrirStandOneRight = 124,
            FenrirStandOneLeft = 125,
            FenrirWalk = 126,
            FenrirWalkTwoSword = 127,
            FenrirWalkOneRight = 128,
            FenrirWalkOneLeft = 129,
            AttackBowUp = 130,
            AttackCrossbowUp = 131,
            AttackFlyBowUp = 132,
            AttackFlyCrossbowUp = 133,
            AttackRideBowUp = 134,
            AttackRideCrossbowUp = 135,
            AttackOneFlash = 136,
            AttackRush = 137,
            AttackDeathCannon = 138,
            AttackRemoval = 139,
            AttackStun = 140,
            HighShock = 141,
            StopTwoHandSwordTwo = 142,
            WalkTwoHandSwordTwo = 143,
            RunTwoHandSwordTwo = 144,
            AttackTwoHandSwordTwo = 145,
            SkillHand1 = 146,
            SkillHand2 = 147,
            SkillWeapon1 = 148,
            SkillWeapon2 = 149,
            SkillElf1 = 150,
            SkillTeleport = 151,
            SkillFlash = 152,
            SkillInferno = 153,
            SkillHell = 154,
            RideSkill = 155,
            SkillSleep = 156,
            SkillSleepUni = 157,
            SkillSleepDino = 158,
            SkillSleepFenrir = 159,
            SkillChainLightning = 160,
            SkillChainLightningUni = 161,
            SkillChainLightningDino = 162,
            SkillChainLightningFenrir = 163,
            SkillLightningOrb = 164,
            SkillLightningOrbUni = 165,
            SkillLightningOrbDino = 166,
            SkillLightningOrbFenrir = 167,
            SkillDrainLife = 168,
            SkillDrainLifeUni = 169,
            SkillDrainLifeDino = 170,
            SkillDrainLifeFenrir = 171,
            SkillSummon = 172,
            SkillSummonUni = 173,
            SkillSummonDino = 174,
            SkillSummonFenrir = 175,
            SkillBlowOfDestruction = 176,
            SkillSwellOfMp = 177,
            SkillMultishotBowStand = 178,
            SkillMultishotBowFlying = 179,
            SkillMultishotCrossbowStand = 180,
            SkillMultishotCrossbowFlying = 181,
            SkillRecovery = 182,
            SkillGiganticstorm = 183,
            SkillFlamestrike = 184,
            SkillLightningShock = 185,
            SkillGiganticstormUni = 186,
            SkillGiganticstormDino = 187,
            SkillGiganticstormFenrir = 188,
            AttackSkillWheelUni = 189,
            AttackSkillWheelDino = 190,
            AttackSkillWheelFenrir = 191,
            Defense1 = 192,
            Greeting1 = 193,
            GreetingFemale1 = 194,
            Goodbye1 = 195,
            GoodbyeFemale1 = 196,
            Clap1 = 197,
            ClapFemale1 = 198,
            Cheer1 = 199,
            CheerFemale1 = 200,
            Direction1 = 201,
            DirectionFemale1 = 202,
            Gesture1 = 203,
            GestureFemale1 = 204,
            Unknown1 = 205,
            UnknownFemale1 = 206,
            Cry1 = 207,
            CryFemale1 = 208,
            Awkward1 = 209,
            AwkwardFemale1 = 210,
            See1 = 211,
            SeeFemale1 = 212,
            Win1 = 213,
            WinFemale1 = 214,
            Smile1 = 215,
            SmileFemale1 = 216,
            Sleep1 = 217,
            SleepFemale1 = 218,
            Cold1 = 219,
            ColdFemale1 = 220,
            Again1 = 221,
            AgainFemale1 = 222,
            Respect1 = 223,
            Salute1 = 224,
            Scissors = 225,
            Rock = 226,
            Paper = 227,
            Hustle = 228,
            Provocation = 229,
            LookAround = 230,
            Cheers = 231,
            Rush1 = 232,
            ComeUp = 233,
            Shock = 234,
            Die1 = 235,
            Die2 = 236,
            Sit1 = 237,
            Sit2 = 238,
            SitFemale1 = 239,
            SitFemale2 = 240,
            Healing1 = 241,
            HealingFemale1 = 242,
            Pose1 = 243,
            PoseFemale1 = 244,
            Jack1 = 245,
            Jack2 = 246,
            Santa1 = 247,
            Santa2 = 248,
            ChangeUp = 249,
            RecoverSkill = 250,
            SkillThrust = 251,
            SkillStamp = 252,
            SkillGiantswing = 253,
            SkillDarksideReady = 254,
            SkillDarksideAttack = 255,
            SkillDragonkick = 256,
            SkillDragonlore = 257,
            SkillAttUpOurforces = 258,
            SkillHpUpOurforces = 259,
            RageUniAttack = 260,
            RageUniAttackOneRight = 261,
            RageUniRun = 262,
            RageUniRunOneRight = 263,
            RageUniStopOneRight = 264,
            RageFenrir = 265,
            RageFenrirTwoSword = 266,
            RageFenrirOneRight = 267,
            RageFenrirOneLeft = 268,
            RageFenrirWalk = 269,
            RageFenrirWalkOneRight = 270,
            RageFenrirWalkOneLeft = 271,
            RageFenrirWalkTwoSword = 272,
            RageFenrirRun = 273,
            RageFenrirRunTwoSword = 274,
            RageFenrirRunOneRight = 275,
            RageFenrirRunOneLeft = 276,
            RageFenrirStand = 277,
            RageFenrirStandTwoSword = 278,
            RageFenrirStandOneRight = 279,
            RageFenrirStandOneLeft = 280,
            RageFenrirDamage = 281,
            RageFenrirDamageTwoSword = 282,
            RageFenrirDamageOneRight = 283,
            RageFenrirDamageOneLeft = 284,
            RageFenrirAttackRight = 285,
            StopRagefighter = 286,
        }

        impl PlayerAction {
            pub fn from_index(i: usize) -> Option<PlayerAction> {
                if i > 286 {
                    return None;
                }
                Some(unsafe { std::mem::transmute::<u16, PlayerAction>(i as u16) })
            }

            pub fn name(&self) -> &'static str {
                // Return the variant name as a short string
                // Using debug formatting trimmed to just the variant name
                match self {
                    Self::Set => "Set",
                    Self::StopMale => "StopMale",
                    Self::StopFemale => "StopFemale",
                    Self::StopSummoner => "StopSummoner",
                    Self::StopSword => "StopSword",
                    Self::StopTwoHandSword => "StopTwoHandSword",
                    Self::StopSpear => "StopSpear",
                    Self::StopScythe => "StopScythe",
                    Self::StopBow => "StopBow",
                    Self::StopCrossbow => "StopCrossbow",
                    Self::StopWand => "StopWand",
                    Self::StopFly => "StopFly",
                    Self::StopFlyCrossbow => "StopFlyCrossbow",
                    Self::StopRide => "StopRide",
                    Self::StopRideWeapon => "StopRideWeapon",
                    Self::WalkMale => "WalkMale",
                    Self::WalkFemale => "WalkFemale",
                    Self::WalkSword => "WalkSword",
                    Self::WalkTwoHandSword => "WalkTwoHandSword",
                    Self::WalkSpear => "WalkSpear",
                    Self::WalkScythe => "WalkScythe",
                    Self::WalkBow => "WalkBow",
                    Self::WalkCrossbow => "WalkCrossbow",
                    Self::WalkWand => "WalkWand",
                    Self::WalkSwim => "WalkSwim",
                    Self::Run => "Run",
                    Self::RunSword => "RunSword",
                    Self::RunTwoSword => "RunTwoSword",
                    Self::RunTwoHandSword => "RunTwoHandSword",
                    Self::RunSpear => "RunSpear",
                    Self::RunBow => "RunBow",
                    Self::RunCrossbow => "RunCrossbow",
                    Self::RunWand => "RunWand",
                    Self::RunSwim => "RunSwim",
                    Self::Fly => "Fly",
                    Self::FlyCrossbow => "FlyCrossbow",
                    Self::RunRide => "RunRide",
                    Self::RunRideWeapon => "RunRideWeapon",
                    Self::AttackFist => "AttackFist",
                    Self::AttackSwordRight1 => "AttackSwordRight1",
                    Self::AttackSwordRight2 => "AttackSwordRight2",
                    Self::AttackSwordLeft1 => "AttackSwordLeft1",
                    Self::AttackSwordLeft2 => "AttackSwordLeft2",
                    Self::AttackTwoHandSword1 => "AttackTwoHandSword1",
                    Self::AttackTwoHandSword2 => "AttackTwoHandSword2",
                    Self::AttackTwoHandSword3 => "AttackTwoHandSword3",
                    Self::AttackSpear1 => "AttackSpear1",
                    Self::AttackScythe1 => "AttackScythe1",
                    Self::AttackScythe2 => "AttackScythe2",
                    Self::AttackScythe3 => "AttackScythe3",
                    Self::AttackBow => "AttackBow",
                    Self::AttackCrossbow => "AttackCrossbow",
                    Self::AttackFlyBow => "AttackFlyBow",
                    Self::AttackFlyCrossbow => "AttackFlyCrossbow",
                    Self::AttackRideSword => "AttackRideSword",
                    Self::AttackRideTwoHandSword => "AttackRideTwoHandSword",
                    Self::AttackRideSpear => "AttackRideSpear",
                    Self::AttackRideScythe => "AttackRideScythe",
                    Self::AttackRideBow => "AttackRideBow",
                    Self::AttackRideCrossbow => "AttackRideCrossbow",
                    Self::AttackSkillSword1 => "AttackSkillSword1",
                    Self::AttackSkillSword2 => "AttackSkillSword2",
                    Self::AttackSkillSword3 => "AttackSkillSword3",
                    Self::AttackSkillSword4 => "AttackSkillSword4",
                    Self::AttackSkillSword5 => "AttackSkillSword5",
                    Self::AttackSkillWheel => "AttackSkillWheel",
                    Self::AttackSkillFuryStrike => "AttackSkillFuryStrike",
                    Self::SkillVitality => "SkillVitality",
                    Self::SkillRider => "SkillRider",
                    Self::SkillRiderFly => "SkillRiderFly",
                    Self::AttackSkillSpear => "AttackSkillSpear",
                    Self::AttackOneToOne => "AttackOneToOne",
                    Self::SkillHellBegin => "SkillHellBegin",
                    Self::SkillHellStart => "SkillHellStart",
                    Self::FlyRide => "FlyRide",
                    Self::FlyRideWeapon => "FlyRideWeapon",
                    Self::DarklordStand => "DarklordStand",
                    Self::DarklordWalk => "DarklordWalk",
                    Self::StopRideHorse => "StopRideHorse",
                    Self::RunRideHorse => "RunRideHorse",
                    Self::AttackStrike => "AttackStrike",
                    Self::AttackTeleport => "AttackTeleport",
                    Self::AttackRideStrike => "AttackRideStrike",
                    Self::AttackRideTeleport => "AttackRideTeleport",
                    Self::AttackRideHorseSword => "AttackRideHorseSword",
                    Self::AttackRideAttackFlash => "AttackRideAttackFlash",
                    Self::AttackRideAttackMagic => "AttackRideAttackMagic",
                    Self::AttackDarkhorse => "AttackDarkhorse",
                    Self::Idle1Darkhorse => "Idle1Darkhorse",
                    Self::Idle2Darkhorse => "Idle2Darkhorse",
                    Self::FenrirAttack => "FenrirAttack",
                    Self::FenrirAttackDarklordAqua => "FenrirAttackDarklordAqua",
                    Self::FenrirAttackDarklordStrike => "FenrirAttackDarklordStrike",
                    Self::FenrirAttackDarklordSword => "FenrirAttackDarklordSword",
                    Self::FenrirAttackDarklordTeleport => "FenrirAttackDarklordTeleport",
                    Self::FenrirAttackDarklordFlash => "FenrirAttackDarklordFlash",
                    Self::FenrirAttackTwoSword => "FenrirAttackTwoSword",
                    Self::FenrirAttackMagic => "FenrirAttackMagic",
                    Self::FenrirAttackCrossbow => "FenrirAttackCrossbow",
                    Self::FenrirAttackSpear => "FenrirAttackSpear",
                    Self::FenrirAttackOneSword => "FenrirAttackOneSword",
                    Self::FenrirAttackBow => "FenrirAttackBow",
                    Self::FenrirSkill => "FenrirSkill",
                    Self::FenrirSkillTwoSword => "FenrirSkillTwoSword",
                    Self::FenrirSkillOneRight => "FenrirSkillOneRight",
                    Self::FenrirSkillOneLeft => "FenrirSkillOneLeft",
                    Self::FenrirDamage => "FenrirDamage",
                    Self::FenrirDamageTwoSword => "FenrirDamageTwoSword",
                    Self::FenrirDamageOneRight => "FenrirDamageOneRight",
                    Self::FenrirDamageOneLeft => "FenrirDamageOneLeft",
                    Self::FenrirRun => "FenrirRun",
                    Self::FenrirRunTwoSword => "FenrirRunTwoSword",
                    Self::FenrirRunOneRight => "FenrirRunOneRight",
                    Self::FenrirRunOneLeft => "FenrirRunOneLeft",
                    Self::FenrirRunMagom => "FenrirRunMagom",
                    Self::FenrirRunTwoSwordMagom => "FenrirRunTwoSwordMagom",
                    Self::FenrirRunOneRightMagom => "FenrirRunOneRightMagom",
                    Self::FenrirRunOneLeftMagom => "FenrirRunOneLeftMagom",
                    Self::FenrirRunElf => "FenrirRunElf",
                    Self::FenrirRunTwoSwordElf => "FenrirRunTwoSwordElf",
                    Self::FenrirRunOneRightElf => "FenrirRunOneRightElf",
                    Self::FenrirRunOneLeftElf => "FenrirRunOneLeftElf",
                    Self::FenrirStand => "FenrirStand",
                    Self::FenrirStandTwoSword => "FenrirStandTwoSword",
                    Self::FenrirStandOneRight => "FenrirStandOneRight",
                    Self::FenrirStandOneLeft => "FenrirStandOneLeft",
                    Self::FenrirWalk => "FenrirWalk",
                    Self::FenrirWalkTwoSword => "FenrirWalkTwoSword",
                    Self::FenrirWalkOneRight => "FenrirWalkOneRight",
                    Self::FenrirWalkOneLeft => "FenrirWalkOneLeft",
                    Self::AttackBowUp => "AttackBowUp",
                    Self::AttackCrossbowUp => "AttackCrossbowUp",
                    Self::AttackFlyBowUp => "AttackFlyBowUp",
                    Self::AttackFlyCrossbowUp => "AttackFlyCrossbowUp",
                    Self::AttackRideBowUp => "AttackRideBowUp",
                    Self::AttackRideCrossbowUp => "AttackRideCrossbowUp",
                    Self::AttackOneFlash => "AttackOneFlash",
                    Self::AttackRush => "AttackRush",
                    Self::AttackDeathCannon => "AttackDeathCannon",
                    Self::AttackRemoval => "AttackRemoval",
                    Self::AttackStun => "AttackStun",
                    Self::HighShock => "HighShock",
                    Self::StopTwoHandSwordTwo => "StopTwoHandSwordTwo",
                    Self::WalkTwoHandSwordTwo => "WalkTwoHandSwordTwo",
                    Self::RunTwoHandSwordTwo => "RunTwoHandSwordTwo",
                    Self::AttackTwoHandSwordTwo => "AttackTwoHandSwordTwo",
                    Self::SkillHand1 => "SkillHand1",
                    Self::SkillHand2 => "SkillHand2",
                    Self::SkillWeapon1 => "SkillWeapon1",
                    Self::SkillWeapon2 => "SkillWeapon2",
                    Self::SkillElf1 => "SkillElf1",
                    Self::SkillTeleport => "SkillTeleport",
                    Self::SkillFlash => "SkillFlash",
                    Self::SkillInferno => "SkillInferno",
                    Self::SkillHell => "SkillHell",
                    Self::RideSkill => "RideSkill",
                    Self::SkillSleep => "SkillSleep",
                    Self::SkillSleepUni => "SkillSleepUni",
                    Self::SkillSleepDino => "SkillSleepDino",
                    Self::SkillSleepFenrir => "SkillSleepFenrir",
                    Self::SkillChainLightning => "SkillChainLightning",
                    Self::SkillChainLightningUni => "SkillChainLightningUni",
                    Self::SkillChainLightningDino => "SkillChainLightningDino",
                    Self::SkillChainLightningFenrir => "SkillChainLightningFenrir",
                    Self::SkillLightningOrb => "SkillLightningOrb",
                    Self::SkillLightningOrbUni => "SkillLightningOrbUni",
                    Self::SkillLightningOrbDino => "SkillLightningOrbDino",
                    Self::SkillLightningOrbFenrir => "SkillLightningOrbFenrir",
                    Self::SkillDrainLife => "SkillDrainLife",
                    Self::SkillDrainLifeUni => "SkillDrainLifeUni",
                    Self::SkillDrainLifeDino => "SkillDrainLifeDino",
                    Self::SkillDrainLifeFenrir => "SkillDrainLifeFenrir",
                    Self::SkillSummon => "SkillSummon",
                    Self::SkillSummonUni => "SkillSummonUni",
                    Self::SkillSummonDino => "SkillSummonDino",
                    Self::SkillSummonFenrir => "SkillSummonFenrir",
                    Self::SkillBlowOfDestruction => "SkillBlowOfDestruction",
                    Self::SkillSwellOfMp => "SkillSwellOfMp",
                    Self::SkillMultishotBowStand => "SkillMultishotBowStand",
                    Self::SkillMultishotBowFlying => "SkillMultishotBowFlying",
                    Self::SkillMultishotCrossbowStand => "SkillMultishotCrossbowStand",
                    Self::SkillMultishotCrossbowFlying => "SkillMultishotCrossbowFlying",
                    Self::SkillRecovery => "SkillRecovery",
                    Self::SkillGiganticstorm => "SkillGiganticstorm",
                    Self::SkillFlamestrike => "SkillFlamestrike",
                    Self::SkillLightningShock => "SkillLightningShock",
                    Self::SkillGiganticstormUni => "SkillGiganticstormUni",
                    Self::SkillGiganticstormDino => "SkillGiganticstormDino",
                    Self::SkillGiganticstormFenrir => "SkillGiganticstormFenrir",
                    Self::AttackSkillWheelUni => "AttackSkillWheelUni",
                    Self::AttackSkillWheelDino => "AttackSkillWheelDino",
                    Self::AttackSkillWheelFenrir => "AttackSkillWheelFenrir",
                    Self::Defense1 => "Defense1",
                    Self::Greeting1 => "Greeting1",
                    Self::GreetingFemale1 => "GreetingFemale1",
                    Self::Goodbye1 => "Goodbye1",
                    Self::GoodbyeFemale1 => "GoodbyeFemale1",
                    Self::Clap1 => "Clap1",
                    Self::ClapFemale1 => "ClapFemale1",
                    Self::Cheer1 => "Cheer1",
                    Self::CheerFemale1 => "CheerFemale1",
                    Self::Direction1 => "Direction1",
                    Self::DirectionFemale1 => "DirectionFemale1",
                    Self::Gesture1 => "Gesture1",
                    Self::GestureFemale1 => "GestureFemale1",
                    Self::Unknown1 => "Unknown1",
                    Self::UnknownFemale1 => "UnknownFemale1",
                    Self::Cry1 => "Cry1",
                    Self::CryFemale1 => "CryFemale1",
                    Self::Awkward1 => "Awkward1",
                    Self::AwkwardFemale1 => "AwkwardFemale1",
                    Self::See1 => "See1",
                    Self::SeeFemale1 => "SeeFemale1",
                    Self::Win1 => "Win1",
                    Self::WinFemale1 => "WinFemale1",
                    Self::Smile1 => "Smile1",
                    Self::SmileFemale1 => "SmileFemale1",
                    Self::Sleep1 => "Sleep1",
                    Self::SleepFemale1 => "SleepFemale1",
                    Self::Cold1 => "Cold1",
                    Self::ColdFemale1 => "ColdFemale1",
                    Self::Again1 => "Again1",
                    Self::AgainFemale1 => "AgainFemale1",
                    Self::Respect1 => "Respect1",
                    Self::Salute1 => "Salute1",
                    Self::Scissors => "Scissors",
                    Self::Rock => "Rock",
                    Self::Paper => "Paper",
                    Self::Hustle => "Hustle",
                    Self::Provocation => "Provocation",
                    Self::LookAround => "LookAround",
                    Self::Cheers => "Cheers",
                    Self::Rush1 => "Rush1",
                    Self::ComeUp => "ComeUp",
                    Self::Shock => "Shock",
                    Self::Die1 => "Die1",
                    Self::Die2 => "Die2",
                    Self::Sit1 => "Sit1",
                    Self::Sit2 => "Sit2",
                    Self::SitFemale1 => "SitFemale1",
                    Self::SitFemale2 => "SitFemale2",
                    Self::Healing1 => "Healing1",
                    Self::HealingFemale1 => "HealingFemale1",
                    Self::Pose1 => "Pose1",
                    Self::PoseFemale1 => "PoseFemale1",
                    Self::Jack1 => "Jack1",
                    Self::Jack2 => "Jack2",
                    Self::Santa1 => "Santa1",
                    Self::Santa2 => "Santa2",
                    Self::ChangeUp => "ChangeUp",
                    Self::RecoverSkill => "RecoverSkill",
                    Self::SkillThrust => "SkillThrust",
                    Self::SkillStamp => "SkillStamp",
                    Self::SkillGiantswing => "SkillGiantswing",
                    Self::SkillDarksideReady => "SkillDarksideReady",
                    Self::SkillDarksideAttack => "SkillDarksideAttack",
                    Self::SkillDragonkick => "SkillDragonkick",
                    Self::SkillDragonlore => "SkillDragonlore",
                    Self::SkillAttUpOurforces => "SkillAttUpOurforces",
                    Self::SkillHpUpOurforces => "SkillHpUpOurforces",
                    Self::RageUniAttack => "RageUniAttack",
                    Self::RageUniAttackOneRight => "RageUniAttackOneRight",
                    Self::RageUniRun => "RageUniRun",
                    Self::RageUniRunOneRight => "RageUniRunOneRight",
                    Self::RageUniStopOneRight => "RageUniStopOneRight",
                    Self::RageFenrir => "RageFenrir",
                    Self::RageFenrirTwoSword => "RageFenrirTwoSword",
                    Self::RageFenrirOneRight => "RageFenrirOneRight",
                    Self::RageFenrirOneLeft => "RageFenrirOneLeft",
                    Self::RageFenrirWalk => "RageFenrirWalk",
                    Self::RageFenrirWalkOneRight => "RageFenrirWalkOneRight",
                    Self::RageFenrirWalkOneLeft => "RageFenrirWalkOneLeft",
                    Self::RageFenrirWalkTwoSword => "RageFenrirWalkTwoSword",
                    Self::RageFenrirRun => "RageFenrirRun",
                    Self::RageFenrirRunTwoSword => "RageFenrirRunTwoSword",
                    Self::RageFenrirRunOneRight => "RageFenrirRunOneRight",
                    Self::RageFenrirRunOneLeft => "RageFenrirRunOneLeft",
                    Self::RageFenrirStand => "RageFenrirStand",
                    Self::RageFenrirStandTwoSword => "RageFenrirStandTwoSword",
                    Self::RageFenrirStandOneRight => "RageFenrirStandOneRight",
                    Self::RageFenrirStandOneLeft => "RageFenrirStandOneLeft",
                    Self::RageFenrirDamage => "RageFenrirDamage",
                    Self::RageFenrirDamageTwoSword => "RageFenrirDamageTwoSword",
                    Self::RageFenrirDamageOneRight => "RageFenrirDamageOneRight",
                    Self::RageFenrirDamageOneLeft => "RageFenrirDamageOneLeft",
                    Self::RageFenrirAttackRight => "RageFenrirAttackRight",
                    Self::StopRagefighter => "StopRagefighter",
                }
            }
        }

        pub fn animation_display_name(index: usize) -> String {
            match PlayerAction::from_index(index) {
                Some(action) => format!("{:03} {}", index, action.name()),
                None => format!("{:03} Action{}", index, index),
            }
        }
    }

    pub mod equipment {
        use super::types::*;

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum EquipmentSet {
            ClassDefault,
            Standard(u8),
            ClassWar(u8),
            HighDarkKnight(u8),
            Mask(u8),
            ElfCosmic(u8),
        }

        impl EquipmentSet {
            pub fn available_for(body_type: BodyType) -> Vec<EquipmentSet> {
                let mut sets = vec![EquipmentSet::ClassDefault];
                match body_type {
                    BodyType::Male => {
                        for id in 1..=10 { sets.push(EquipmentSet::Standard(id)); }
                        for id in 16..=29 { sets.push(EquipmentSet::Standard(id)); }
                        for id in 1..=5 { sets.push(EquipmentSet::ClassWar(id)); }
                        for id in 1..=5 { sets.push(EquipmentSet::HighDarkKnight(id)); }
                        for id in [1, 6, 7, 9, 10] { sets.push(EquipmentSet::Mask(id)); }
                    }
                    BodyType::Elf => {
                        for id in 1..=5 { sets.push(EquipmentSet::Standard(id)); }
                        for id in 1..=2 { sets.push(EquipmentSet::ElfCosmic(id)); }
                    }
                    BodyType::Monk => {
                        for id in 1..=4 { sets.push(EquipmentSet::Standard(id)); }
                    }
                }
                sets
            }

            pub fn glb_path(&self, slot: BodySlot, body_type: BodyType, class: CharacterClass) -> String {
                match self {
                    EquipmentSet::ClassDefault => {
                        format!("data/player/{}_class_{:02}.glb", slot.prefix(), class.class_id())
                    }
                    EquipmentSet::Standard(id) => {
                        format!("data/player/{}_{}_{:02}.glb", slot.prefix(), body_type.slug(), id)
                    }
                    EquipmentSet::ClassWar(id) => {
                        format!("data/player/cw_{}_{}_{:02}.glb", slot.prefix(), body_type.slug(), id)
                    }
                    EquipmentSet::HighDarkKnight(id) => {
                        format!("data/player/hdk_{}_{}_{:02}.glb", slot.prefix(), body_type.slug(), id)
                    }
                    EquipmentSet::Mask(id) => {
                        if slot == BodySlot::Helm {
                            format!("data/player/mask_helm_{}_{:02}.glb", body_type.slug(), id)
                        } else {
                            format!("data/player/{}_class_{:02}.glb", slot.prefix(), class.class_id())
                        }
                    }
                    EquipmentSet::ElfCosmic(id) => {
                        format!("data/player/{}_elf_c_{:02}.glb", slot.prefix(), id)
                    }
                }
            }

            pub fn display_name(&self) -> String {
                match self {
                    EquipmentSet::ClassDefault => "Class Default".to_string(),
                    EquipmentSet::Standard(id) => format!("Set {:02}", id),
                    EquipmentSet::ClassWar(id) => format!("ClassWar {:02}", id),
                    EquipmentSet::HighDarkKnight(id) => format!("HDK {:02}", id),
                    EquipmentSet::Mask(id) => format!("Mask {:02}", id),
                    EquipmentSet::ElfCosmic(id) => format!("Elf Cosmic {:02}", id),
                }
            }
        }
    }
}

use character::animations::animation_display_name;
use character::controller::{CharacterAnimState, CharacterController, CharacterState};
use character::equipment::EquipmentSet;
use character::types::*;

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_PLAYBACK_SPEED: f32 = 0.16;

/// MU terrain grid cell size (same as terrain scale in scene_loader).
const GRID_CELL_SIZE: f32 = 100.0;

/// Number of heightmap cells in each direction.
const GROUND_CELLS: usize = 256;

/// Ground plane total size (256 grid cells).
const GROUND_SIZE: f32 = GROUND_CELLS as f32 * GRID_CELL_SIZE;

/// Height multiplier from terrain config (world_1 default).
const HEIGHT_MULTIPLIER: f32 = 1.5;

/// Number of grid cells visible in each direction from the character.
const GRID_VISIBLE_HALF_CELLS: i32 = 25;

/// MU Camera parameters (from MuClient5.2 ZzzScene.cpp).
const MU_CAMERA_PITCH_DEG: f32 = 48.5;
const MU_CAMERA_YAW_DEG: f32 = -45.0;
const MU_CAMERA_DISTANCE: f32 = 1000.0;
const MU_CAMERA_LOOK_HEIGHT: f32 = 80.0;

const ZOOM_MIN: f32 = 300.0;
const ZOOM_MAX: f32 = 2500.0;
const ZOOM_SPEED: f32 = 100.0;

/// Camera rotation sensitivity (degrees per pixel of mouse movement).
const CAMERA_ROTATION_SENSITIVITY: f32 = 0.3;
const CAMERA_PITCH_MIN: f32 = 5.0;
const CAMERA_PITCH_MAX: f32 = 89.0;

const WALK_SPEED: f32 = 300.0;
const RUN_SPEED: f32 = 375.0;
const ARRIVAL_THRESHOLD: f32 = 5.0;
const TURN_SPEED: f32 = 10.0;
const RUN_TO_WALK_THRESHOLD: f32 = 300.0;

/// Small yaw correction for model bind-pose alignment (radians).
const MODEL_YAW_OFFSET: f32 = 0.0;

// ============================================================================
// Heightmap resource
// ============================================================================

#[derive(serde::Deserialize)]
struct HeightmapJson {
    width: u32,
    height: u32,
    heights: Vec<Vec<f32>>,
}

#[derive(Resource)]
struct HeightmapResource {
    width: usize,
    height: usize,
    /// Row-major flat buffer: heights[z * width + x]
    heights: Vec<f32>,
}

impl HeightmapResource {
    fn from_json(json: HeightmapJson) -> Self {
        let w = json.width as usize;
        let h = json.height as usize;
        let mut flat = vec![0.0f32; w * h];
        for (z, row) in json.heights.iter().enumerate() {
            for (x, &val) in row.iter().enumerate() {
                if z < h && x < w {
                    flat[z * w + x] = val;
                }
            }
        }
        Self { width: w, height: h, heights: flat }
    }

    fn get_height(&self, x: usize, z: usize) -> f32 {
        if x < self.width && z < self.height {
            self.heights[z * self.width + x]
        } else {
            0.0
        }
    }
}

/// Bilinear interpolation of terrain height at world coordinates.
fn terrain_height_at(heightmap: &HeightmapResource, world_x: f32, world_z: f32) -> f32 {
    let fx = world_x / GRID_CELL_SIZE;
    let fz = world_z / GRID_CELL_SIZE;

    let x0 = (fx.floor() as isize).max(0) as usize;
    let z0 = (fz.floor() as isize).max(0) as usize;
    let x1 = (x0 + 1).min(heightmap.width.saturating_sub(1));
    let z1 = (z0 + 1).min(heightmap.height.saturating_sub(1));

    let tx = (fx - x0 as f32).clamp(0.0, 1.0);
    let tz = (fz - z0 as f32).clamp(0.0, 1.0);

    let h00 = heightmap.get_height(x0, z0) * HEIGHT_MULTIPLIER;
    let h10 = heightmap.get_height(x1, z0) * HEIGHT_MULTIPLIER;
    let h01 = heightmap.get_height(x0, z1) * HEIGHT_MULTIPLIER;
    let h11 = heightmap.get_height(x1, z1) * HEIGHT_MULTIPLIER;

    let h0 = h00 + (h10 - h00) * tx;
    let h1 = h01 + (h11 - h01) * tx;
    h0 + (h1 - h0) * tz
}

// ============================================================================
// Viewer state
// ============================================================================

#[derive(Resource)]
struct ViewerState {
    selected_class_index: usize,
    selected_animation: usize,
    playback_speed: f32,
    playing: bool,
    character_entity: Option<Entity>,
    pending_class_change: bool,
    pending_animation_change: bool,
    pending_toggle_playback: bool,
    movement_target: Option<Vec3>,
    status: String,
    selected_set_index: usize,
    available_sets: Vec<EquipmentSet>,
    use_remaster: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        let body_type = CharacterClass::ALL[0].body_type();
        Self {
            selected_class_index: 0,
            selected_animation: 1, // StopMale (idle)
            playback_speed: DEFAULT_PLAYBACK_SPEED,
            playing: true,
            character_entity: None,
            pending_class_change: true, // Spawn on startup
            pending_animation_change: false,
            pending_toggle_playback: false,
            movement_target: None,
            status: "Loading player.glb...".to_string(),
            selected_set_index: 0,
            available_sets: EquipmentSet::available_for(body_type),
            use_remaster: false,
        }
    }
}

#[derive(Resource)]
struct PlayerAnimLib {
    gltf_handle: Handle<Gltf>,
    graph_handle: Option<Handle<AnimationGraph>>,
    animation_nodes: Vec<AnimationNodeIndex>,
    animation_names: Vec<String>,
    initialized: bool,
}

#[derive(Component)]
struct AnimBound;

/// Marker for the invisible animated skeleton scene (player.glb).
#[derive(Component)]
struct SkeletonMarker;

/// MU-style follow camera with fixed pitch/yaw and adjustable distance.
#[derive(Component)]
struct MuCamera {
    pitch_deg: f32,
    yaw_deg: f32,
    distance: f32,
}

/// Tracks the ground texture handle for deferred sampler configuration.
#[derive(Resource)]
struct GroundTextureState {
    handle: Handle<Image>,
    configured: bool,
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    // Load heightmap synchronously at startup (small JSON file).
    let asset_root = asset_root_path();
    let heightmap_path = format!("{}/data/world_1/terrain_height.json", asset_root);
    let heightmap_json: HeightmapJson = {
        let bytes = std::fs::read(&heightmap_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", heightmap_path, e));
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|e| panic!("Failed to parse {}: {}", heightmap_path, e))
    };
    let heightmap = HeightmapResource::from_json(heightmap_json);
    info!(
        "Loaded heightmap: {}x{} from {}",
        heightmap.width, heightmap.height, heightmap_path
    );

    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 250.0,
        })
        .insert_resource(ViewerState::default())
        .insert_resource(heightmap)
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "MU Character Viewer".to_string(),
                        resolution: WindowResolution::new(1440.0, 900.0),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: asset_root.into(),
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin)
        .add_systems(Startup, setup_viewer)
        .add_systems(
            Update,
            (
                configure_ground_texture,
                draw_character_viewer_ui,
                draw_bottom_info_bar,
                handle_class_change,
                init_player_animation_lib,
                bind_anim_players,
                apply_animation_changes,
                handle_click_to_move,
                advance_movement,
                rotate_idle_to_mouse,
                handle_scroll_zoom,
                handle_camera_rotation,
                update_mu_camera,
                draw_grid_lines,
                draw_movement_target,
            ),
        )
        .add_systems(
            PostUpdate,
            sync_bone_transforms.before(bevy::transform::TransformSystem::TransformPropagate),
        )
        .run();
}

fn asset_root_path() -> String {
    format!("{}/../assets", env!("CARGO_MANIFEST_DIR"))
}

// ============================================================================
// Setup
// ============================================================================

fn setup_viewer(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    heightmap: Res<HeightmapResource>,
) {
    // MU-style follow camera with Gaussian shadow filtering
    let mu_cam = MuCamera {
        pitch_deg: MU_CAMERA_PITCH_DEG,
        yaw_deg: MU_CAMERA_YAW_DEG,
        distance: MU_CAMERA_DISTANCE,
    };
    let cam_transform = compute_mu_camera_transform(&mu_cam, Vec3::ZERO);

    commands.spawn((
        Camera3dBundle {
            transform: cam_transform,
            tonemapping: Tonemapping::ReinhardLuminance,
            ..default()
        },
        mu_cam,
        ShadowFilteringMethod::Gaussian,
    ));

    // Directional light with high-quality shadow cascades
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 18_000.0,
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 3,
            minimum_distance: 10.0,
            maximum_distance: 8_000.0,
            first_cascade_far_bound: 800.0,
            overlap_proportion: 0.15,
        }
        .build(),
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.8, 0.0)),
        ..default()
    });

    // Load ground texture (tile_ground_01 from world 1)
    let ground_texture: Handle<Image> = asset_server.load("data/world_1/tile_ground_01.png");

    // Build 256x256 heightmap terrain mesh
    let terrain_mesh = build_terrain_mesh(&heightmap);

    commands.spawn(PbrBundle {
        mesh: meshes.add(terrain_mesh),
        material: materials.add(StandardMaterial {
            base_color_texture: Some(ground_texture.clone()),
            perceptual_roughness: 0.95,
            metallic: 0.0,
            ..default()
        }),
        ..default()
    });

    // Store texture handle for deferred sampler configuration
    commands.insert_resource(GroundTextureState {
        handle: ground_texture,
        configured: false,
    });

    // Load player.glb for animations
    let gltf_handle: Handle<Gltf> = asset_server.load("data/player/player.glb");
    commands.insert_resource(PlayerAnimLib {
        gltf_handle,
        graph_handle: None,
        animation_nodes: Vec::new(),
        animation_names: Vec::new(),
        initialized: false,
    });
}

/// Build a 256x256 vertex terrain mesh from the heightmap.
fn build_terrain_mesh(heightmap: &HeightmapResource) -> Mesh {
    let w = heightmap.width.min(GROUND_CELLS);
    let h = heightmap.height.min(GROUND_CELLS);

    let mut positions = Vec::with_capacity(w * h);
    let cells_per_tile = 4.0;

    for z in 0..h {
        for x in 0..w {
            let height = heightmap.get_height(x, z) * HEIGHT_MULTIPLIER;
            positions.push([x as f32 * GRID_CELL_SIZE, height, z as f32 * GRID_CELL_SIZE]);
        }
    }

    // Build indices
    let mut indices = Vec::with_capacity((w - 1) * (h - 1) * 6);
    for z in 0..(h - 1) {
        for x in 0..(w - 1) {
            let tl = (z * w + x) as u32;
            let tr = tl + 1;
            let bl = ((z + 1) * w + x) as u32;
            let br = bl + 1;
            indices.push(tl);
            indices.push(bl);
            indices.push(tr);
            indices.push(tr);
            indices.push(bl);
            indices.push(br);
        }
    }

    // Compute normals from triangle faces
    let mut normals = vec![[0.0f32, 0.0, 0.0]; positions.len()];
    for triangle in indices.chunks(3) {
        let i0 = triangle[0] as usize;
        let i1 = triangle[1] as usize;
        let i2 = triangle[2] as usize;
        let p0 = Vec3::from(positions[i0]);
        let p1 = Vec3::from(positions[i1]);
        let p2 = Vec3::from(positions[i2]);
        let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
        for &i in &[i0, i1, i2] {
            normals[i][0] += normal.x;
            normals[i][1] += normal.y;
            normals[i][2] += normal.z;
        }
    }
    for n in &mut normals {
        let v = Vec3::from(*n).normalize_or_zero();
        let v = if v.length_squared() > 0.0 { v } else { Vec3::Y };
        *n = [v.x, v.y, v.z];
    }

    // Tiled UVs (each tile covers cells_per_tile grid cells)
    let uv_step = 1.0 / cells_per_tile;
    let uvs: Vec<[f32; 2]> = (0..h)
        .flat_map(|z| (0..w).map(move |x| [x as f32 * uv_step, z as f32 * uv_step]))
        .collect();

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// Once the ground texture is loaded, set its sampler to Repeat for tiling.
fn configure_ground_texture(
    mut ground: ResMut<GroundTextureState>,
    mut images: ResMut<Assets<Image>>,
) {
    if ground.configured {
        return;
    }
    let Some(image) = images.get_mut(&ground.handle) else {
        return;
    };
    ground.configured = true;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        address_mode_w: ImageAddressMode::Repeat,
        ..default()
    });
}

// ============================================================================
// Scroll wheel zoom
// ============================================================================

fn handle_scroll_zoom(
    mut scroll_events: EventReader<MouseWheel>,
    mut cameras: Query<&mut MuCamera>,
    mut egui_contexts: EguiContexts,
) {
    if egui_contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    let mut delta = 0.0f32;
    for event in scroll_events.read() {
        delta += event.y;
    }

    if delta.abs() < 0.001 {
        return;
    }

    for mut mu_cam in &mut cameras {
        mu_cam.distance = (mu_cam.distance - delta * ZOOM_SPEED).clamp(ZOOM_MIN, ZOOM_MAX);
    }
}

// ============================================================================
// Right-click camera rotation
// ============================================================================

fn handle_camera_rotation(
    mouse: Res<ButtonInput<MouseButton>>,
    mut motion_events: EventReader<MouseMotion>,
    mut cameras: Query<&mut MuCamera>,
    mut egui_contexts: EguiContexts,
) {
    // Consume motion events regardless to avoid stale accumulation
    let total_delta: Vec2 = motion_events.read().map(|e| e.delta).sum();

    if !mouse.pressed(MouseButton::Right) {
        return;
    }

    if egui_contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    if total_delta.length_squared() < 0.001 {
        return;
    }

    for mut mu_cam in &mut cameras {
        mu_cam.yaw_deg -= total_delta.x * CAMERA_ROTATION_SENSITIVITY;
        mu_cam.pitch_deg = (mu_cam.pitch_deg + total_delta.y * CAMERA_ROTATION_SENSITIVITY)
            .clamp(CAMERA_PITCH_MIN, CAMERA_PITCH_MAX);
    }
}

// ============================================================================
// Grid lines (Gizmos)
// ============================================================================

fn draw_grid_lines(
    mut gizmos: Gizmos,
    characters: Query<&Transform, With<CharacterRoot>>,
    heightmap: Res<HeightmapResource>,
) {
    let char_pos = characters
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let cx = (char_pos.x / GRID_CELL_SIZE).round() * GRID_CELL_SIZE;
    let cz = (char_pos.z / GRID_CELL_SIZE).round() * GRID_CELL_SIZE;

    let half = GRID_VISIBLE_HALF_CELLS;
    let extent = half as f32 * GRID_CELL_SIZE;
    let color = Color::srgb(0.3, 0.3, 0.3);
    let y_offset = 0.5;

    for i in -half..=half {
        let pos = i as f32 * GRID_CELL_SIZE;

        // Lines along X axis (varying Z)
        let z_world = cz + pos;
        let x_start = cx - extent;
        let x_end = cx + extent;
        let y_start = terrain_height_at(&heightmap, x_start, z_world) + y_offset;
        let y_end = terrain_height_at(&heightmap, x_end, z_world) + y_offset;
        gizmos.line(
            Vec3::new(x_start, y_start, z_world),
            Vec3::new(x_end, y_end, z_world),
            color,
        );

        // Lines along Z axis (varying X)
        let x_world = cx + pos;
        let z_start = cz - extent;
        let z_end = cz + extent;
        let y_start2 = terrain_height_at(&heightmap, x_world, z_start) + y_offset;
        let y_end2 = terrain_height_at(&heightmap, x_world, z_end) + y_offset;
        gizmos.line(
            Vec3::new(x_world, y_start2, z_start),
            Vec3::new(x_world, y_end2, z_end),
            color,
        );
    }
}

// ============================================================================
// MU-style follow camera
// ============================================================================

fn compute_mu_camera_transform(cam: &MuCamera, char_pos: Vec3) -> Transform {
    let pitch_rad = cam.pitch_deg.to_radians();
    let yaw_rad = cam.yaw_deg.to_radians();

    let horizontal = cam.distance * pitch_rad.cos();
    let vertical = cam.distance * pitch_rad.sin();

    let offset = Vec3::new(
        horizontal * yaw_rad.sin(),
        vertical,
        horizontal * yaw_rad.cos(),
    );

    let look_at = Vec3::new(char_pos.x, char_pos.y + MU_CAMERA_LOOK_HEIGHT, char_pos.z);
    let eye = look_at + offset;

    Transform::from_translation(eye).looking_at(look_at, Vec3::Y)
}

fn update_mu_camera(
    characters: Query<&Transform, With<CharacterRoot>>,
    mut cameras: Query<(&mut Transform, &MuCamera), Without<CharacterRoot>>,
) {
    let char_pos = characters
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    for (mut cam_transform, mu_cam) in &mut cameras {
        *cam_transform = compute_mu_camera_transform(mu_cam, char_pos);
    }
}

// ============================================================================
// UI Panel
// ============================================================================

fn draw_character_viewer_ui(
    mut contexts: EguiContexts,
    mut viewer: ResMut<ViewerState>,
    library: Option<Res<PlayerAnimLib>>,
) {
    let ctx = contexts.ctx_mut();
    egui::Window::new("Character Viewer")
        .default_pos(egui::pos2(12.0, 12.0))
        .default_width(420.0)
        .show(ctx, |ui| {
            // Class selector
            ui.label("Character Class:");
            let current_class = CharacterClass::ALL[viewer.selected_class_index];
            let mut new_class_index = viewer.selected_class_index;
            egui::ComboBox::from_label("Class")
                .selected_text(current_class.name())
                .show_ui(ui, |ui| {
                    for (i, class) in CharacterClass::ALL.iter().enumerate() {
                        ui.selectable_value(&mut new_class_index, i, class.name());
                    }
                });
            if new_class_index != viewer.selected_class_index {
                viewer.selected_class_index = new_class_index;
                let new_class = CharacterClass::ALL[new_class_index];
                viewer.available_sets = EquipmentSet::available_for(new_class.body_type());
                viewer.selected_set_index = 0;
                viewer.pending_class_change = true;
            }

            // Equipment set selector
            ui.separator();
            ui.label("Equipment Set:");
            let current_set_name = viewer.available_sets
                .get(viewer.selected_set_index)
                .map(|s| s.display_name())
                .unwrap_or_else(|| "Class Default".to_string());
            let mut new_set_index = viewer.selected_set_index;
            egui::ComboBox::from_label("Set")
                .selected_text(&current_set_name)
                .show_ui(ui, |ui| {
                    for (i, set) in viewer.available_sets.iter().enumerate() {
                        ui.selectable_value(&mut new_set_index, i, set.display_name());
                    }
                });
            if new_set_index != viewer.selected_set_index {
                viewer.selected_set_index = new_set_index;
                viewer.pending_class_change = true; // Respawn with new equipment
            }

            // Remaster toggle
            ui.separator();
            let prev_remaster = viewer.use_remaster;
            ui.checkbox(&mut viewer.use_remaster, "Use Remaster models");
            if viewer.use_remaster != prev_remaster {
                viewer.pending_class_change = true; // Respawn with new paths
            }

            ui.separator();

            // Animation selector
            if let Some(lib) = &library {
                if lib.animation_names.is_empty() {
                    ui.label("Animations: waiting for player.glb...");
                } else {
                    let mut selected = viewer.selected_animation;
                    let selected_name = lib
                        .animation_names
                        .get(viewer.selected_animation)
                        .cloned()
                        .unwrap_or_else(|| format!("Action{}", viewer.selected_animation));

                    egui::ComboBox::from_label("Animation")
                        .selected_text(&selected_name)
                        .show_ui(ui, |ui| {
                            for (i, name) in lib.animation_names.iter().enumerate() {
                                ui.selectable_value(&mut selected, i, name);
                            }
                        });

                    if selected != viewer.selected_animation {
                        viewer.selected_animation = selected;
                        viewer.pending_animation_change = true;
                    }
                }
            }

            // Playback speed
            let speed_slider =
                egui::Slider::new(&mut viewer.playback_speed, 0.02..=1.2).text("Speed");
            if ui.add(speed_slider).changed() {
                viewer.pending_animation_change = true;
            }

            // Play/Pause
            ui.horizontal(|ui| {
                let label = if viewer.playing { "Pause" } else { "Play" };
                if ui.button(label).clicked() {
                    viewer.pending_toggle_playback = true;
                }
            });

            ui.label("LMB: move | Scroll: zoom | RMB: rotate camera");

            ui.separator();
            ui.label(format!("Status: {}", viewer.status));
        });
}

/// Bottom info bar with MU grid coordinates.
fn draw_bottom_info_bar(
    mut contexts: EguiContexts,
    characters: Query<&Transform, With<CharacterRoot>>,
) {
    let char_pos = characters
        .get_single()
        .map(|t| t.translation)
        .unwrap_or(Vec3::ZERO);

    let col = ((char_pos.x / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);
    let row = ((char_pos.z / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);

    let ctx = contexts.ctx_mut();
    egui::TopBottomPanel::bottom("info_bar")
        .frame(egui::Frame::none().fill(egui::Color32::from_rgba_unmultiplied(0, 0, 0, 128)))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    egui::Color32::WHITE,
                    format!(
                        "  Position: ({}, {})  |  World: ({:.0}, {:.0}, {:.0})",
                        col, row, char_pos.x, char_pos.y, char_pos.z
                    ),
                );
            });
        });
}

// ============================================================================
// Remaster path resolution
// ============================================================================

/// Given a standard asset path like `data/player/foo.glb`, return the remaster
/// version `remaster/data/player/foo.glb` if it exists on disk, otherwise the original.
fn resolve_asset_path(path: &str, use_remaster: bool) -> String {
    if !use_remaster {
        return path.to_string();
    }
    let remaster_path = format!("remaster/{}", path);
    let asset_root = asset_root_path();
    let full = format!("{}/{}", asset_root, remaster_path);
    if std::path::Path::new(&full).exists() {
        remaster_path
    } else {
        path.to_string()
    }
}

// ============================================================================
// Class change -> despawn/respawn character
// ============================================================================

fn handle_class_change(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut viewer: ResMut<ViewerState>,
    heightmap: Res<HeightmapResource>,
) {
    if !viewer.pending_class_change {
        return;
    }
    viewer.pending_class_change = false;

    // Despawn existing character
    if let Some(entity) = viewer.character_entity.take() {
        commands.entity(entity).despawn_recursive();
    }

    let class = CharacterClass::ALL[viewer.selected_class_index];
    let body_type = class.body_type();
    let slots = BodySlot::slots_for(body_type);

    // Set default idle animation for the class
    viewer.selected_animation = idle_action_for_class(class);

    // Get selected equipment set
    let equipment_set = viewer.available_sets
        .get(viewer.selected_set_index)
        .copied()
        .unwrap_or(EquipmentSet::ClassDefault);

    // Spawn position at center of map with terrain height
    let spawn_x = GRID_CELL_SIZE * 128.5;
    let spawn_z = GRID_CELL_SIZE * 128.5;
    let spawn_y = terrain_height_at(&heightmap, spawn_x, spawn_z);

    let root = commands
        .spawn((
            SpatialBundle {
                transform: Transform::from_translation(Vec3::new(spawn_x, spawn_y, spawn_z)),
                ..default()
            },
            CharacterRoot,
            CharacterController {
                class,
                state: CharacterState::Idle,
            },
            CharacterAnimState {
                current_action: viewer.selected_animation,
                playback_speed: viewer.playback_speed,
            },
        ))
        .id();

    for &slot in slots {
        let base_path = equipment_set.glb_path(slot, body_type, class);
        let glb_path = resolve_asset_path(&base_path, viewer.use_remaster);
        let scene_path = format!("{glb_path}#Scene0");
        let scene_handle: Handle<Scene> = asset_server.load(scene_path);

        let part = commands
            .spawn((
                SceneBundle {
                    scene: scene_handle,
                    ..default()
                },
                BodyPartMarker { slot },
            ))
            .id();

        commands.entity(root).add_child(part);
    }

    // Spawn the animated skeleton (player.glb has animations, 0 meshes).
    let skeleton_glb = resolve_asset_path("data/player/player.glb", viewer.use_remaster);
    let skeleton_scene: Handle<Scene> = asset_server.load(format!("{}#Scene0", skeleton_glb));
    let skeleton = commands
        .spawn((
            SceneBundle {
                scene: skeleton_scene,
                ..default()
            },
            SkeletonMarker,
        ))
        .id();
    commands.entity(root).add_child(skeleton);

    viewer.character_entity = Some(root);
    let remaster_tag = if viewer.use_remaster { " [Remaster]" } else { "" };
    viewer.status = format!(
        "Spawned {} ({} body, {}){}",
        class.name(),
        body_type.slug(),
        equipment_set.display_name(),
        remaster_tag
    );
}

// ============================================================================
// Animation library init
// ============================================================================

fn init_player_animation_lib(
    mut library: ResMut<PlayerAnimLib>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut viewer: ResMut<ViewerState>,
) {
    if library.initialized {
        return;
    }

    let Some(gltf) = gltfs.get(&library.gltf_handle) else {
        return;
    };

    library.initialized = true;

    if gltf.animations.is_empty() {
        viewer.status =
            "player.glb has no animations. Run bmd_converter on player.bmd first.".to_string();
        return;
    }

    let mut graph = AnimationGraph::new();
    let nodes: Vec<AnimationNodeIndex> = graph
        .add_clips(gltf.animations.iter().cloned(), 1.0, graph.root)
        .collect();

    // Use PlayerAction names for animation display
    let mut names = Vec::with_capacity(gltf.animations.len());
    names.extend((0..gltf.animations.len()).map(|i| animation_display_name(i)));

    let index_by_clip: std::collections::HashMap<bevy::asset::AssetId<AnimationClip>, usize> = gltf
        .animations
        .iter()
        .enumerate()
        .map(|(i, h)| (h.id(), i))
        .collect();

    for (name, handle) in &gltf.named_animations {
        if let Some(&idx) = index_by_clip.get(&handle.id()) {
            // Keep the PlayerAction name if it's more descriptive
            let pa_name = animation_display_name(idx);
            if pa_name.contains("Action") {
                // The named animation from glTF is more specific
                names[idx] = format!("{:03} {}", idx, name);
            }
        }
    }

    viewer.status = format!("Loaded {} animations from player.glb", nodes.len());
    library.graph_handle = Some(graphs.add(graph));
    library.animation_nodes = nodes;
    library.animation_names = names;
}

// ============================================================================
// Bind animation players
// ============================================================================

fn bind_anim_players(
    mut commands: Commands,
    library: Res<PlayerAnimLib>,
    viewer: Res<ViewerState>,
    children_query: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer), Without<AnimBound>>,
) {
    let Some(graph_handle) = library.graph_handle.clone() else {
        return;
    };

    let Some(root_entity) = viewer.character_entity else {
        return;
    };

    let animation_node = library
        .animation_nodes
        .get(viewer.selected_animation)
        .copied()
        .or_else(|| library.animation_nodes.first().copied());

    let Some(animation_node) = animation_node else {
        return;
    };

    let unbound = find_unbound_players(root_entity, &children_query, &players);

    for player_entity in unbound {
        if let Ok((entity, mut player)) = players.get_mut(player_entity) {
            let mut transitions = AnimationTransitions::new();
            transitions
                .play(&mut player, animation_node, Duration::ZERO)
                .set_speed(viewer.playback_speed.max(0.001))
                .repeat();

            if !viewer.playing {
                player.pause_all();
            }

            commands
                .entity(entity)
                .insert((graph_handle.clone(), transitions, AnimBound));
        }
    }
}

fn find_unbound_players(
    root: Entity,
    children_query: &Query<&Children>,
    players: &Query<(Entity, &mut AnimationPlayer), Without<AnimBound>>,
) -> Vec<Entity> {
    let mut result = Vec::new();
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if players.contains(entity) {
            result.push(entity);
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
    result
}

// ============================================================================
// Apply animation / playback changes
// ============================================================================

fn apply_animation_changes(
    mut viewer: ResMut<ViewerState>,
    library: Res<PlayerAnimLib>,
    mut bound_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions), With<AnimBound>>,
    mut anim_state_query: Query<&mut CharacterAnimState, With<CharacterRoot>>,
) {
    let anim_changed = std::mem::take(&mut viewer.pending_animation_change);
    let toggle = std::mem::take(&mut viewer.pending_toggle_playback);

    if toggle {
        viewer.playing = !viewer.playing;
    }

    if !anim_changed && !toggle {
        return;
    }

    if anim_changed {
        if let Some(root) = viewer.character_entity {
            if let Ok(mut state) = anim_state_query.get_mut(root) {
                state.current_action = viewer.selected_animation;
                state.playback_speed = viewer.playback_speed;
            }
        }
    }

    let animation_node = library
        .animation_nodes
        .get(viewer.selected_animation)
        .copied()
        .or_else(|| library.animation_nodes.first().copied());

    let Some(animation_node) = animation_node else {
        return;
    };

    for (mut player, mut transitions) in &mut bound_players {
        if anim_changed {
            transitions
                .play(&mut player, animation_node, Duration::from_millis(200))
                .set_speed(viewer.playback_speed.max(0.001))
                .repeat();
        }

        if toggle || anim_changed {
            if viewer.playing {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }
    }

    if anim_changed {
        let name = library
            .animation_names
            .get(viewer.selected_animation)
            .map(String::as_str)
            .unwrap_or("unnamed");
        viewer.status = format!("Playing {} (index {})", name, viewer.selected_animation);
    } else if toggle {
        viewer.status = if viewer.playing {
            "Resumed.".to_string()
        } else {
            "Paused.".to_string()
        };
    }
}

// ============================================================================
// Click-to-move (grid-snapped)
// ============================================================================

fn handle_click_to_move(
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut viewer: ResMut<ViewerState>,
    _library: Res<PlayerAnimLib>,
    mut characters: Query<(&mut CharacterController, &mut CharacterAnimState), With<CharacterRoot>>,
    mut egui_contexts: EguiContexts,
    heightmap: Res<HeightmapResource>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    if egui_contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, camera_transform)) = cameras.get_single() else {
        return;
    };

    let Some(ray) = camera.viewport_to_world(camera_transform, cursor_pos) else {
        return;
    };

    if ray.direction.y.abs() < 1e-6 {
        return;
    }

    // Approximate raycast: intersect with Y=avg_height plane, then refine
    let approx_y = terrain_height_at(&heightmap, ray.origin.x, ray.origin.z);
    let t = (approx_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return;
    }

    let hit = ray.origin + ray.direction * t;

    // Snap target to nearest cell center
    let half_cell = GRID_CELL_SIZE * 0.5;
    let snapped_x = ((hit.x - half_cell) / GRID_CELL_SIZE).round() * GRID_CELL_SIZE + half_cell;
    let snapped_z = ((hit.z - half_cell) / GRID_CELL_SIZE).round() * GRID_CELL_SIZE + half_cell;
    let target_y = terrain_height_at(&heightmap, snapped_x, snapped_z);
    let target = Vec3::new(snapped_x, target_y, snapped_z);

    for (mut controller, mut anim_state) in &mut characters {
        controller.state = CharacterState::Running { target };
        let run_action = run_action_for_class(controller.class);
        anim_state.current_action = run_action;
        anim_state.playback_speed = run_playback_speed(controller.class);
        viewer.selected_animation = run_action;
        viewer.pending_animation_change = true;
    }

    viewer.movement_target = Some(target);

    let col = ((snapped_x / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);
    let row = ((snapped_z / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);
    viewer.status = format!("Running to ({}, {})", col, row);
}

fn advance_movement(
    time: Res<Time>,
    mut viewer: ResMut<ViewerState>,
    _library: Res<PlayerAnimLib>,
    mut characters: Query<
        (
            &mut Transform,
            &mut CharacterController,
            &mut CharacterAnimState,
        ),
        With<CharacterRoot>,
    >,
    heightmap: Res<HeightmapResource>,
) {
    let dt = time.delta_seconds();

    for (mut transform, mut controller, mut anim_state) in &mut characters {
        let (target, speed) = match controller.state {
            CharacterState::Running { target } => (target, RUN_SPEED),
            CharacterState::Walking { target } => (target, WALK_SPEED),
            CharacterState::Idle => {
                // Keep character on terrain surface even when idle
                transform.translation.y =
                    terrain_height_at(&heightmap, transform.translation.x, transform.translation.z);
                continue;
            }
        };

        let diff = Vec3::new(
            target.x - transform.translation.x,
            0.0,
            target.z - transform.translation.z,
        );
        let distance = diff.length();

        // Arrival: snap to target and go idle
        if distance < ARRIVAL_THRESHOLD {
            transform.translation.x = target.x;
            transform.translation.z = target.z;
            transform.translation.y = terrain_height_at(&heightmap, target.x, target.z);
            controller.state = CharacterState::Idle;
            let idle_action = idle_action_for_class(controller.class);
            anim_state.current_action = idle_action;
            viewer.selected_animation = idle_action;
            viewer.pending_animation_change = true;
            viewer.movement_target = None;
            let col = ((target.x / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);
            let row = ((target.z / GRID_CELL_SIZE).floor() as i32 + 1).clamp(1, 256);
            viewer.status = format!("Arrived at ({}, {}). Idle.", col, row);
            continue;
        }

        // Run->Walk transition when close to target
        if matches!(controller.state, CharacterState::Running { .. })
            && distance < RUN_TO_WALK_THRESHOLD
        {
            controller.state = CharacterState::Walking { target };
            let walk_action = walk_action_for_class(controller.class);
            anim_state.current_action = walk_action;
            anim_state.playback_speed = walk_playback_speed(controller.class);
            viewer.selected_animation = walk_action;
            viewer.pending_animation_change = true;
            viewer.status = "Walking (close to target)".to_string();
            let direction = diff / distance;
            let target_yaw = direction.x.atan2(direction.z) + std::f32::consts::PI + MODEL_YAW_OFFSET;
            let target_rot = Quat::from_rotation_y(target_yaw);
            transform.rotation = transform
                .rotation
                .slerp(target_rot, (TURN_SPEED * dt).min(1.0));
            let step = (WALK_SPEED * dt).min(distance);
            transform.translation.x += direction.x * step;
            transform.translation.z += direction.z * step;
            transform.translation.y =
                terrain_height_at(&heightmap, transform.translation.x, transform.translation.z);
            continue;
        }

        let direction = diff / distance;

        // Face movement direction (add PI because glTF models face -Z)
        let target_yaw = direction.x.atan2(direction.z) + std::f32::consts::PI + MODEL_YAW_OFFSET;
        let target_rot = Quat::from_rotation_y(target_yaw);
        transform.rotation = transform
            .rotation
            .slerp(target_rot, (TURN_SPEED * dt).min(1.0));

        // Move at current speed (run or walk) — horizontal only
        let step = (speed * dt).min(distance);
        transform.translation.x += direction.x * step;
        transform.translation.z += direction.z * step;

        // Follow terrain surface
        transform.translation.y =
            terrain_height_at(&heightmap, transform.translation.x, transform.translation.z);
    }
}

// ============================================================================
// Rotate idle character to face mouse cursor
// ============================================================================

fn rotate_idle_to_mouse(
    time: Res<Time>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    mut characters: Query<(&mut Transform, &CharacterController), With<CharacterRoot>>,
    mut egui_contexts: EguiContexts,
    heightmap: Res<HeightmapResource>,
) {
    if egui_contexts.ctx_mut().wants_pointer_input() {
        return;
    }

    let Ok(window) = windows.get_single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok((camera, cam_gt)) = cameras.get_single() else {
        return;
    };
    let Some(ray) = camera.viewport_to_world(cam_gt, cursor_pos) else {
        return;
    };
    if ray.direction.y.abs() < 1e-6 {
        return;
    }

    // Approximate ground plane intersection using terrain height
    let approx_y = terrain_height_at(&heightmap, ray.origin.x, ray.origin.z);
    let t = (approx_y - ray.origin.y) / ray.direction.y;
    if t < 0.0 {
        return;
    }
    let hit = ray.origin + ray.direction * t;

    let dt = time.delta_seconds();

    for (mut transform, controller) in &mut characters {
        if !matches!(controller.state, CharacterState::Idle) {
            continue;
        }

        let diff = hit - transform.translation;
        let horiz = Vec2::new(diff.x, diff.z);
        if horiz.length() < 1.0 {
            continue;
        }

        let target_yaw = horiz.x.atan2(horiz.y) + std::f32::consts::PI + MODEL_YAW_OFFSET;
        let target_rot = Quat::from_rotation_y(target_yaw);
        transform.rotation = transform
            .rotation
            .slerp(target_rot, (TURN_SPEED * dt).min(1.0));
    }
}

// ============================================================================
// Movement target marker
// ============================================================================

fn draw_movement_target(mut gizmos: Gizmos, viewer: Res<ViewerState>) {
    let Some(target) = viewer.movement_target else {
        return;
    };

    let y = target.y + 1.0; // Slightly above terrain
    let center = Vec3::new(target.x, y, target.z);
    let color = Color::srgb(1.0, 0.3, 0.3);

    draw_circle_xz(&mut gizmos, center, 20.0, 24, color);
    draw_circle_xz(&mut gizmos, center, 8.0, 12, color);

    let arm = 14.0;
    gizmos.line(
        Vec3::new(target.x - arm, y, target.z),
        Vec3::new(target.x + arm, y, target.z),
        color,
    );
    gizmos.line(
        Vec3::new(target.x, y, target.z - arm),
        Vec3::new(target.x, y, target.z + arm),
        color,
    );
}

fn draw_circle_xz(gizmos: &mut Gizmos, center: Vec3, radius: f32, segments: usize, color: Color) {
    let step = std::f32::consts::TAU / segments as f32;
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = Vec3::new(
            center.x + radius * a0.cos(),
            center.y,
            center.z + radius * a0.sin(),
        );
        let p1 = Vec3::new(
            center.x + radius * a1.cos(),
            center.y,
            center.z + radius * a1.sin(),
        );
        gizmos.line(p0, p1, color);
    }
}

// ============================================================================
// Class -> animation helpers
// ============================================================================

fn idle_action_for_class(class: CharacterClass) -> usize {
    match class {
        CharacterClass::DarkKnight
        | CharacterClass::DarkWizard
        | CharacterClass::MagicGladiator => 1, // StopMale
        CharacterClass::FairyElf => 2,         // StopFemale
        CharacterClass::Summoner => 3,          // StopSummoner
        CharacterClass::DarkLord => 76,         // DarklordStand
        CharacterClass::RageFighter => 286,     // StopRagefighter
    }
}

fn walk_action_for_class(class: CharacterClass) -> usize {
    match class {
        CharacterClass::DarkKnight
        | CharacterClass::DarkWizard
        | CharacterClass::MagicGladiator
        | CharacterClass::DarkLord
        | CharacterClass::RageFighter => 15, // WalkMale
        CharacterClass::FairyElf | CharacterClass::Summoner => 16, // WalkFemale
    }
}

fn run_action_for_class(_class: CharacterClass) -> usize {
    25 // PLAYER_RUN — same for all classes when unarmed
}

fn walk_playback_speed(class: CharacterClass) -> f32 {
    match class {
        CharacterClass::RageFighter => 0.32,
        _ => 0.33,
    }
}

fn run_playback_speed(class: CharacterClass) -> f32 {
    match class {
        CharacterClass::RageFighter => 0.28,
        _ => 0.34,
    }
}

// ============================================================================
// Bone transform sync: skeleton -> body parts
// ============================================================================

fn sync_bone_transforms(
    skeleton_query: Query<Entity, With<SkeletonMarker>>,
    body_part_query: Query<Entity, With<BodyPartMarker>>,
    children_query: Query<&Children>,
    name_query: Query<&Name>,
    mut transform_query: Query<&mut Transform>,
) {
    for skeleton_entity in &skeleton_query {
        let bone_transforms = collect_bone_transforms(
            skeleton_entity,
            &children_query,
            &name_query,
            &transform_query,
        );

        if bone_transforms.is_empty() {
            continue;
        }

        for body_part_entity in &body_part_query {
            apply_bone_transforms(
                body_part_entity,
                &children_query,
                &name_query,
                &mut transform_query,
                &bone_transforms,
            );
        }
    }
}

fn collect_bone_transforms(
    root: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &Query<&mut Transform>,
) -> HashMap<String, Transform> {
    let mut map = HashMap::new();
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if let Ok(name) = name_query.get(entity) {
            if let Ok(t) = transform_query.get(entity) {
                map.insert(name.to_string(), *t);
            }
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
    map
}

const ROOT_BONE_NAME: &str = "Bip01";

fn apply_bone_transforms(
    root: Entity,
    children_query: &Query<&Children>,
    name_query: &Query<&Name>,
    transform_query: &mut Query<&mut Transform>,
    bone_transforms: &HashMap<String, Transform>,
) {
    let mut queue = vec![root];
    while let Some(entity) = queue.pop() {
        if let Ok(name) = name_query.get(entity) {
            if let Some(&skel_t) = bone_transforms.get(name.as_str()) {
                if let Ok(mut bp_t) = transform_query.get_mut(entity) {
                    if name.as_str() == ROOT_BONE_NAME {
                        bp_t.rotation = skel_t.rotation;
                        bp_t.translation.y = skel_t.translation.y;
                    } else {
                        *bp_t = skel_t;
                    }
                }
            }
        }
        if let Ok(children) = children_query.get(entity) {
            queue.extend(children.iter());
        }
    }
}
