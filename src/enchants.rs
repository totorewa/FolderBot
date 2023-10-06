use std::ops::RangeInclusive;

use rand::{rngs::ThreadRng, seq::SliceRandom, Rng, thread_rng};

pub struct Enchant {
    pub name: &'static str,
    pub short: &'static str,
    weight: u16,
    costs: &'static [RangeInclusive<u32>],
}

#[rustfmt::skip]
impl Enchant {
    pub const AQUA_AFFINITY: Self = Self::new("Aqua Affinity", "Aqua Aff.", 2, &[1..=41]);
    pub const BANE_OF_ARTHROPODS: Self = Self::new("Bane of Arthropods", "Bane Art.", 5, &[5..=25, 13..=33, 21..=41, 29..=49, 37..=57]);
    pub const BLAST_PROTECTION: Self = Self::new("Blast Protection", "Blast Pro.", 2, &[5..=13, 13..=21, 21..=29, 29..=37]);
    pub const CHANNELING: Self = Self::new("Channeling", "Chann", 1, &[25..=50]);
    pub const DEPTH_STRIDER: Self = Self::new("Depth Strider", "Depth", 2, &[10..=25, 20..=35, 30..=45]);
    pub const EFFICIENCY: Self = Self::new("Efficiency", "Eff", 10, &[1..=51, 11..=61, 21..=71, 31..=81, 41..=91]);
    pub const FEATHER_FALLING: Self = Self::new("Feather Falling", "Feather", 5, &[5..=11, 11..=17, 17..=23, 23..=29]);
    pub const FIRE_ASPECT: Self = Self::new("Fire Aspect", "Fire Asp.", 2, &[10..=60, 30..=80]);
    pub const FIRE_PROTECTION: Self = Self::new("Fire Protection", "Fire Pro.", 5, &[10..=18, 18..=26, 26..=34, 34..=42]);
    pub const FLAME: Self = Self::new("Flame", "Flame", 2, &[20..=50]);
    pub const FORTUNE: Self = Self::new("Fortune", "Fort", 2, &[15..=65, 24..=74, 33..=83]);
    pub const IMPALING: Self = Self::new("Impaling", "Impale", 2, &[1..=21, 9..=29, 17..=37, 25..=45, 33..=53]);
    pub const INFINITY: Self = Self::new("Infinity", "Inf", 1, &[20..=30]);
    pub const KNOCKBACK: Self = Self::new("Knockback", "Knock", 5, &[5..=55, 25..=75]);
    pub const LOOTING: Self = Self::new("Looting", "Loot", 2, &[15..=65, 24..=74, 33..=83]);
    pub const LOYALTY: Self = Self::new("Loyalty", "Loyal", 5, &[12..=50, 19..=50, 26..=50]); // static max
    pub const LUCK_OF_THE_SEA: Self = Self::new("Luck of the Sea", "Luck Sea",  2, &[15..=65, 24..=74, 33..=83]);
    pub const LURE: Self = Self::new("Lure",  "Lure", 2, &[15..=65, 24..=74, 33..=83]);
    pub const MULTISHOT: Self = Self::new("Multishot", "Multishot", 2, &[20..=50]);
    pub const PIERCING: Self = Self::new("Piercing", "Pierce", 10, &[1..=50, 11..=50, 21..=50, 31..=50, 41..=50]); // static max
    pub const POWER: Self = Self::new("Power", "Power", 10, &[1..=16, 11..=26, 21..=36, 31..=46, 41..=56]);
    pub const PROJECTILE_PROTECTION: Self = Self::new("Projectile Protection", "Proj. Pro.", 5, &[3..=9, 9..=15, 15..=21, 21..=27]);
    pub const PROTECTION: Self = Self::new("Protection", "Protec", 10, &[1..=12, 12..=23, 23..=34, 45..=56]);
    pub const PUNCH: Self = Self::new("Punch", "Punch", 2, &[12..=37, 32..=57]);
    pub const QUICK_CHARGE: Self = Self::new("Quick Charge", "Qu. Charge", 5, &[12..=50, 32..=50, 52..=50]); // static max
    pub const RESPIRATION: Self = Self::new("Respiration", "Resp", 2, &[10..=40, 20..=50, 30..=60]);
    pub const RIPTIDE: Self = Self::new("Riptide", "Riptide", 2, &[17..=50, 24..=50, 31..=50]); // static max
    pub const SHARPNESS: Self = Self::new("Sharpness", "Sharp", 10, &[1..=21, 12..=32, 23..=43, 34..=54, 45..=65]);
    pub const SILK_TOUCH: Self = Self::new("Silk Touch", "Silk", 1, &[15..=65]);
    pub const SMITE: Self = Self::new("Smite", "Smite", 5, &[5..=25, 13..=33, 21..=41, 29..=49, 37..=57]);
    pub const SWEEPING_EDGE: Self = Self::new("Sweeping Edge", "Sweep", 2, &[5..=20, 14..=29, 23..=38]);
    pub const THORNS: Self = Self::new("Thorns", "Thorns", 1, &[10..=60, 30..=70, 50..=80]);
    pub const UNBREAKING: Self = Self::new("Unbreaking", "Unbreak", 5, &[5..=55, 13..=63, 21..=71]);

