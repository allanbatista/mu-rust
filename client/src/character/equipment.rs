use super::types::{BodySlot, BodyType, CharacterClass};

/// Equipment set variants available in the character viewer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EquipmentSet {
    /// `{slot}_class_{class_id:02}.glb`
    ClassDefault,
    /// `{slot}_{body_type}_{set_id:02}.glb`
    Standard(u8),
    /// `cw_{slot}_{body_type}_{id:02}.glb` (male only)
    ClassWar(u8),
    /// `hdk_{slot}_{body_type}_{id:02}.glb` (male only)
    HighDarkKnight(u8),
    /// `mask_helm_{body_type}_{id:02}.glb` (helm only, male only)
    Mask(u8),
    /// `{slot}_elf_c_{id:02}.glb` (elf only)
    ElfCosmic(u8),
}

impl EquipmentSet {
    /// Return all equipment sets available for a given body type.
    pub fn available_for(body_type: BodyType) -> Vec<EquipmentSet> {
        let mut sets = vec![EquipmentSet::ClassDefault];

        match body_type {
            BodyType::Male => {
                // Standard: 01-10, 16-29
                for id in 1..=10 {
                    sets.push(EquipmentSet::Standard(id));
                }
                for id in 16..=29 {
                    sets.push(EquipmentSet::Standard(id));
                }
                // ClassWar: 01-05
                for id in 1..=5 {
                    sets.push(EquipmentSet::ClassWar(id));
                }
                // HighDarkKnight: 01-05
                for id in 1..=5 {
                    sets.push(EquipmentSet::HighDarkKnight(id));
                }
                // Mask: 01, 06, 07, 09, 10
                for id in [1, 6, 7, 9, 10] {
                    sets.push(EquipmentSet::Mask(id));
                }
            }
            BodyType::Elf => {
                // Standard elf: 01-05
                for id in 1..=5 {
                    sets.push(EquipmentSet::Standard(id));
                }
                // Elf cosmic: 01-02
                for id in 1..=2 {
                    sets.push(EquipmentSet::ElfCosmic(id));
                }
            }
            BodyType::Monk => {
                // Standard monk: 01-04
                for id in 1..=4 {
                    sets.push(EquipmentSet::Standard(id));
                }
            }
        }

        sets
    }

    /// GLB asset path for a specific slot in this equipment set.
    pub fn glb_path(&self, slot: BodySlot, body_type: BodyType, class: CharacterClass) -> String {
        match self {
            EquipmentSet::ClassDefault => {
                format!(
                    "data/player/{}_class_{:02}.glb",
                    slot.prefix(),
                    class.class_id()
                )
            }
            EquipmentSet::Standard(id) => {
                format!(
                    "data/player/{}_{}_{:02}.glb",
                    slot.prefix(),
                    body_type.slug(),
                    id
                )
            }
            EquipmentSet::ClassWar(id) => {
                format!(
                    "data/player/cw_{}_{:02}.glb",
                    slot_with_body(slot, body_type),
                    id
                )
            }
            EquipmentSet::HighDarkKnight(id) => {
                format!(
                    "data/player/hdk_{}_{:02}.glb",
                    slot_with_body(slot, body_type),
                    id
                )
            }
            EquipmentSet::Mask(id) => {
                // Mask is helm-only; for non-helm slots fall back to class default.
                if slot == BodySlot::Helm {
                    format!("data/player/mask_helm_{}_{:02}.glb", body_type.slug(), id)
                } else {
                    format!(
                        "data/player/{}_class_{:02}.glb",
                        slot.prefix(),
                        class.class_id()
                    )
                }
            }
            EquipmentSet::ElfCosmic(id) => {
                format!("data/player/{}_elf_c_{:02}.glb", slot.prefix(), id)
            }
        }
    }

    /// Human-readable name for the UI dropdown.
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

fn slot_with_body(slot: BodySlot, body_type: BodyType) -> String {
    format!("{}_{}", slot.prefix(), body_type.slug())
}
