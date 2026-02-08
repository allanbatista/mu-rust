//! MU Online World/Map Definitions
//!
//! This crate defines all available worlds (maps) in the MU Online game.
//! Maps are categorized into different regions and event areas.
//! Names and IDs based on muonline-cross WorldInfo attributes (Season 6+)
//! and Season 20 map data.
//!
//! Enum discriminant values match the World folder numbers (World1, World2, etc.)
//! from the game data files.

/// Represents all available worlds/maps in MU Online
///
/// The ID values correspond to the World folder numbers used in the game data
/// (e.g. `Lorencia = 1` matches the `World1/` folder).
/// Names sourced from muonline-cross C# client WorldInfo attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum WorldMap {
    /// Placeholder for ID 0 (no World0 folder exists)
    Unk0 = 0,

    // ==================== Main Worlds ====================
    /// World1: Lorencia - The starting city and main trading hub
    Lorencia = 1,

    /// World2: Dungeon - Underground dungeon with monsters
    Dungeon = 2,

    /// World3: Devias - Ice town
    Devias = 3,

    /// World4: Noria - Fairy town
    Noria = 4,

    /// World5: Lost Tower - Tower dungeon
    LostTower = 5,

    /// World6: Exile
    Exile = 6,

    /// World7: Arena - PvP arena
    Arena = 7,

    /// World8: Atlans - Underwater world
    Atlans = 8,

    /// World9: Tarkan - Desert world
    Tarkan = 9,

    /// World10: Devil Square - Special event dungeon
    DevilSquare = 10,

    /// World11: Icarus - Sky world
    Icarus = 11,

    // ==================== Blood Castle (8 Levels) ====================
    /// World12: Blood Castle 1
    BloodCastle1 = 12,

    /// World13: Blood Castle 2
    BloodCastle2 = 13,

    /// World14: Blood Castle 3
    BloodCastle3 = 14,

    /// World15: Blood Castle 4
    BloodCastle4 = 15,

    /// World16: Blood Castle 5
    BloodCastle5 = 16,

    /// World17: Blood Castle 6
    BloodCastle6 = 17,

    /// World18: Blood Castle 7
    BloodCastle7 = 18,

    // ==================== Chaos Castle (7 Levels) ====================
    /// World19: Chaos Castle 1
    ChaosCastle1 = 19,

    /// World20: Chaos Castle 2
    ChaosCastle2 = 20,

    /// World21: Chaos Castle 3
    ChaosCastle3 = 21,

    /// World22: Chaos Castle 4
    ChaosCastle4 = 22,

    /// World23: Chaos Castle 5
    ChaosCastle5 = 23,

    /// World24: Chaos Castle 6
    ChaosCastle6 = 24,

    // ==================== Kalima (7 Levels) ====================
    /// World25: Kalima 1
    Kalima1 = 25,

    /// World26: Kalima 2
    Kalima2 = 26,

    /// World27: Kalima 3
    Kalima3 = 27,

    /// World28: Kalima 4
    Kalima4 = 28,

    /// World29: Kalima 5
    Kalima5 = 29,

    /// World30: Kalima 6
    Kalima6 = 30,

    // ==================== Battle Grounds ====================
    /// World31: Valley of Loren - Guild siege warfare area
    ValleyOfLoren = 31,

    /// World32: Land of Trials - Monster hunting area
    LandOfTrials = 32,

    /// World33: Devil Square 2
    DevilSquare2 = 33,

    // ==================== Extended Maps ====================
    /// World34: Aida
    Aida = 34,

    /// World35: Crywolf - Special event map
    Crywolf = 35,

    /// World37: Kalima 7
    Kalima7 = 37,

    // ==================== Kanturu ====================
    /// World38: Kanturu - Main area
    Kanturu = 38,

    /// World39: Kanturu Remain
    KanturuRemain = 39,

    /// World40: Refine Tower
    RefineTower = 40,

    // ==================== Special Areas ====================
    /// World41: Silent Map - GM/special area
    SilentMap = 41,

    /// World42: Balgass Barracks - 3rd class change quest
    BalgassBarracks = 42,

    /// World43: Balgass Refuge - 3rd class change quest
    BalgassRefuge = 43,

    // ==================== Illusion Temple (5 Levels) ====================
    /// World46: Illusion Temple 1
    IllusionTemple1 = 46,

    /// World47: Illusion Temple 2
    IllusionTemple2 = 47,

    /// World48: Illusion Temple 3
    IllusionTemple3 = 48,

    /// World49: Illusion Temple 4
    IllusionTemple4 = 49,

    /// World50: Illusion Temple 5
    IllusionTemple5 = 50,

    // ==================== Elbeland ====================
    /// World51: Elbeland 2
    Elbeland2 = 51,

    /// World52: Elbeland - Home for Summoner class
    Elbeland = 52,

    // ==================== Extended Blood/Chaos Castle ====================
    /// World53: Blood Castle 8 (Master Level)
    BloodCastle8 = 53,

    /// World54: Chaos Castle 7 (Master Level)
    ChaosCastle7 = 54,

    // ==================== Login & Character Selection ====================
    /// World55: Character selection/creation screen
    CharacterScene = 55,

    /// World56: Login scene background
    LoginScene = 56,

    // ==================== Extended Maps ====================
    /// World57: Swamp of Peace
    SwampOfPeace = 57,

    /// World58: Raklion
    Raklion = 58,

    /// World59: Raklion Boss area
    RaklionBoss = 59,

    // ==================== Holiday/Special Events ====================
    /// World63: Santa Village - Christmas event map
    SantaVillage = 63,

    /// World64: Vulcanus - PK/PvP area
    Vulcanus = 64,

    /// World65: Duel Arena - PvP tournament arena
    DuelArena = 65,

    // ==================== Doppelganger (4 Zones) ====================
    /// World66: Doppelganger Ice Zone
    DoppelgangerIceZone = 66,

    /// World67: Doppelganger Blaze Zone
    DoppelgangerBlazeZone = 67,

    /// World68: Doppelganger Underwater
    DoppelgangerUnderwater = 68,

    /// World69: Doppelganger Crystal Cave
    DoppelgangerCrystalCave = 69,

    // ==================== Imperial Guardian (4 Phases) ====================
    /// World70: Imperial Guardian 4
    ImperialGuardian4 = 70,

    /// World71: Imperial Guardian 3
    ImperialGuardian3 = 71,

    /// World72: Imperial Guardian 2
    ImperialGuardian2 = 72,

    /// World73: Imperial Guardian 1
    ImperialGuardian1 = 73,

    // ==================== Login Scenes ====================
    /// World74: New login scene (version 1)
    NewLoginScene1 = 74,

    /// World75: Event Square
    EventSquare = 75,

    /// World78: New login scene (version 2)
    NewLoginScene2 = 78,

    /// World79: New character selection screen (version 2)
    NewCharacterScene2 = 79,

    // ==================== Market & Trading ====================
    /// World80: Loren Market
    LorenMarket = 80,

    // ==================== Karutan ====================
    /// World81: Karutan 1
    Karutan1 = 81,

    /// World82: Karutan 2
    Karutan2 = 82,

    // ==================== Season 6+ Maps ====================
    /// World83: Doppelganger Renewal
    DoppelgangerRenewal = 83,

    /// World84: New Arena
    NewArena = 84,

    /// World92: Acheron
    Acheron = 92,

    /// World93: Acheron 2
    Acheron2 = 93,

    /// World94: Uruk Mountain 3
    UrukMountain3 = 94,

    /// World95: Uruk Mountain 2
    UrukMountain2 = 95,

    /// World96: Debenter
    Debenter = 96,

    /// World97: Debenter Arca Battle
    DebenterArcaBattle = 97,

    /// World99: Illusion Temple League
    IllusionTempleLeague = 99,

    /// World100: Illusion Temple League 2
    IllusionTempleLeague2 = 100,

    /// World101: Uruk Mountain
    UrukMountain = 101,

    /// World103: Tormented Square
    TormentedSquare = 103,

    /// World111: Nars
    Nars = 111,

    /// World113: Ferea
    Ferea = 113,

    /// World114: Nixies Lake
    NixiesLake = 114,

    /// World115: Loren Market (Season 6 version)
    LorenMarketS6 = 115,

    /// World117: Deep Dungeon 1
    DeepDungeon1 = 117,

    /// World118: Deep Dungeon 2
    DeepDungeon2 = 118,

    /// World119: Deep Dungeon 3
    DeepDungeon3 = 119,

    /// World120: Deep Dungeon 4
    DeepDungeon4 = 120,

    /// World121: Deep Dungeon 5
    DeepDungeon5 = 121,

    /// World122: Place of Qualification
    PlaceOfQualification = 122,

    /// World123: Swamp of Darkness
    SwampOfDarkness = 123,

    /// World124: Kubera Mine
    KuberaMine1 = 124,

    /// World125: Kubera Mine 2
    KuberaMine2 = 125,

    /// World129: Abyss of Atlans
    AbyssOfAtlans = 129,

    /// World130: Abyss of Atlans 2
    AbyssOfAtlans2 = 130,

    /// World131: Abyss of Atlans 3
    AbyssOfAtlans3 = 131,

    /// World132: Scorched Tunnels
    ScorchedTunnels = 132,

    /// World133: Red Smoke Icarus
    RedSmokeIcarus = 133,

    /// World134: Temple of Arnil
    TempleOfArnil = 134,

    /// World135: Ashen Aida
    AshenAida = 135,

    /// World136: Old Kethotum
    OldKethotum = 136,

    /// World137: Blaze Kethotum
    BlazeKethotum = 137,

    /// World138: Kanturu Undergrounds
    KanturuUndergrounds = 138,

    /// World139: Ignis Volcano
    IgnisVolcano = 139,

    /// World140: Boss Battle Zone
    BossBattleZone = 140,

    /// World141: Bloody Tarkan
    BloodyTarkan = 141,

    /// World142: Tormenta Island
    TormentaIsland = 142,

    /// World143: Doppelganger Ice Zone (New)
    DoppelgangerIceZoneNew = 143,
}

