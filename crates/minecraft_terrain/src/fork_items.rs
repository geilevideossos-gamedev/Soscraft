use std::collections::HashMap;
use std::sync::OnceLock;

use crate::creative_tabs::{CreativeTabs, TabEntry, Variants};

pub const NAMESPACE: &str = "iw4l:";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Gun,
    Ammo,
    Lethal,
    Tactical,
    Attachment,
    Part,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bucket {
    Pistol,
    MachinePistol,
    Smg,
    AssaultRifle,
    Shotgun,
    Special,
    Sniper,
    Lmg,
    Launcher,
    Lethal,
    Tactical,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    Early,
    Middle,
    Diamond,
    Nether,
    Late,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Icon {
    KillIcon,
    Material(&'static str),
    Sprite {
        item: &'static str,
        tint: Option<[u8; 3]>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForkItem {
    pub id: &'static str,
    pub kind: Kind,
    pub weapon: Option<&'static str>,
    pub bucket: Bucket,
    pub ammo: Option<&'static str>,
    pub max_stack: u8,
    pub name: &'static str,
    pub tier: Tier,
    pub icon: Icon,
    pub fire_proof: bool,
    pub aliases: &'static [&'static str],
}

impl ForkItem {
    pub fn path(&self) -> &'static str {
        &self.id[NAMESPACE.len()..]
    }

    pub fn language_key(&self) -> String {
        format!("item.iw4l.{}", self.path())
    }

    pub fn creative_order(&self) -> usize {
        ROWS.iter()
            .position(|row| row.id == self.id)
            .unwrap_or(usize::MAX)
    }

    const fn fire_proof(self) -> Self {
        ForkItem {
            fire_proof: true,
            ..self
        }
    }
}

pub const PISTOL_AMMO: &str = "iw4l:pistol_ammo";
pub const RIFLE_AMMO: &str = "iw4l:rifle_ammo";
pub const LMG_AMMO: &str = "iw4l:lmg_ammo";
pub const SHOTGUN_SHELLS: &str = "iw4l:shotgun_shells";
pub const SNIPER_ROUNDS: &str = "iw4l:sniper_rounds";
pub const EXPLOSIVE_ROUND: &str = "iw4l:explosive_round";
pub const ROCKET: &str = "iw4l:rocket";
pub const THROWING_KNIFE: &str = "iw4l:throwing_knife";

pub const AMMO_TYPES: [&str; 7] = [
    PISTOL_AMMO,
    RIFLE_AMMO,
    LMG_AMMO,
    SHOTGUN_SHELLS,
    SNIPER_ROUNDS,
    EXPLOSIVE_ROUND,
    ROCKET,
];

const fn gun(
    id: &'static str,
    weapon: &'static str,
    bucket: Bucket,
    ammo: Option<&'static str>,
    name: &'static str,
    tier: Tier,
) -> ForkItem {
    ForkItem {
        id,
        kind: Kind::Gun,
        weapon: Some(weapon),
        bucket,
        ammo,
        max_stack: 1,
        name,
        tier,
        icon: Icon::KillIcon,
        fire_proof: false,
        aliases: &[],
    }
}

const fn grenade(
    id: &'static str,
    weapon: &'static str,
    kind: Kind,
    name: &'static str,
    tier: Tier,
    icon: &'static str,
) -> ForkItem {
    ForkItem {
        id,
        kind,
        weapon: Some(weapon),
        bucket: match kind {
            Kind::Tactical => Bucket::Tactical,
            _ => Bucket::Lethal,
        },
        ammo: None,
        max_stack: 64,
        name,
        tier,
        icon: Icon::Material(icon),
        fire_proof: false,
        aliases: &[],
    }
}

const fn ammo(
    id: &'static str,
    name: &'static str,
    max_stack: u8,
    tier: Tier,
    item: &'static str,
    tint: Option<[u8; 3]>,
) -> ForkItem {
    ForkItem {
        id,
        kind: Kind::Ammo,
        weapon: None,
        bucket: Bucket::Other,
        ammo: None,
        max_stack,
        name,
        tier,
        icon: Icon::Sprite { item, tint },
        fire_proof: false,
        aliases: &[],
    }
}

const fn attachment(
    id: &'static str,
    name: &'static str,
    aliases: &'static [&'static str],
    tier: Tier,
    icon: Icon,
) -> ForkItem {
    ForkItem {
        id,
        kind: Kind::Attachment,
        weapon: None,
        bucket: Bucket::Other,
        ammo: None,
        max_stack: 64,
        name,
        tier,
        icon,
        fire_proof: false,
        aliases,
    }
}

const fn part(id: &'static str, name: &'static str, item: &'static str) -> ForkItem {
    ForkItem {
        id,
        kind: Kind::Part,
        weapon: None,
        bucket: Bucket::Other,
        ammo: None,
        max_stack: 64,
        name,
        tier: Tier::Early,
        icon: Icon::Sprite { item, tint: None },
        fire_proof: false,
        aliases: &[],
    }
}

const P: Option<&str> = Some(PISTOL_AMMO);
const R: Option<&str> = Some(RIFLE_AMMO);
const L: Option<&str> = Some(LMG_AMMO);
const S: Option<&str> = Some(SHOTGUN_SHELLS);
const N: Option<&str> = Some(SNIPER_ROUNDS);
const X: Option<&str> = Some(EXPLOSIVE_ROUND);
const K: Option<&str> = Some(ROCKET);

const BRASS: [u8; 3] = [222, 184, 96];
const COPPER: [u8; 3] = [214, 120, 78];
const GREY: [u8; 3] = [150, 150, 156];
const GOLD: [u8; 3] = [250, 214, 70];
const RED: [u8; 3] = [196, 52, 44];
const GREEN: [u8; 3] = [96, 200, 84];

use Bucket::{AssaultRifle, Launcher, Lmg, MachinePistol, Pistol, Shotgun, Smg, Sniper, Special};
use Tier::{Diamond, Early, Late, Middle, Nether};

static ROWS: [ForkItem; 94] = [
    gun(
        "iw4l:beretta",
        "iw4:weapon/beretta_mp",
        Pistol,
        P,
        "M9",
        Early,
    ),
    gun("iw4l:usp", "iw4:weapon/usp_mp", Pistol, P, "USP .45", Early),
    gun(
        "iw4l:coltanaconda",
        "iw4:weapon/coltanaconda_mp",
        Pistol,
        P,
        ".44 Magnum",
        Early,
    ),
    gun(
        "iw4l:deserteagle",
        "iw4:weapon/deserteagle_mp",
        Pistol,
        P,
        "Desert Eagle",
        Early,
    ),
    gun(
        "iw4l:deserteaglegold",
        "iw4:weapon/deserteaglegold_mp",
        Pistol,
        P,
        "Gold Desert Eagle",
        Early,
    ),
    gun(
        "iw4l:t5_cz75",
        "t5:weapon/cz75_mp",
        Pistol,
        P,
        "CZ75",
        Early,
    ),
    gun(
        "iw4l:t5_python",
        "t5:weapon/python_mp",
        Pistol,
        P,
        "Python",
        Early,
    ),
    gun(
        "iw4l:iw5_44magnum",
        "iw5:weapon/iw5_44magnum_mp",
        Pistol,
        P,
        ".44 Magnum",
        Early,
    ),
    gun(
        "iw4l:glock",
        "iw4:weapon/glock_mp",
        MachinePistol,
        P,
        "G18",
        Early,
    ),
    gun(
        "iw4l:pp2000",
        "iw4:weapon/pp2000_mp",
        MachinePistol,
        P,
        "PP2000",
        Early,
    ),
    gun(
        "iw4l:tmp",
        "iw4:weapon/tmp_mp",
        MachinePistol,
        P,
        "TMP",
        Early,
    ),
    gun(
        "iw4l:beretta393",
        "iw4:weapon/beretta393_mp",
        MachinePistol,
        P,
        "M93 Raffica",
        Early,
    ),
    gun(
        "iw4l:iw5_fmg9",
        "iw5:weapon/iw5_fmg9_mp",
        MachinePistol,
        P,
        "FMG9",
        Early,
    ),
    gun("iw4l:mp5k", "iw4:weapon/mp5k_mp", Smg, P, "MP5K", Early),
    gun("iw4l:uzi", "iw4:weapon/uzi_mp", Smg, P, "Mini-Uzi", Early),
    gun("iw4l:p90", "iw4:weapon/p90_mp", Smg, P, "P90", Early),
    gun("iw4l:kriss", "iw4:weapon/kriss_mp", Smg, P, "Vector", Early),
    gun("iw4l:ump45", "iw4:weapon/ump45_mp", Smg, P, "UMP45", Early),
    gun(
        "iw4l:t5_spectre",
        "t5:weapon/spectre_mp",
        Smg,
        P,
        "Spectre",
        Early,
    ),
    gun(
        "iw4l:t5_ak74u",
        "t5:weapon/ak74u_mp",
        Smg,
        P,
        "AK-74u",
        Early,
    ),
    gun(
        "iw4l:iw5_mp7",
        "iw5:weapon/iw5_mp7_mp",
        Smg,
        P,
        "MP7",
        Early,
    ),
    gun(
        "iw4l:iw5_pp90m1",
        "iw5:weapon/iw5_pp90m1_mp",
        Smg,
        P,
        "PP90M1",
        Early,
    ),
    gun(
        "iw4l:m4",
        "iw4:weapon/m4_mp",
        AssaultRifle,
        R,
        "M4A1",
        Middle,
    ),
    gun(
        "iw4l:famas",
        "iw4:weapon/famas_mp",
        AssaultRifle,
        R,
        "FAMAS",
        Middle,
    ),
    gun(
        "iw4l:scar",
        "iw4:weapon/scar_mp",
        AssaultRifle,
        R,
        "SCAR-H",
        Middle,
    ),
    gun(
        "iw4l:tar21",
        "iw4:weapon/tar21_mp",
        AssaultRifle,
        R,
        "TAR-21",
        Middle,
    ),
    gun(
        "iw4l:fal",
        "iw4:weapon/fal_mp",
        AssaultRifle,
        R,
        "FAL",
        Middle,
    ),
    gun(
        "iw4l:m16",
        "iw4:weapon/m16_mp",
        AssaultRifle,
        R,
        "M16A4",
        Middle,
    ),
    gun(
        "iw4l:masada",
        "iw4:weapon/masada_mp",
        AssaultRifle,
        R,
        "ACR",
        Middle,
    ),
    gun(
        "iw4l:fn2000",
        "iw4:weapon/fn2000_mp",
        AssaultRifle,
        R,
        "F2000",
        Middle,
    ),
    gun(
        "iw4l:ak47",
        "iw4:weapon/ak47_mp",
        AssaultRifle,
        R,
        "AK-47",
        Middle,
    ),
    gun(
        "iw4l:t5_galil",
        "t5:weapon/galil_mp",
        AssaultRifle,
        R,
        "Galil",
        Middle,
    ),
    gun(
        "iw4l:t5_commando",
        "t5:weapon/commando_mp",
        AssaultRifle,
        R,
        "Commando",
        Middle,
    ),
    gun(
        "iw4l:iw5_type95",
        "iw5:weapon/iw5_type95_mp",
        AssaultRifle,
        R,
        "Type 95",
        Middle,
    ),
    gun(
        "iw4l:ranger",
        "iw4:weapon/ranger_mp",
        Shotgun,
        S,
        "Ranger",
        Middle,
    ),
    gun(
        "iw4l:model1887",
        "iw4:weapon/model1887_mp",
        Shotgun,
        S,
        "Model 1887",
        Middle,
    ),
    gun(
        "iw4l:striker",
        "iw4:weapon/striker_mp",
        Shotgun,
        S,
        "Striker",
        Middle,
    ),
    gun(
        "iw4l:aa12",
        "iw4:weapon/aa12_mp",
        Shotgun,
        S,
        "AA-12",
        Middle,
    ),
    gun(
        "iw4l:m1014",
        "iw4:weapon/m1014_mp",
        Shotgun,
        S,
        "M1014",
        Middle,
    ),
    gun(
        "iw4l:spas12",
        "iw4:weapon/spas12_mp",
        Shotgun,
        S,
        "SPAS-12",
        Middle,
    ),
    gun(
        "iw4l:t5_spas",
        "t5:weapon/spas_mp",
        Shotgun,
        S,
        "SPAS-12",
        Middle,
    ),
    gun(
        "iw4l:iw5_ksg",
        "iw5:weapon/iw5_ksg_mp",
        Shotgun,
        S,
        "KSG 12",
        Middle,
    ),
    gun(
        "iw4l:riotshield",
        "iw4:weapon/riotshield_mp",
        Special,
        None,
        "Riot Shield",
        Middle,
    ),
    gun(
        "iw4l:t5_crossbow_explosive",
        "t5:weapon/crossbow_explosive_mp",
        Special,
        X,
        "Explosive Crossbow",
        Middle,
    ),
    gun(
        "iw4l:t5_knife_ballistic",
        "t5:weapon/knife_ballistic_mp",
        Special,
        Some(THROWING_KNIFE),
        "Ballistic Knife",
        Middle,
    ),
    gun(
        "iw4l:m21",
        "iw4:weapon/m21_mp",
        Sniper,
        N,
        "M21 EBR",
        Diamond,
    ),
    gun(
        "iw4l:wa2000",
        "iw4:weapon/wa2000_mp",
        Sniper,
        N,
        "WA2000",
        Diamond,
    ),
    gun(
        "iw4l:cheytac",
        "iw4:weapon/cheytac_mp",
        Sniper,
        N,
        "Intervention",
        Diamond,
    ),
    gun(
        "iw4l:barrett",
        "iw4:weapon/barrett_mp",
        Sniper,
        N,
        "Barrett .50cal",
        Late,
    )
    .fire_proof(),
    gun(
        "iw4l:iw5_msr",
        "iw5:weapon/iw5_msr_mp",
        Sniper,
        N,
        "MSR",
        Diamond,
    ),
    gun("iw4l:rpd", "iw4:weapon/rpd_mp", Lmg, L, "RPD", Nether),
    gun("iw4l:sa80", "iw4:weapon/sa80_mp", Lmg, L, "L86 LSW", Nether),
    gun("iw4l:mg4", "iw4:weapon/mg4_mp", Lmg, L, "MG4", Nether),
    gun("iw4l:aug", "iw4:weapon/aug_mp", Lmg, L, "AUG HBAR", Nether),
    gun("iw4l:m240", "iw4:weapon/m240_mp", Lmg, L, "M240", Nether).fire_proof(),
    gun(
        "iw4l:t5_stoner63",
        "t5:weapon/stoner63_mp",
        Lmg,
        L,
        "Stoner63",
        Nether,
    ),
    gun(
        "iw4l:at4",
        "iw4:weapon/at4_mp",
        Launcher,
        K,
        "AT4-HS",
        Nether,
    ),
    gun(
        "iw4l:rpg",
        "iw4:weapon/rpg_mp",
        Launcher,
        K,
        "RPG-7",
        Nether,
    ),
    gun(
        "iw4l:stinger",
        "iw4:weapon/stinger_mp",
        Launcher,
        K,
        "Stinger",
        Late,
    ),
    gun(
        "iw4l:javelin",
        "iw4:weapon/javelin_mp",
        Launcher,
        K,
        "Javelin",
        Late,
    ),
    gun(
        "iw4l:t5_strela",
        "t5:weapon/strela_mp",
        Launcher,
        K,
        "Strela-3",
        Nether,
    ),
    gun(
        "iw4l:t5_china_lake",
        "t5:weapon/china_lake_mp",
        Launcher,
        X,
        "China Lake",
        Nether,
    ),
    gun(
        "iw4l:iw5_xm25",
        "iw5:weapon/xm25_mp",
        Launcher,
        X,
        "XM25",
        Nether,
    ),
    part("iw4l:gun_barrel", "Gun Barrel", "minecraft:iron_bars"),
    part(
        "iw4l:firing_mechanism",
        "Firing Mechanism",
        "minecraft:tripwire_hook",
    ),
    attachment(
        "iw4l:red_dot_sight",
        "Red Dot Sight",
        &["reflex", "elbit"],
        Early,
        Icon::Material("weapon_attachment_reflex"),
    ),
    attachment(
        "iw4l:silencer",
        "Silencer",
        &["silencer", "silencer02", "silencer03"],
        Early,
        Icon::Material("weapon_attachment_suppressor"),
    ),
    attachment(
        "iw4l:fmj",
        "FMJ",
        &["fmj"],
        Early,
        Icon::Material("weapon_attachment_fmj"),
    ),
    attachment(
        "iw4l:extended_mags",
        "Extended Mags",
        &["xmags", "extclip", "dualclip"],
        Early,
        Icon::Material("weapon_attachment_mags"),
    ),
    attachment(
        "iw4l:grip",
        "Grip",
        &["grip"],
        Early,
        Icon::Material("weapon_attachment_grip"),
    ),
    attachment(
        "iw4l:rapid_fire",
        "Rapid Fire",
        &["rof", "rf", "auto"],
        Early,
        Icon::Sprite {
            item: "minecraft:repeater",
            tint: None,
        },
    ),
    attachment(
        "iw4l:tactical_knife",
        "Tactical Knife",
        &["tactical"],
        Early,
        Icon::Material("weapon_attachment_tactical"),
    ),
    attachment(
        "iw4l:holographic_sight",
        "Holographic Sight",
        &["eotech"],
        Middle,
        Icon::Material("weapon_attachment_eotech"),
    ),
    attachment(
        "iw4l:acog_scope",
        "ACOG Scope",
        &["acog", "hamrhybrid", "hybrid"],
        Middle,
        Icon::Material("weapon_attachment_acog"),
    ),
    attachment(
        "iw4l:grenade_launcher",
        "Grenade Launcher",
        &["gl", "gp"],
        Middle,
        Icon::Material("weapon_attachment_m203"),
    ),
    attachment(
        "iw4l:underbarrel_shotgun",
        "Underbarrel Shotgun",
        &["shotgun", "mk"],
        Middle,
        Icon::Material("weapon_attachment_shotgun"),
    ),
    attachment(
        "iw4l:thermal_scope",
        "Thermal Scope",
        &["thermal", "ir"],
        Nether,
        Icon::Material("weapon_attachment_thermal"),
    ),
    attachment(
        "iw4l:heartbeat_sensor",
        "Heartbeat Sensor",
        &["heartbeat"],
        Late,
        Icon::Material("weapon_attachment_heartbeat"),
    ),
    ammo(
        PISTOL_AMMO,
        "Pistol Ammo",
        64,
        Early,
        "minecraft:iron_nugget",
        Some(BRASS),
    ),
    ammo(
        RIFLE_AMMO,
        "Rifle Ammo",
        64,
        Middle,
        "minecraft:iron_nugget",
        Some(COPPER),
    ),
    ammo(
        LMG_AMMO,
        "LMG Ammo",
        64,
        Nether,
        "minecraft:iron_nugget",
        Some(GREY),
    ),
    ammo(
        SHOTGUN_SHELLS,
        "Shotgun Shells",
        64,
        Middle,
        "minecraft:iron_nugget",
        Some(RED),
    ),
    ammo(
        SNIPER_ROUNDS,
        "Sniper Rounds",
        64,
        Diamond,
        "minecraft:iron_nugget",
        Some(GOLD),
    ),
    ammo(
        EXPLOSIVE_ROUND,
        "Explosive Round",
        16,
        Middle,
        "minecraft:fire_charge",
        Some(GREEN),
    ),
    ammo(
        ROCKET,
        "Rocket",
        16,
        Nether,
        "minecraft:firework_rocket",
        None,
    ),
    grenade(
        "iw4l:frag_grenade",
        "iw4:weapon/frag_grenade_mp",
        Kind::Lethal,
        "Frag Grenade",
        Early,
        "weapon_fraggrenade",
    ),
    grenade(
        "iw4l:semtex",
        "iw4:weapon/semtex_mp",
        Kind::Lethal,
        "Semtex",
        Early,
        "cardicon_semtex",
    ),
    grenade(
        THROWING_KNIFE,
        "iw4:weapon/throwingknife_mp",
        Kind::Lethal,
        "Throwing Knife",
        Early,
        "cardicon_throwing_knive",
    ),
    grenade(
        "iw4l:claymore",
        "iw4:weapon/claymore_mp",
        Kind::Lethal,
        "Claymore",
        Early,
        "weapon_claymore",
    ),
    grenade(
        "iw4l:c4",
        "iw4:weapon/c4_mp",
        Kind::Lethal,
        "C4",
        Early,
        "weapon_c4",
    ),
    grenade(
        "iw4l:flash_grenade",
        "iw4:weapon/flash_grenade_mp",
        Kind::Tactical,
        "Flash Grenade",
        Early,
        "weapon_flashbang",
    ),
    grenade(
        "iw4l:concussion_grenade",
        "iw4:weapon/concussion_grenade_mp",
        Kind::Tactical,
        "Stun Grenade",
        Early,
        "weapon_concgrenade",
    ),
    grenade(
        "iw4l:smoke_grenade",
        "iw4:weapon/smoke_grenade_mp",
        Kind::Tactical,
        "Smoke Grenade",
        Early,
        "weapon_smokegrenade",
    ),
    grenade(
        "iw4l:tactical_insertion",
        "iw4:weapon/flare_mp",
        Kind::Tactical,
        "Tactical Insertion",
        Middle,
        "specialty_tactical_insert",
    ),
];

pub fn rows() -> &'static [ForkItem] {
    &ROWS
}

