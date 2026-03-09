//! Convenience accessors over `Utterance` dependent tiers.
//!
//! Accessors return the first matching tier of each type. This matches common
//! CHAT expectations (one instance per tier kind) while preserving full tier
//! order in `Utterance::dependent_tiers` for tooling that needs every entry.
//!
//! Related CHAT sections:
//! - <https://talkbank.org/0info/manuals/CHAT.html#Dependent_Tiers>
//! - <https://talkbank.org/0info/manuals/CHAT.html#Main_Line>

use crate::LanguageMetadata;
use crate::model::dependent_tier::{
    ActTier, CodTier, DependentTier, GraTier, MorTier, PhoTier, PhoalnTier, SinTier, SylTier,
};

use super::Utterance;

impl Utterance {
    /// Return the first `%mor` tier, if present.
    pub fn mor_tier(&self) -> Option<&MorTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Mor(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first mutable `%mor` tier, if present.
    pub fn mor_tier_mut(&mut self) -> Option<&mut MorTier> {
        self.dependent_tiers.iter_mut().find_map(|t| match t {
            DependentTier::Mor(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%gra` tier, if present.
    pub fn gra_tier(&self) -> Option<&GraTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Gra(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%wor` tier, if present.
    pub fn wor_tier(&self) -> Option<&crate::model::dependent_tier::WorTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Wor(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%pho` tier, if present.
    pub fn pho_tier(&self) -> Option<&PhoTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Pho(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%mod` tier, if present.
    ///
    /// `%mod` uses the same concrete type as `%pho` in the current model.
    pub fn mod_tier(&self) -> Option<&PhoTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Mod(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%sin` tier, if present.
    pub fn sin_tier(&self) -> Option<&SinTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Sin(tier) => Some(tier),
            _ => None,
        })
    }

    /// Convenience alias returning an owned clone of the first `%mor` tier.
    pub fn mor(&self) -> Option<MorTier> {
        self.mor_tier().cloned()
    }

    /// Convenience alias returning an owned clone of the first `%gra` tier.
    pub fn gra(&self) -> Option<GraTier> {
        self.gra_tier().cloned()
    }

    /// Convenience alias returning an owned clone of the first `%pho` tier.
    pub fn pho(&self) -> Option<PhoTier> {
        self.pho_tier().cloned()
    }

    /// Convenience alias returning an owned clone of the first `%sin` tier.
    pub fn sin(&self) -> Option<SinTier> {
        self.sin_tier().cloned()
    }

    /// Return the first `%act` tier, if present.
    pub fn act(&self) -> Option<&ActTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Act(a) => Some(a),
            _ => None,
        })
    }

    /// Return the first `%cod` tier, if present.
    pub fn cod(&self) -> Option<&CodTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Cod(c) => Some(c),
            _ => None,
        })
    }

    /// Return the first `%com` tier, if present.
    pub fn com(&self) -> Option<&crate::model::dependent_tier::ComTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Com(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%exp` tier, if present.
    pub fn exp(&self) -> Option<&crate::model::dependent_tier::ExpTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Exp(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%add` tier, if present.
    pub fn add(&self) -> Option<&crate::model::dependent_tier::AddTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Add(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%spa` tier, if present.
    pub fn spa(&self) -> Option<&crate::model::dependent_tier::SpaTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Spa(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%sit` tier, if present.
    pub fn sit(&self) -> Option<&crate::model::dependent_tier::SitTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Sit(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%gpx` tier, if present.
    pub fn gpx(&self) -> Option<&crate::model::dependent_tier::GpxTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Gpx(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%int` tier, if present.
    pub fn int(&self) -> Option<&crate::model::dependent_tier::IntTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Int(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%ort` tier payload, if present.
    pub fn ort(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Ort(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%eng` tier payload, if present.
    pub fn eng(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Eng(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%gls` tier payload, if present.
    pub fn gls(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Gls(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%alt` tier payload, if present.
    pub fn alt(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Alt(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%coh` tier payload, if present.
    pub fn coh(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Coh(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%def` tier payload, if present.
    pub fn def(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Def(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%err` tier payload, if present.
    pub fn err(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Err(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%fac` tier payload, if present.
    pub fn fac(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Fac(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%flo` tier payload, if present.
    pub fn flo(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Flo(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%par` tier payload, if present.
    pub fn par(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Par(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%tim` tier payload, if present.
    pub fn tim(&self) -> Option<&str> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Tim(s) => Some(s.as_str()),
            _ => None,
        })
    }

    /// Return the first `%modsyl` / `%xmodsyl` tier, if present.
    pub fn modsyl_tier(&self) -> Option<&SylTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Modsyl(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%phosyl` / `%xphosyl` tier, if present.
    pub fn phosyl_tier(&self) -> Option<&SylTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Phosyl(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return the first `%phoaln` / `%xphoaln` tier, if present.
    pub fn phoaln_tier(&self) -> Option<&PhoalnTier> {
        self.dependent_tiers.iter().find_map(|t| match t {
            DependentTier::Phoaln(tier) => Some(tier),
            _ => None,
        })
    }

    /// Return computed per-word language metadata, if available.
    pub fn computed_language_metadata(&self) -> Option<&LanguageMetadata> {
        self.language_metadata.as_computed()
    }
}
