use std::cmp::max;
use rand::{rngs::ThreadRng, Rng};

pub struct Enchant {
    name: &'static str,
    weight: i32,
    min_costs: &'static [i32],
    cost_span: i32,
}

impl Enchant {
    pub const AQUA_AFFINITY: Self = Self { name: "Aqua Affinity", weight: 2, min_costs: &[1], cost_span: 40 };
    pub const BANE_OF_ARTHROPODS: Self = Self { name: "Bane of Arthropods", weight: 5, min_costs: &[5, 13, 21, 29, 37], cost_span: 20 };
    pub const BLAST_PROTECTION: Self = Self { name: "Blast Protection", weight: 2, min_costs: &[5, 13, 21, 29], cost_span: 8 };
    pub const CHANNELING: Self = Self { name: "Channeling", weight: 1, min_costs: &[25], cost_span: 25 };
    pub const DEPTH_STRIDER: Self = Self { name: "Depth Strider", weight: 2, min_costs: &[10, 20, 30], cost_span: 15 };
    pub const EFFICIENCY: Self = Self { name: "Efficiency", weight: 10, min_costs: &[1, 11, 21, 31, 41], cost_span: 50 };
    pub const FEATHER_FALLING: Self = Self { name: "Feather Falling", weight: 5, min_costs: &[5, 11, 17, 23], cost_span: 6 };
    pub const FIRE_ASPECT: Self = Self { name: "Fire Aspect", weight: 2, min_costs: &[10, 30], cost_span: 50 };
    pub const FIRE_PROTECTION: Self = Self { name: "Fire Protection", weight: 5, min_costs: &[10, 18, 26, 34], cost_span: 8 };
    pub const FLAME: Self = Self { name: "Flame", weight: 2, min_costs: &[20], cost_span: 30 };
    pub const FORTUNE: Self = Self { name: "Fortune", weight: 2, min_costs: &[15, 24, 33], cost_span: 50 };
    pub const IMPALING: Self = Self { name: "Impaling", weight: 2, min_costs: &[1, 9, 17, 25, 33], cost_span: 20 };
    pub const INFINITY: Self = Self { name: "Infinity", weight: 1, min_costs: &[20], cost_span: 30 };
    pub const KNOCKBACK: Self = Self { name: "Knockback", weight: 5, min_costs: &[5, 25], cost_span: 50 };
    pub const LOOTING: Self = Self { name: "Looting", weight: 2, min_costs: &[15, 24, 33], cost_span: 50 };
    pub const LOYALTY: Self = Self { name: "Loyalty", weight: 5, min_costs: &[12, 19, 26], cost_span: 50 }; // i crie
    pub const LUCK_OF_THE_SEA: Self = Self { name: "Luck of the Sea",  weight: 2, min_costs: &[15, 24, 33], cost_span: 50 };
    pub const LURE: Self = Self { name: "Lure", weight: 2, min_costs: &[15, 24, 33], cost_span: 50 };
    pub const MULTISHOT: Self = Self { name: "Multishot", weight: 2, min_costs: &[20], cost_span: 30 };
    pub const PIERCING: Self = Self { name: "Piercing", weight: 10, min_costs: &[1, 11, 21, 31, 41], cost_span: 50 }; // i crie sum moor
    pub const POWER: Self = Self { name: "Power", weight: 10, min_costs: &[1, 11, 21, 31, 41], cost_span: 15 };
    pub const PROJECTILE_PROTECTION: Self = Self { name: "Projectile Protection", weight: 5, min_costs: &[3, 9, 15, 21], cost_span: 6 };
    pub const PROTECTION: Self = Self { name: "Protection", weight: 10, min_costs: &[1, 12, 23, 45], cost_span: 11 };
    pub const PUNCH: Self = Self { name: "Punch", weight: 2, min_costs: &[12, 32], cost_span: 25 };
    pub const QUICK_CHARGE: Self = Self { name: "Quick Charge", weight: 5, min_costs: &[12, 32, 52], cost_span: 50 }; // stap i cen crie any moor
    pub const RESPIRATION: Self = Self { name: "Respiration", weight: 2, min_costs: &[10, 20, 30], cost_span: 30 };
    pub const RIPTIDE: Self = Self { name: "Riptide", weight: 2, min_costs: &[17, 24, 31], cost_span: 50 }; // i ded
    pub const SHARPNESS: Self = Self { name: "Sharpness", weight: 10, min_costs: &[1, 12, 23, 34, 45], cost_span: 20 };
    pub const SILK_TOUCH: Self = Self { name: "Silk Touch", weight: 1, min_costs: &[15], cost_span: 50 };
    pub const SMITE: Self = Self { name: "Smite", weight: 5, min_costs: &[5, 13, 21, 29, 37], cost_span: 20 };
    pub const SWEEPING_EDGE: Self = Self { name: "Sweeping Edge", weight: 2, min_costs: &[5, 14, 23], cost_span: 15 };
    pub const THORNS: Self = Self { name: "Thorns", weight: 1, min_costs: &[10, 30, 50], cost_span: 50 };
    pub const UNBREAKING: Self = Self { name: "Unbreaking", weight: 5, min_costs: &[5, 13, 21], cost_span: 50 };
}