fn index() -> &'static HashMap<&'static str, &'static ForkItem> {
    static INDEX: OnceLock<HashMap<&'static str, &'static ForkItem>> = OnceLock::new();
    INDEX.get_or_init(|| ROWS.iter().map(|row| (row.id, row)).collect())
}

pub fn get(id: &str) -> Option<&'static ForkItem> {
    index().get(id).copied()
}

pub fn fire_proof_ids() -> Vec<String> {
    rows()
        .iter()
        .filter(|row| row.fire_proof)
        .map(|row| row.id.to_owned())
        .collect()
}

pub fn is_fork(id: &str) -> bool {
    get(id).is_some()
}

pub fn by_weapon(weapon: &str) -> Option<&'static ForkItem> {
    ROWS.iter().find(|row| row.weapon == Some(weapon))
}

pub fn by_alias(alias: &str) -> Option<&'static ForkItem> {
    ROWS.iter()
        .find(|row| row.kind == Kind::Attachment && row.aliases.contains(&alias))
}

pub fn ammo_index(id: &str) -> Option<u8> {
    AMMO_TYPES
        .iter()
        .position(|ammo| *ammo == id)
        .and_then(|index| u8::try_from(index).ok())
}

pub trait Catalog {
    fn put_fork_item(&mut self, id: &str, max_stack: u8);
}

