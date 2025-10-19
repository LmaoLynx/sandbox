use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumString)]
// todo: repr u16 for compactness?
#[strum(serialize_all="SCREAMING_SNAKE_CASE")]
pub enum Mod {
    TargetedShame,
    Flinch,
    #[strum(serialize="WILD")]
    Mild,
    Reverberating,
    Fireproof,
    Soundproof,
    Shelled,
    #[strum(serialize="LIFE_OF_PARTY")]
    LifeOfTheParty,
    Gravity,
    NightVision,
    #[strum(serialize="EXTRA_STRIKE")]
    FourthStrike,
    #[strum(serialize="DEBT_THREE")]
    DebtU,
    #[strum(serialize="MARKED")]
    Unstable,
    Superallergic,
    Spicy,
    HeatingUp,
    #[strum(serialize="ON_FIRE")]
    RedHot,
    Minimized,
    Electric,
    RefinancedDebt,
    Flickering,
    Stable,
    HomeFieldAdvantage,
    BaseInstincts,
    AffinityForCrows,
    Growth,
    ConsolidatedDebt,
    Repeating,
    #[strum(serialize="EXTRA_BASE")]
    FifthBase,
    #[strum(serialize="LOVE")]
    Charm,
    #[strum(serialize="FLIICKERRRIIING")]
    SuperFlickering, //not bothering to type the actual mod name correctly
    Squiddish,
    Siphon,
    FriendOfCrows,
    FireEater,
    Magmatic,
    HoneyRoasted,
    Traveling,
    Haunted,
    Sealant,
    Blaserunning,
    BirdSeed,
    Superyummy,
    Overperforming,
    Underperforming,
    WalkInThePark,
    ONo,
    Wired,
    Tired,
    #[strum(serialize="COFFEE_RALLY")]
    FreeRefill,
    TripleThreat,
    Perk,
    Elsewhere,
    Scattered,
    #[strum(serialize="SWIM_BLADDER")]
    Flippers,
    Earlbirds,
    LateToTheParty,
    #[strum(serialize="WANDERER")]
    Roaming,
    HardBoiled,
    Undersea,
    Ambush
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModLifetime {
    Game,
    Week,
    Season,
    LegendaryItem,
    Permanent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModWithLifetime {
    lifetime: ModLifetime,
    the_mod: Mod, // mod is a keyword lmao
}

#[derive(Debug, Clone)]
pub struct Mods {
    // todo: store this as a set? or a smallvec?
    // we only have <10 entries so i think searching a vec might be faster anyway
    mods: Vec<ModWithLifetime>,
}

impl Mods {
    pub fn new() -> Mods {
        Mods { mods: Vec::new() }
    }

    pub fn has(&self, m: Mod) -> bool {
        self.mods.iter().any(|x| x.the_mod == m)
    }

    pub fn add(&mut self, m: Mod, lifetime: ModLifetime) {
        let ml = ModWithLifetime {
            the_mod: m,
            lifetime: lifetime,
        };
        if !self.mods.contains(&ml) {
            self.mods.push(ml);
        }
    }

    pub fn remove(&mut self, m: Mod) {
        self.mods.retain(|x| x.the_mod != m)
    }

    pub fn clear_game(&mut self) {
        self.mods.retain(|x| x.lifetime != ModLifetime::Game);
    }

    pub fn clear_weekly(&mut self) {
        self.mods.retain(|x| x.lifetime != ModLifetime::Week);
    }

    pub fn clear_season(&mut self) {
        self.mods.retain(|x| x.lifetime != ModLifetime::Season);
    }

    pub fn clear_legendary_item(&mut self) {
        self.mods.retain(|x| x.lifetime != ModLifetime::LegendaryItem);
    }
}
