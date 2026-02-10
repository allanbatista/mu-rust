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

    /// 1-based class ID matching the C++ CLASS_TYPE enum (+1).
    /// Used for `_class_{id:02}` equipment file naming.
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

    /// Return slots available for a given body type.
    pub fn slots_for(body_type: BodyType) -> &'static [BodySlot] {
        match body_type {
            // Monk has no gloves
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

    /// GLB asset path for the default (tier 01) body part.
    pub fn default_glb_path(&self, body_type: BodyType) -> String {
        format!("data/player/{}_{}_01.glb", self.prefix(), body_type.slug())
    }
}

/// Marker component for body part entities.
#[derive(Component)]
pub struct BodyPartMarker {
    pub slot: BodySlot,
}

/// Marker for the character root entity.
#[derive(Component)]
pub struct CharacterRoot;