struct EnchantOffer<'a> {
    enchant: &'a &'static Enchant,
    level: i32,
}

pub fn roll_enchant(rng: &mut ThreadRng, mut row: i32) -> String {
    const ENCHANTS: &[&Enchant] = &[
        &Enchant::AQUA_AFFINITY, &Enchant::BANE_OF_ARTHROPODS, &Enchant::BLAST_PROTECTION, &Enchant::CHANNELING,
        &Enchant::DEPTH_STRIDER, &Enchant::EFFICIENCY, &Enchant::FEATHER_FALLING, &Enchant::FIRE_ASPECT,
        &Enchant::FIRE_PROTECTION, &Enchant::FLAME, &Enchant::FORTUNE, &Enchant::IMPALING, &Enchant::INFINITY, 
        &Enchant::KNOCKBACK, &Enchant::LOOTING, &Enchant::LOYALTY, &Enchant::LUCK_OF_THE_SEA, &Enchant::LURE, 
        &Enchant::MULTISHOT, &Enchant::PIERCING, &Enchant::POWER, &Enchant::PROJECTILE_PROTECTION, 
        &Enchant::PROTECTION, &Enchant::PUNCH, &Enchant::QUICK_CHARGE, &Enchant::RESPIRATION, &Enchant::RIPTIDE, 
        &Enchant::SHARPNESS, &Enchant::SILK_TOUCH, &Enchant::SMITE, &Enchant::SWEEPING_EDGE, &Enchant::THORNS, 
        &Enchant::UNBREAKING,
    ];
    const ROMAN_MAP: &[&str] = &[
        "I", "II", "III", "IV", "V"
    ];

    row -= 1;

    let mut enchantability: i32 = enchant_cost(rng.gen_range(8..=30), row) + rng.gen_range(1..=3);
    enchantability = max((enchantability as f64 + enchantability as f64 * rng.gen_range(-0.15f64..=0.15f64)).round() as i32, 1);
    
    let mut offers: Vec<EnchantOffer> = Vec::new();
    let mut total_weight: i32 = 0;
    for enc in ENCHANTS {
        for i in (0..enc.min_costs.len()).rev() {
            let min_cost = enc.min_costs[i];
            if enchantability >= min_cost || enchantability <= min_cost + enc.cost_span { // some enchants have a fixed max cost - this does not account for those cases (see i crie comments)
                offers.push(EnchantOffer { enchant: enc, level: i as i32 + 1 });
                total_weight += enc.weight;
                break;
            }
        }
    }

    match weighted_random(rng, &offers, total_weight) {
        Some(i) => {
            let offer = &offers[i];
            format!("{} {}", offer.enchant.name, ROMAN_MAP[(offer.level - 1) as usize])
        }
        _ => "".to_string()
    }
}

fn enchant_cost(n: i32, row: i32) -> i32 {
    if row == 0 {
        return max(n / 3, 1);
    }
    if row == 1 {
        return n * 2 / 3 + 1;
    }
    return max(n, 30);
}

fn weighted_random(rng: &mut ThreadRng, offers: &Vec<EnchantOffer>, total_weight: i32) -> Option<usize> {
    let mut offset = rng.gen_range(0..=total_weight);
    for (i, offer) in offers.iter().enumerate() {
        offset -= offer.enchant.weight;
        if offset < 0 {
            return Some(i)
        }
    }
    None
}