impl WorldMap {
    /// Returns a human-readable name for the world
    pub fn name(&self) -> &'static str {
        match self {
            WorldMap::Unk0 => "Unknown",
            WorldMap::Lorencia => "Lorencia",
            WorldMap::Dungeon => "Dungeon",
            WorldMap::Devias => "Devias",
            WorldMap::Noria => "Noria",
            WorldMap::LostTower => "Lost Tower",
            WorldMap::Exile => "Exile",
            WorldMap::Arena => "Arena",
            WorldMap::Atlans => "Atlans",
            WorldMap::Tarkan => "Tarkan",
            WorldMap::DevilSquare => "Devil Square",
            WorldMap::Icarus => "Icarus",
            WorldMap::BloodCastle1 => "Blood Castle 1",
            WorldMap::BloodCastle2 => "Blood Castle 2",
            WorldMap::BloodCastle3 => "Blood Castle 3",
            WorldMap::BloodCastle4 => "Blood Castle 4",
            WorldMap::BloodCastle5 => "Blood Castle 5",
            WorldMap::BloodCastle6 => "Blood Castle 6",
            WorldMap::BloodCastle7 => "Blood Castle 7",
            WorldMap::ChaosCastle1 => "Chaos Castle 1",
            WorldMap::ChaosCastle2 => "Chaos Castle 2",
            WorldMap::ChaosCastle3 => "Chaos Castle 3",
            WorldMap::ChaosCastle4 => "Chaos Castle 4",
            WorldMap::ChaosCastle5 => "Chaos Castle 5",
            WorldMap::ChaosCastle6 => "Chaos Castle 6",
            WorldMap::Kalima1 => "Kalima 1",
            WorldMap::Kalima2 => "Kalima 2",
            WorldMap::Kalima3 => "Kalima 3",
            WorldMap::Kalima4 => "Kalima 4",
            WorldMap::Kalima5 => "Kalima 5",
            WorldMap::Kalima6 => "Kalima 6",
            WorldMap::ValleyOfLoren => "Valley of Loren",
            WorldMap::LandOfTrials => "Land of Trials",
            WorldMap::DevilSquare2 => "Devil Square 2",
            WorldMap::Aida => "Aida",
            WorldMap::Crywolf => "Crywolf",
            WorldMap::Kalima7 => "Kalima 7",
            WorldMap::Kanturu => "Kanturu",
            WorldMap::KanturuRemain => "Kanturu Remain",
            WorldMap::RefineTower => "Refine Tower",
            WorldMap::SilentMap => "Silent Map",
            WorldMap::BalgassBarracks => "Balgass Barracks",
            WorldMap::BalgassRefuge => "Balgass Refuge",
            WorldMap::IllusionTemple1 => "Illusion Temple 1",
            WorldMap::IllusionTemple2 => "Illusion Temple 2",
            WorldMap::IllusionTemple3 => "Illusion Temple 3",
            WorldMap::IllusionTemple4 => "Illusion Temple 4",
            WorldMap::IllusionTemple5 => "Illusion Temple 5",
            WorldMap::Elbeland2 => "Elbeland 2",
            WorldMap::Elbeland => "Elbeland",
            WorldMap::BloodCastle8 => "Blood Castle 8",
            WorldMap::ChaosCastle7 => "Chaos Castle 7",
            WorldMap::CharacterScene => "Character Selection",
            WorldMap::LoginScene => "Login Scene",
            WorldMap::SwampOfPeace => "Swamp of Peace",
            WorldMap::Raklion => "Raklion",
            WorldMap::RaklionBoss => "Raklion Boss",
            WorldMap::SantaVillage => "Santa Village",
            WorldMap::Vulcanus => "Vulcanus",
            WorldMap::DuelArena => "Duel Arena",
            WorldMap::DoppelgangerIceZone => "Doppelganger Ice Zone",
            WorldMap::DoppelgangerBlazeZone => "Doppelganger Blaze Zone",
            WorldMap::DoppelgangerUnderwater => "Doppelganger Underwater",
            WorldMap::DoppelgangerCrystalCave => "Doppelganger Crystal Cave",
            WorldMap::ImperialGuardian4 => "Imperial Guardian 4",
            WorldMap::ImperialGuardian3 => "Imperial Guardian 3",
            WorldMap::ImperialGuardian2 => "Imperial Guardian 2",
            WorldMap::ImperialGuardian1 => "Imperial Guardian 1",
            WorldMap::NewLoginScene1 => "New Login Scene 1",
            WorldMap::EventSquare => "Event Square",
            WorldMap::NewLoginScene2 => "New Login Scene 2",
            WorldMap::NewCharacterScene2 => "New Character Scene 2",
            WorldMap::LorenMarket => "Loren Market",
            WorldMap::Karutan1 => "Karutan 1",
            WorldMap::Karutan2 => "Karutan 2",
            WorldMap::DoppelgangerRenewal => "Doppelganger Renewal",
            WorldMap::NewArena => "New Arena",
            WorldMap::Acheron => "Acheron",
            WorldMap::Acheron2 => "Acheron 2",
            WorldMap::UrukMountain3 => "Uruk Mountain 3",
            WorldMap::UrukMountain2 => "Uruk Mountain 2",
            WorldMap::Debenter => "Debenter",
            WorldMap::DebenterArcaBattle => "Debenter Arca Battle",
            WorldMap::IllusionTempleLeague => "Illusion Temple League",
            WorldMap::IllusionTempleLeague2 => "Illusion Temple League 2",
            WorldMap::UrukMountain => "Uruk Mountain",
            WorldMap::TormentedSquare => "Tormented Square",
            WorldMap::Nars => "Nars",
            WorldMap::Ferea => "Ferea",
            WorldMap::NixiesLake => "Nixies Lake",
            WorldMap::LorenMarketS6 => "Loren Market",
            WorldMap::DeepDungeon1 => "Deep Dungeon 1",
            WorldMap::DeepDungeon2 => "Deep Dungeon 2",
            WorldMap::DeepDungeon3 => "Deep Dungeon 3",
            WorldMap::DeepDungeon4 => "Deep Dungeon 4",
            WorldMap::DeepDungeon5 => "Deep Dungeon 5",
            WorldMap::PlaceOfQualification => "Place of Qualification",
            WorldMap::SwampOfDarkness => "Swamp of Darkness",
            WorldMap::KuberaMine1 => "Kubera Mine",
            WorldMap::KuberaMine2 => "Kubera Mine 2",
            WorldMap::AbyssOfAtlans => "Abyss of Atlans",
            WorldMap::AbyssOfAtlans2 => "Abyss of Atlans 2",
            WorldMap::AbyssOfAtlans3 => "Abyss of Atlans 3",
            WorldMap::ScorchedTunnels => "Scorched Tunnels",
            WorldMap::RedSmokeIcarus => "Red Smoke Icarus",
            WorldMap::TempleOfArnil => "Temple of Arnil",
            WorldMap::AshenAida => "Ashen Aida",
            WorldMap::OldKethotum => "Old Kethotum",
            WorldMap::BlazeKethotum => "Blaze Kethotum",
            WorldMap::KanturuUndergrounds => "Kanturu Undergrounds",
            WorldMap::IgnisVolcano => "Ignis Volcano",
            WorldMap::BossBattleZone => "Boss Battle Zone",
            WorldMap::BloodyTarkan => "Bloody Tarkan",
            WorldMap::TormentaIsland => "Tormenta Island",
            WorldMap::DoppelgangerIceZoneNew => "Doppelganger Ice Zone (New)",
        }
    }

    /// Returns the World folder name for this map (e.g. "World1" for Lorencia)
    pub fn world_folder(&self) -> String {
        format!("World{}", *self as u8)
    }

    /// Returns true if this is a login/character selection scene
    pub fn is_login_scene(&self) -> bool {
        matches!(
            self,
            WorldMap::LoginScene
                | WorldMap::CharacterScene
                | WorldMap::NewLoginScene1
                | WorldMap::NewLoginScene2
                | WorldMap::NewCharacterScene2
        )
    }

    /// Returns true if this is a PvP area
    pub fn is_pvp_area(&self) -> bool {
        matches!(
            self,
            WorldMap::Arena
                | WorldMap::Vulcanus
                | WorldMap::DuelArena
                | WorldMap::DevilSquare
                | WorldMap::NewArena
        )
    }

    /// Returns true if this is a special event dungeon
    pub fn is_event_dungeon(&self) -> bool {
        matches!(
            self,
            WorldMap::BloodCastle1
                | WorldMap::BloodCastle2
                | WorldMap::BloodCastle3
                | WorldMap::BloodCastle4
                | WorldMap::BloodCastle5
                | WorldMap::BloodCastle6
                | WorldMap::BloodCastle7
                | WorldMap::BloodCastle8
                | WorldMap::ChaosCastle1
                | WorldMap::ChaosCastle2
                | WorldMap::ChaosCastle3
                | WorldMap::ChaosCastle4
                | WorldMap::ChaosCastle5
                | WorldMap::ChaosCastle6
                | WorldMap::ChaosCastle7
                | WorldMap::DevilSquare
                | WorldMap::DevilSquare2
                | WorldMap::Crywolf
                | WorldMap::Kanturu
                | WorldMap::KanturuRemain
                | WorldMap::RefineTower
                | WorldMap::IllusionTemple1
                | WorldMap::IllusionTemple2
                | WorldMap::IllusionTemple3
                | WorldMap::IllusionTemple4
                | WorldMap::IllusionTemple5
                | WorldMap::IllusionTempleLeague
                | WorldMap::IllusionTempleLeague2
                | WorldMap::DoppelgangerIceZone
                | WorldMap::DoppelgangerBlazeZone
                | WorldMap::DoppelgangerUnderwater
                | WorldMap::DoppelgangerCrystalCave
                | WorldMap::DoppelgangerRenewal
                | WorldMap::DoppelgangerIceZoneNew
                | WorldMap::ImperialGuardian1
                | WorldMap::ImperialGuardian2
                | WorldMap::ImperialGuardian3
                | WorldMap::ImperialGuardian4
                | WorldMap::TormentedSquare
                | WorldMap::BossBattleZone
        )
    }

    /// Tries to create a WorldMap from a World folder ID (e.g. 1 for World1/Lorencia)
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(WorldMap::Unk0),
            1 => Some(WorldMap::Lorencia),
            2 => Some(WorldMap::Dungeon),
            3 => Some(WorldMap::Devias),
            4 => Some(WorldMap::Noria),
            5 => Some(WorldMap::LostTower),
            6 => Some(WorldMap::Exile),
            7 => Some(WorldMap::Arena),
            8 => Some(WorldMap::Atlans),
            9 => Some(WorldMap::Tarkan),
            10 => Some(WorldMap::DevilSquare),
            11 => Some(WorldMap::Icarus),
            12 => Some(WorldMap::BloodCastle1),
            13 => Some(WorldMap::BloodCastle2),
            14 => Some(WorldMap::BloodCastle3),
            15 => Some(WorldMap::BloodCastle4),
            16 => Some(WorldMap::BloodCastle5),
            17 => Some(WorldMap::BloodCastle6),
            18 => Some(WorldMap::BloodCastle7),
            19 => Some(WorldMap::ChaosCastle1),
            20 => Some(WorldMap::ChaosCastle2),
            21 => Some(WorldMap::ChaosCastle3),
            22 => Some(WorldMap::ChaosCastle4),
            23 => Some(WorldMap::ChaosCastle5),
            24 => Some(WorldMap::ChaosCastle6),
            25 => Some(WorldMap::Kalima1),
            26 => Some(WorldMap::Kalima2),
            27 => Some(WorldMap::Kalima3),
            28 => Some(WorldMap::Kalima4),
            29 => Some(WorldMap::Kalima5),
            30 => Some(WorldMap::Kalima6),
            31 => Some(WorldMap::ValleyOfLoren),
            32 => Some(WorldMap::LandOfTrials),
            33 => Some(WorldMap::DevilSquare2),
            34 => Some(WorldMap::Aida),
            35 => Some(WorldMap::Crywolf),
            37 => Some(WorldMap::Kalima7),
            38 => Some(WorldMap::Kanturu),
            39 => Some(WorldMap::KanturuRemain),
            40 => Some(WorldMap::RefineTower),
            41 => Some(WorldMap::SilentMap),
            42 => Some(WorldMap::BalgassBarracks),
            43 => Some(WorldMap::BalgassRefuge),
            46 => Some(WorldMap::IllusionTemple1),
            47 => Some(WorldMap::IllusionTemple2),
            48 => Some(WorldMap::IllusionTemple3),
            49 => Some(WorldMap::IllusionTemple4),
            50 => Some(WorldMap::IllusionTemple5),
            51 => Some(WorldMap::Elbeland2),
            52 => Some(WorldMap::Elbeland),
            53 => Some(WorldMap::BloodCastle8),
            54 => Some(WorldMap::ChaosCastle7),
            55 => Some(WorldMap::CharacterScene),
            56 => Some(WorldMap::LoginScene),
            57 => Some(WorldMap::SwampOfPeace),
            58 => Some(WorldMap::Raklion),
            59 => Some(WorldMap::RaklionBoss),
            63 => Some(WorldMap::SantaVillage),
            64 => Some(WorldMap::Vulcanus),
            65 => Some(WorldMap::DuelArena),
            66 => Some(WorldMap::DoppelgangerIceZone),
            67 => Some(WorldMap::DoppelgangerBlazeZone),
            68 => Some(WorldMap::DoppelgangerUnderwater),
            69 => Some(WorldMap::DoppelgangerCrystalCave),
            70 => Some(WorldMap::ImperialGuardian4),
            71 => Some(WorldMap::ImperialGuardian3),
            72 => Some(WorldMap::ImperialGuardian2),
            73 => Some(WorldMap::ImperialGuardian1),
            74 => Some(WorldMap::NewLoginScene1),
            75 => Some(WorldMap::EventSquare),
            78 => Some(WorldMap::NewLoginScene2),
            79 => Some(WorldMap::NewCharacterScene2),
            80 => Some(WorldMap::LorenMarket),
            81 => Some(WorldMap::Karutan1),
            82 => Some(WorldMap::Karutan2),
            83 => Some(WorldMap::DoppelgangerRenewal),
            84 => Some(WorldMap::NewArena),
            92 => Some(WorldMap::Acheron),
            93 => Some(WorldMap::Acheron2),
            94 => Some(WorldMap::UrukMountain3),
            95 => Some(WorldMap::UrukMountain2),
            96 => Some(WorldMap::Debenter),
            97 => Some(WorldMap::DebenterArcaBattle),
            99 => Some(WorldMap::IllusionTempleLeague),
            100 => Some(WorldMap::IllusionTempleLeague2),
            101 => Some(WorldMap::UrukMountain),
            103 => Some(WorldMap::TormentedSquare),
            111 => Some(WorldMap::Nars),
            113 => Some(WorldMap::Ferea),
            114 => Some(WorldMap::NixiesLake),
            115 => Some(WorldMap::LorenMarketS6),
            117 => Some(WorldMap::DeepDungeon1),
            118 => Some(WorldMap::DeepDungeon2),
            119 => Some(WorldMap::DeepDungeon3),
            120 => Some(WorldMap::DeepDungeon4),
            121 => Some(WorldMap::DeepDungeon5),
            122 => Some(WorldMap::PlaceOfQualification),
            123 => Some(WorldMap::SwampOfDarkness),
            124 => Some(WorldMap::KuberaMine1),
            125 => Some(WorldMap::KuberaMine2),
            129 => Some(WorldMap::AbyssOfAtlans),
            130 => Some(WorldMap::AbyssOfAtlans2),
            131 => Some(WorldMap::AbyssOfAtlans3),
            132 => Some(WorldMap::ScorchedTunnels),
            133 => Some(WorldMap::RedSmokeIcarus),
            134 => Some(WorldMap::TempleOfArnil),
            135 => Some(WorldMap::AshenAida),
            136 => Some(WorldMap::OldKethotum),
            137 => Some(WorldMap::BlazeKethotum),
            138 => Some(WorldMap::KanturuUndergrounds),
            139 => Some(WorldMap::IgnisVolcano),
            140 => Some(WorldMap::BossBattleZone),
            141 => Some(WorldMap::BloodyTarkan),
            142 => Some(WorldMap::TormentaIsland),
            143 => Some(WorldMap::DoppelgangerIceZoneNew),
            _ => None,
        }
    }
}