impl Catalog for minecraftoss_player::item_catalog::ItemCatalog {
    fn put_fork_item(&mut self, id: &str, max_stack: u8) {
        self.insert(
            id,
            minecraftoss_player::item_catalog::ItemProperties::plain(max_stack),
        );
    }
}

impl Catalog for minecraftoss_core::item::ItemCatalog {
    fn put_fork_item(&mut self, id: &str, max_stack: u8) {
        self.insert_max_stack(id, i32::from(max_stack));
    }
}

pub fn load(catalog: &mut impl Catalog) {
    for row in rows() {
        catalog.put_fork_item(row.id, row.max_stack);
    }
}

pub fn add_names(language: &mut HashMap<String, String>) {
    for row in rows() {
        language
            .entry(row.language_key())
            .or_insert_with(|| row.name.to_owned());
    }
}

pub fn table_hash() -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for row in rows() {
        for byte in format!("{row:?}").bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    for byte in crate::fork_recipes::JSON.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

pub const CUSTOM_DATA: &str = "minecraft:custom_data";
pub const FULL_CLIP: i64 = -1;
pub const CREATIVE_TAB: &str = "minecraft:combat";

pub fn loaded_gun() -> serde_json::Value {
    let state = serde_json::json!({"iw4l": {"clip": FULL_CLIP}});
    serde_json::Value::Object([(CUSTOM_DATA.to_owned(), state)].into_iter().collect())
}

pub fn creative_entries() -> Vec<TabEntry> {
    let mut rows: Vec<&ForkItem> = rows().iter().collect();
    rows.sort_by_key(|row| row.creative_order());
    rows.into_iter()
        .map(|row| TabEntry {
            item: row.id.to_owned(),
            variants: (row.kind == Kind::Gun && row.ammo.is_some()).then_some(Variants::FullClip),
            in_tab: true,
            in_search: true,
        })
        .collect()
}

pub fn add_to_creative(tabs: &mut CreativeTabs) -> bool {
    let Some(tab) = tabs.tabs.iter_mut().find(|tab| tab.id == CREATIVE_TAB) else {
        return false;
    };
    tab.entries.extend(creative_entries());
    true
}

pub fn has_alt_mode(attachments: &[String]) -> bool {
    attachments.iter().any(|name| {
        by_alias(name).is_some_and(|attachment| {
            matches!(
                attachment.id,
                "iw4l:grenade_launcher" | "iw4l:underbarrel_shotgun"
            )
        })
    })
}

pub fn with_full_clip(
    id: &str,
    components: Option<serde_json::Value>,
) -> Option<serde_json::Value> {
    use serde_json::{Map, Value};
    if !get(id).is_some_and(|row| row.kind == Kind::Gun && row.ammo.is_some()) {
        return components;
    }
    let mut components = match components {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    let custom = components
        .entry(CUSTOM_DATA)
        .or_insert_with(|| Value::Object(Map::new()));
    if !custom.is_object() {
        *custom = Value::Object(Map::new());
    }
    let state = custom
        .as_object_mut()
        .map(|custom| {
            custom
                .entry("iw4l")
                .or_insert_with(|| Value::Object(Map::new()))
        })
        .filter(|state| state.is_object());
    let Some(Value::Object(state)) = state else {
        return Some(Value::Object(components));
    };
    let attachments: Vec<String> = state
        .get("attachments")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let alt = has_alt_mode(&attachments);
    let mut fill = vec!["clip"];
    if attachments.iter().any(|name| name == "akimbo") {
        fill.push("clip_left");
    }
    if alt {
        fill.push("alt_clip");
    }
    for key in fill {
        state.entry(key).or_insert_with(|| Value::from(FULL_CLIP));
    }
    Some(Value::Object(components))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn quoted(text: &str) -> Vec<&str> {
        text.split('"').skip(1).step_by(2).collect()
    }

    fn class_picker_weapons() -> Vec<String> {
        let source = include_str!("../../ui/src/classes/setup.rs");
        let start = source.find("fn cac_roster").expect("cac_roster");
        let rest = &source[start..];
        let end = rest.find("ClassEditRow::Perk1").expect("the perk rows");
        quoted(&rest[..end])
            .into_iter()
            .filter(|name| name.ends_with("_mp"))
            .map(|name| format!("iw4:weapon/{name}"))
            .collect()
    }

    fn preset_weapons() -> Vec<String> {
        quoted(include_str!("../../frame/src/class_presets.rs"))
            .into_iter()
            .filter(|name| name.contains(":weapon/"))
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn every_class_picker_and_preset_weapon_has_a_row() {
        let picker = class_picker_weapons();
        let presets = preset_weapons();
        assert_eq!(picker.len(), 51);
        assert!(presets.len() >= 60, "{}", presets.len());
        for weapon in picker.iter().chain(&presets) {
            assert!(by_weapon(weapon).is_some(), "{weapon} has no fork item");
        }
    }

    #[test]
    fn ids_and_weapons_are_unique_and_in_the_fork_namespace() {
        let mut ids = HashSet::new();
        let mut weapons = HashSet::new();
        for row in rows() {
            assert!(row.id.starts_with(NAMESPACE), "{}", row.id);
            assert!(ids.insert(row.id), "{} twice", row.id);
            if let Some(weapon) = row.weapon {
                assert!(weapons.insert(weapon), "{weapon} twice");
            }
            assert_eq!(get(row.id), Some(row));
            assert!(is_fork(row.id));
            assert_eq!(rows()[row.creative_order()].id, row.id);
        }
        assert!(!is_fork("minecraft:carrot_on_a_stick"));
        assert!(!is_fork("iw4:weapon/12"));
        assert!(!is_fork("iw4l:nothing"));
    }

    #[test]
    fn the_roster_has_the_designed_size() {
        let count = |kind| rows().iter().filter(|row| row.kind == kind).count();
        assert_eq!(count(Kind::Gun), 63);
        assert_eq!(count(Kind::Ammo), 7);
        assert_eq!(count(Kind::Attachment), 13);
        assert_eq!(count(Kind::Part), 2);
        assert_eq!(count(Kind::Lethal), 5);
        assert_eq!(count(Kind::Tactical), 4);
    }

    #[test]
    fn stack_sizes_follow_the_design() {
        for row in rows() {
            let expected = match row.kind {
                Kind::Gun => 1,
                _ if row.id == ROCKET || row.id == EXPLOSIVE_ROUND => 16,
                _ => 64,
            };
            assert_eq!(row.max_stack, expected, "{}", row.id);
        }
    }

    #[test]
    fn guns_have_ammo_and_only_netherite_guns_resist_fire() {
        for row in rows() {
            if let Some(ammo) = row.ammo {
                assert!(
                    ammo_index(ammo).is_some() || ammo == THROWING_KNIFE,
                    "{}",
                    row.id
                );
            }
            assert_eq!(
                row.fire_proof,
                row.id == "iw4l:barrett" || row.id == "iw4l:m240",
                "{}",
                row.id
            );
            match row.kind {
                Kind::Gun => assert_eq!(
                    row.ammo.is_none(),
                    row.id == "iw4l:riotshield",
                    "{}",
                    row.id
                ),
                Kind::Lethal | Kind::Tactical => assert!(row.weapon.is_some()),
                _ => assert!(row.weapon.is_none() && row.ammo.is_none()),
            }
        }
        assert_eq!(
            get("iw4l:t5_knife_ballistic").unwrap().ammo,
            Some(THROWING_KNIFE)
        );
        assert_eq!(by_alias("reflex").unwrap().id, "iw4l:red_dot_sight");
        assert_eq!(by_alias("rf").unwrap().id, "iw4l:rapid_fire");
        assert_eq!(ammo_index(ROCKET), Some(6));
    }

    #[test]
    fn the_creative_order_puts_guns_by_class_then_parts_attachments_ammo_and_grenades() {
        let rank = |row: &ForkItem| match (row.kind, row.bucket) {
            (Kind::Gun, Bucket::Pistol) => 0,
            (Kind::Gun, Bucket::MachinePistol) => 1,
            (Kind::Gun, Bucket::Smg) => 2,
            (Kind::Gun, Bucket::AssaultRifle) => 3,
            (Kind::Gun, Bucket::Shotgun) => 4,
            (Kind::Gun, Bucket::Special) => 5,
            (Kind::Gun, Bucket::Sniper) => 6,
            (Kind::Gun, Bucket::Lmg) => 7,
            (Kind::Gun, Bucket::Launcher) => 8,
            (Kind::Part, _) => 9,
            (Kind::Attachment, _) => 10,
            (Kind::Ammo, _) => 11,
            (Kind::Lethal, _) => 12,
            (Kind::Tactical, _) => 13,
            (kind, bucket) => panic!("{kind:?} {bucket:?}"),
        };
        let ranks: Vec<_> = rows().iter().map(rank).collect();
        assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]));
    }

    #[test]
    fn names_fill_the_language_map_without_replacing_existing_keys() {
        let mut language = HashMap::new();
        language.insert("item.iw4l.m4".to_owned(), "M4A1 Carbine".to_owned());
        add_names(&mut language);
        assert_eq!(language["item.iw4l.m4"], "M4A1 Carbine");
        assert_eq!(language["item.iw4l.frag_grenade"], "Frag Grenade");
        assert_eq!(language.len(), rows().len());
        assert_eq!(table_hash(), table_hash());
    }

    #[test]
    fn both_catalogs_take_the_fork_stack_sizes() {
        let mut core = minecraftoss_core::item::ItemCatalog::default();
        load(&mut core);
        assert_eq!(core.max_stack("iw4l:m4"), 1);
        assert_eq!(core.max_stack("iw4l:rocket"), 16);
        assert_eq!(core.max_stack("iw4l:frag_grenade"), 64);
        assert!(!core.is_fuel("iw4l:t5_ak74u"));
        let mut player = minecraftoss_player::item_catalog::ItemCatalog::default();
        load(&mut player);
        assert_eq!(player.max_stack("iw4l:barrett"), 1);
        assert_eq!(player.max_stack("iw4l:explosive_round"), 16);
        assert_eq!(player.max_stack("iw4l:fmj"), 64);
        assert_eq!(player.max_stack("minecraft:stone"), 64);
    }

    #[test]
    fn the_combat_tab_ends_with_every_fork_item_in_table_order_and_guns_come_loaded() {
        let mut tabs = crate::creative_tabs::fallback_tabs(Vec::new(), "test");
        let combat = tabs
            .tabs
            .iter_mut()
            .find(|tab| tab.id == CREATIVE_TAB)
            .unwrap();
        combat.entries.push(TabEntry {
            item: "minecraft:bow".to_owned(),
            variants: None,
            in_tab: true,
            in_search: true,
        });
        assert!(add_to_creative(&mut tabs));
        let combat = tabs.tab(CREATIVE_TAB).unwrap();
        let ids = combat.item_ids();
        let table: Vec<&str> = rows().iter().map(|row| row.id).collect();
        assert_eq!(ids[0], "minecraft:bow");
        assert_eq!(ids[1..], table[..]);
        for entry in &combat.entries[1..] {
            let row = get(&entry.item).unwrap();
            let loaded = row.kind == Kind::Gun && row.ammo.is_some();
            assert_eq!(entry.variants.is_some(), loaded, "{}", row.id);
            assert!(entry.in_tab && entry.in_search);
        }
        assert_eq!(combat.entries[1].variants, Some(Variants::FullClip));
        let shield = combat
            .entries
            .iter()
            .find(|entry| entry.item == "iw4l:riotshield")
            .unwrap();
        assert_eq!(shield.variants, None);
        let data = crate::creative_variants::VariantData::default();
        let expanded = data.expand(&Variants::FullClip).unwrap();
        for row in rows()
            .iter()
            .filter(|row| row.kind == Kind::Gun && row.ammo.is_some())
        {
            assert_eq!(
                Some(&expanded[0]),
                with_full_clip(row.id, None).as_ref(),
                "{}",
                row.id
            );
        }
        let search = tabs.search(false);
        assert_eq!(
            search.iter().filter(|entry| is_fork(&entry.item)).count(),
            rows().len()
        );
        assert!(!add_to_creative(&mut CreativeTabs::default()));
    }

    #[test]
    fn the_real_combat_tab_keeps_vanilla_first_and_ends_with_the_fork_items() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = std::path::Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let mut tabs = crate::creative_tabs::read_creative_tabs(&jar).unwrap();
        let vanilla = tabs.tab(CREATIVE_TAB).unwrap().entries.len();
        assert!(vanilla > 0);
        assert!(add_to_creative(&mut tabs));
        let combat = tabs.tab(CREATIVE_TAB).unwrap();
        assert!(
            combat.entries[..vanilla]
                .iter()
                .all(|entry| entry.item.starts_with("minecraft:"))
        );
        let added: Vec<&str> = combat.entries[vanilla..]
            .iter()
            .map(|entry| entry.item.as_str())
            .collect();
        let table: Vec<&str> = rows().iter().map(|row| row.id).collect();
        assert_eq!(added, table);
        assert!(
            tabs.tabs
                .iter()
                .filter(|tab| tab.id != CREATIVE_TAB)
                .flat_map(|tab| &tab.entries)
                .all(|entry| !is_fork(&entry.item))
        );
    }

    #[test]
    fn given_guns_get_full_clips_and_other_items_stay_plain() {
        use serde_json::json;
        assert_eq!(
            with_full_clip("iw4l:m4", None),
            Some(json!({"minecraft:custom_data": {"iw4l": {"clip": -1}}}))
        );
        let named = json!({
            "minecraft:custom_name": "Old Faithful",
            "minecraft:custom_data": {"other": 1, "iw4l": {"attachments": ["akimbo", "gl"], "clip": 7}}
        });
        assert_eq!(
            with_full_clip("iw4l:m4", Some(named)),
            Some(json!({
                "minecraft:custom_name": "Old Faithful",
                "minecraft:custom_data": {"other": 1, "iw4l": {
                    "attachments": ["akimbo", "gl"], "clip": 7, "clip_left": -1, "alt_clip": -1
                }}
            }))
        );
        assert_eq!(with_full_clip("iw4l:riotshield", None), None);
        assert_eq!(with_full_clip("iw4l:rifle_ammo", None), None);
        assert_eq!(with_full_clip("minecraft:bow", None), None);
        let kept = Some(json!({"minecraft:custom_name": "x"}));
        assert_eq!(with_full_clip("iw4l:frag_grenade", kept.clone()), kept);
    }

    #[test]
    fn the_start_kit_gives_loaded_guns_with_their_attachments() {
        let mut catalog = minecraftoss_player::item_catalog::ItemCatalog::default();
        load(&mut catalog);
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.recipes = std::sync::Arc::new(
            minecraftoss_player::crafting::RecipeBook::default()
                .with_item_catalog(std::sync::Arc::new(catalog)),
        );
        let given = crate::crafting::give_kit(
            &mut inventory,
            r#"iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"]}}]*2,iw4l:rocket*20"#,
        );
        assert_eq!(given, 4);
        let stacks: Vec<_> = inventory.slots.iter().flatten().collect();
        assert_eq!(stacks[0].id, "iw4l:m4");
        assert_eq!(stacks[0].count, 1);
        assert_eq!(
            stacks[0].components,
            Some(
                serde_json::json!({"minecraft:custom_data": {"iw4l": {"attachments": ["reflex"], "clip": -1}}})
            )
        );
        assert_eq!(stacks[1].components, stacks[0].components);
        assert_eq!(
            (stacks[2].id.as_str(), stacks[2].count, stacks[3].count),
            ("iw4l:rocket", 16, 4)
        );
    }

    #[test]
    fn two_identical_guns_on_the_ground_do_not_merge() {
        use minecraftoss_core::Registries;
        use minecraftoss_core::registries::DataPaths;
        use minecraftoss_generator::terrain::TerrainGenerator;
        use minecraftoss_world::chunk_map::WorldGen;
        use minecraftoss_world::level::Level;
        use minecraftoss_world::level::container::Stack;
        use minecraftoss_world::level::entity::EntityKind;
        let Ok(paths) = DataPaths::discover() else {
            return;
        };
        let Ok(mut registries) = Registries::load(&paths) else {
            return;
        };
        load(&mut registries.items);
        let registries = std::sync::Arc::new(registries);
        let Ok(terrain) = TerrainGenerator::overworld(registries.clone(), 0) else {
            return;
        };
        let Ok(gen) = WorldGen::for_dimension(std::sync::Arc::new(terrain), "minecraft:overworld")
        else {
            return;
        };
        let on_ground = |id: &str| {
            let mut level = Level::new(&gen.library, -64, 384);
            for _ in 0..2 {
                level.spawn_item_with([0.5, 200.0, 0.5], Stack::new(id, 1), [0.0; 3], 0, 0);
            }
            for _ in 0..41 {
                level.tick();
            }
            level
                .entities
                .iter()
                .filter(|e| {
                    !e.removed && matches!(&e.kind, EntityKind::Item(data) if data.stack.id == id)
                })
                .count()
        };
        assert_eq!(on_ground("minecraft:stick"), 1, "two sticks merge");
        assert_eq!(on_ground("iw4l:m4"), 2, "two guns stay apart");
        assert_eq!(on_ground("iw4l:frag_grenade"), 1, "grenades stack");
    }
}