    const fn new(name: &'static str, short: &'static str, weight: u16, costs: &'static [RangeInclusive<u32>]) -> Self {
        Self { name, short, weight, costs }
    }
}

#[derive(Copy, Clone)]
pub struct EnchantOffer {
    pub enchant: &'static Enchant,
    pub level: u8,
    pub row: u8,
    pub cost: u8,
    pub bookshelves: u32,
    pub special_response: bool,
}

pub fn roll_enchant() -> Option<EnchantOffer> {
    const ENCHANTS: &[&Enchant] = &[
        &Enchant::AQUA_AFFINITY,
        &Enchant::BANE_OF_ARTHROPODS,
        &Enchant::BLAST_PROTECTION,
        &Enchant::CHANNELING,
        &Enchant::DEPTH_STRIDER,
        &Enchant::EFFICIENCY,
        &Enchant::FEATHER_FALLING,
        &Enchant::FIRE_ASPECT,
        &Enchant::FIRE_PROTECTION,
        &Enchant::FLAME,
        &Enchant::FORTUNE,
        &Enchant::IMPALING,
        &Enchant::INFINITY,
        &Enchant::KNOCKBACK,
        &Enchant::LOOTING,
        &Enchant::LOYALTY,
        &Enchant::LUCK_OF_THE_SEA,
        &Enchant::LURE,
        &Enchant::MULTISHOT,
        &Enchant::PIERCING,
        &Enchant::POWER,
        &Enchant::PROJECTILE_PROTECTION,
        &Enchant::PROTECTION,
        &Enchant::PUNCH,
        &Enchant::QUICK_CHARGE,
        &Enchant::RESPIRATION,
        &Enchant::RIPTIDE,
        &Enchant::SHARPNESS,
        &Enchant::SILK_TOUCH,
        &Enchant::SMITE,
        &Enchant::SWEEPING_EDGE,
        &Enchant::THORNS,
        &Enchant::UNBREAKING,
    ];

    const BOOK_ENCHANTMENT_VALUE: u32 = 1;

    let mut rng = thread_rng();
    let (row, bookshelves) = random_enchantment_setup(&mut rng);
    let enchantability: u32 = BOOK_ENCHANTMENT_VALUE + random_cost(&mut rng, row, bookshelves);

    let mut offers: Vec<(&Enchant, u8)> = Vec::new();
    for (i, enc) in ENCHANTS.iter().enumerate() {
        for level in (0..enc.costs.len()).rev() {
            let costs = &enc.costs[level];
            if enchantability < *costs.start() || enchantability > *costs.end() {
                continue;
            }
            offers.push((ENCHANTS[i], level as u8 + 1));
            break;
        }
    }

    let special_response = rng.gen_bool(0.2);

    offers
        .choose_weighted(&mut rng, |o| o.0.weight)
        .ok()
        .map(|o| {
            let (enchant, level) = *o;
            EnchantOffer {
                enchant,
                level,
                row,
                cost: (enchantability - BOOK_ENCHANTMENT_VALUE) as u8,
                bookshelves,
                special_response,
            }
        })
}

fn random_cost(rng: &mut ThreadRng, row: u8, bookshelves: u32) -> u32 {
    let mut rnd = 1 + (bookshelves >> 1) + bookshelves;
    rnd = rng.gen_range(rnd..(rnd + 8));
    match row {
        1 => 1.max(rnd / 3),
        2 => rnd * 2 / 3 + 1,
        _ => rnd.max(bookshelves * 2),
    }
}

/// Get random enchantment table row and random number of bookshelves
///
/// About 1/25 chance of 15 bookshelves, I think..
///
/// Twice the chance of getting first or second row than the third row.
fn random_enchantment_setup(rng: &mut ThreadRng) -> (u8, u32) {
    let upperbounds = rng.gen_range(0..32).min(15);
    (
        match rng.gen_range(0..5) {
            ch if ch == 4 => 3,
            ch if ch > 1 => 2,
            _ => 1,
        },
        rng.gen_range(0..=upperbounds),
    )
}