impl std::fmt::Display for WorldMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_map_ids_match_folders() {
        // IDs must match World folder numbers
        assert_eq!(WorldMap::Lorencia as u8, 1); // World1
        assert_eq!(WorldMap::Dungeon as u8, 2); // World2
        assert_eq!(WorldMap::Arena as u8, 7); // World7
        assert_eq!(WorldMap::Icarus as u8, 11); // World11
        assert_eq!(WorldMap::BloodCastle1 as u8, 12); // World12
        assert_eq!(WorldMap::LoginScene as u8, 56); // World56
        assert_eq!(WorldMap::Acheron as u8, 92); // World92
        assert_eq!(WorldMap::DoppelgangerIceZoneNew as u8, 143); // World143
    }

    #[test]
    fn test_world_folder() {
        assert_eq!(WorldMap::Lorencia.world_folder(), "World1");
        assert_eq!(WorldMap::LoginScene.world_folder(), "World56");
        assert_eq!(WorldMap::Acheron.world_folder(), "World92");
    }

    #[test]
    fn test_from_id() {
        assert_eq!(WorldMap::from_id(1), Some(WorldMap::Lorencia));
        assert_eq!(WorldMap::from_id(56), Some(WorldMap::LoginScene));
        assert_eq!(WorldMap::from_id(0), Some(WorldMap::Unk0));
        assert_eq!(WorldMap::from_id(255), None);
    }

    #[test]
    fn test_renamed_maps() {
        assert_eq!(WorldMap::from_id(6), Some(WorldMap::Exile));
        assert_eq!(WorldMap::Exile.name(), "Exile");
        assert_eq!(WorldMap::from_id(7), Some(WorldMap::Arena));
        assert_eq!(WorldMap::Arena.name(), "Arena");
        assert_eq!(WorldMap::from_id(8), Some(WorldMap::Atlans));
        assert_eq!(WorldMap::Atlans.name(), "Atlans");
        assert_eq!(WorldMap::from_id(11), Some(WorldMap::Icarus));
        assert_eq!(WorldMap::Icarus.name(), "Icarus");
        assert_eq!(WorldMap::from_id(25), Some(WorldMap::Kalima1));
        assert_eq!(WorldMap::from_id(31), Some(WorldMap::ValleyOfLoren));
        assert_eq!(WorldMap::from_id(35), Some(WorldMap::Crywolf));
        assert_eq!(WorldMap::from_id(58), Some(WorldMap::Raklion));
        assert_eq!(WorldMap::from_id(64), Some(WorldMap::Vulcanus));
    }

    #[test]
    fn test_new_maps() {
        assert_eq!(WorldMap::from_id(28), Some(WorldMap::Kalima4));
        assert_eq!(WorldMap::from_id(83), Some(WorldMap::DoppelgangerRenewal));
        assert_eq!(WorldMap::from_id(92), Some(WorldMap::Acheron));
        assert_eq!(WorldMap::from_id(101), Some(WorldMap::UrukMountain));
        assert_eq!(WorldMap::from_id(111), Some(WorldMap::Nars));
        assert_eq!(WorldMap::from_id(113), Some(WorldMap::Ferea));
        assert_eq!(WorldMap::from_id(117), Some(WorldMap::DeepDungeon1));
        assert_eq!(WorldMap::from_id(129), Some(WorldMap::AbyssOfAtlans));
        assert_eq!(
            WorldMap::from_id(143),
            Some(WorldMap::DoppelgangerIceZoneNew)
        );
    }

    #[test]
    fn test_is_login_scene() {
        assert!(WorldMap::LoginScene.is_login_scene());
        assert!(WorldMap::CharacterScene.is_login_scene());
        assert!(WorldMap::NewCharacterScene2.is_login_scene());
        assert!(!WorldMap::EventSquare.is_login_scene());
        assert!(!WorldMap::Lorencia.is_login_scene());
    }

    #[test]
    fn test_is_pvp_area() {
        assert!(WorldMap::Arena.is_pvp_area());
        assert!(WorldMap::Vulcanus.is_pvp_area());
        assert!(!WorldMap::Lorencia.is_pvp_area());
    }

    #[test]
    fn test_is_event_dungeon() {
        assert!(WorldMap::BloodCastle8.is_event_dungeon());
        assert!(WorldMap::ChaosCastle7.is_event_dungeon());
        assert!(WorldMap::IllusionTemple1.is_event_dungeon());
        assert!(WorldMap::DoppelgangerRenewal.is_event_dungeon());
        assert!(WorldMap::TormentedSquare.is_event_dungeon());
        assert!(!WorldMap::Lorencia.is_event_dungeon());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", WorldMap::ValleyOfLoren), "Valley of Loren");
        assert_eq!(format!("{}", WorldMap::Raklion), "Raklion");
    }
}
