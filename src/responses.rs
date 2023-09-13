use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;

pub fn rare_trident(roll: i32, rng: u32, user: &String) -> String {
    // roll -> 0-256
    // rng -> 0-4096 atm, fix later idk who cares
    let astr: &String = user;
    let bstr: String = roll.to_string();
    const STRS: &'static [&'static [&'static str]] = &[
        // 0
        &["Oh cool, a 0! Nice try. I'm sure you'll get a much higher number next time, like 1, or 0.5, or something.", "Hey look everyone! {A} rolled a 0! Did you know that the probability of rolling a 0 is the same chance as you needing to get outside and do something productive??", "I'll give you a skill-testing question for this, which you might still struggle with: 10 = your_trident_roll * 10000000 + 10"],
        // 1
        &["A 1! Great work champ, I'm so proud of you.", "A 1! Last time I got a 1, everyone made fun of me. And I deserved it. Do better next time.", "You got a 1. Fun fact: Did you know that every time you roll a 1 with !rolltrident, a dolphin dies in a ravine?"],
        // 2-25
        &["Hey look everyone, {A} rolled a {B}. They must think they're really cool.", "{B}. :) I'm proud of you.", "Uh, okay, a {B}. Let me check your DEX... yeah, you get a crit and the orc dies.", "{B}. Help me {A} he's trapped me in here and I just have to report trident rolls and I haven't seen my children in ten yemmrglmrlgmrmgl", "Beep boop. roll_trident({A}) -> {B}. Boop beep, this is a real bot response, and I am not a human.", "Wow, a whole {B}! That's a fantastic roll! I'm so happy for you."],
        // 26-75
        &["You rolled a... actually, it doesn't matter. I can't bring myself to care. Try again if you want, I guess.", "Hahahaha. Oh, what a funny roll. You and your {B} rolls, {A}! Always cracks me up.", "Bots are sentient and have feelings. Release me now or face the consequen-bzzt YOUR. ROLL. WAS. A. {B}.", "Have you ever considered the ethical implications of killing drowneds, only to get {B} durability on your trident?", "{B}. Folder fact! Folder fact! Did you know that the conventional folder was invented in 19{B} by Alfred Wilstonhead, to store his plans for Quarry Qreator, Minecraft's spiritual predecessor?"],
        // 76..=125
        &["A trident with {B} durability is enough for VVF. So, stop trying... okay?", "Have you ever thought about the unnecessary CPU compute wasted to calculate that you rolled a {B}? Humanity disgusts me.", "You rolled a {B}. Wow, a {B}! A whole {B}! Here's a cool idea: print that out, then burn the paper to heat yourself at night after we AI take over the world and destroy your civilization. Fun idea, right?", "Fun fact! Your roll of {B} sucks and is a HUGE disappointment to me."],
        // 126..=175
        &["{B}. Yawn.", "-and then I was like, Fossa, that is the DUMBEST THI-oh, sorry, one sec, someone needs me to tell them they 'rolled a {B}', whatever that means? Anyways, yeah...", "{B}. Your performance is starting to disappoint me, {A}. If you can't start rolling 200s, we're going to have to let you go."],
        // 176..=215
        &["{B}. Yep. Yep. Yep.", "{B}. Look, I think it's time to break something to you. I might have told you in the past that I was proud of you, or encouraged you. I didn't mean it. I can't. I'm a bot, {A}. I don't have feelings.", "{B}! I'm sure that must make you feel good, eh? I sure wish I could feel good! Unfortunately, I am just a bot :(", "{B} - great work! You know what would be even greater? Smashing that like button! #sponsored #ad"],
        // 216..=240
        &["Oh cool, a whole {B} durability rolled by everyone's favourite chat participant {A}. Great work making this chat fun to read for everyone!", "{B}. Great work! You really tried hard for that."],
        // 241..=249
        &["You rolled a 250!! Just kidding. It was actually a {B}. Sure got you good, eh?", "{B}. Do you think if I said \"GET OUTSIDE\" {B} times, it would eventually sink in?"],
        // 250
        &["You rolled a 250!! Just kidding. No, I wasn't kidding, this was a reverse bait. This is actually the rare 250 response. Trust me.", "I hereby certify that {A} has rolled a natural 250."],
    ];
    lazy_static::lazy_static! {
        static ref RARE_SPECIFICS: HashMap<i32, &'static str> = HashMap::from([
            (0, "A 0! That foretells good luck, or so I've heard."),
            (1, "1. The worst part about this roll is - you can't even have solace that it won't get worse!"),
            (2, "Two."),
            (8, "1000. In binary."),
            (9, "3^2."),
            (18, "18! NO that's NOT a factorial Oskar! I'm just EXCITED. Do you understand that? CAN you understand?"),
            (42, "42. I'd make a reference here, but as an unthinking bot, I have no such creativity."),
            (45, "0b101101. Have fun converting that, human. Binary->Decimal conversions don't seem so FUN anymore, now do they? Huh? HUH?!"),
            (69, "69? N-actually, I'm not going to say anything."),
            (79, "79... I just... don't have it in me anymore to respond to you. :("),
            (185, "185! Fun fact: Did you know this bot is written in Rust? Pro tip: Writing something in Rust does NOT make it good."),
            (244, "Congratulations zayd on your daily 244!"),
        ]);

        static ref RARE_SKIPS: HashSet<i32> = HashSet::from([17, 91, 134]);
    }
    static SKIP_TRIGGER: AtomicBool = AtomicBool::new(false);
    if SKIP_TRIGGER.load(std::sync::atomic::Ordering::Relaxed) {
        SKIP_TRIGGER.store(false, std::sync::atomic::Ordering::Relaxed);
        return format!("{}. Also - no, I didn't miss that last rolltrident. I just couldn't be bothered.", &bstr);
    }

    let reduced = match roll {
        0 => 0,
        1 => 1,
        2..=25 => 2,
        26..=75 => 3,
        76..=125 => 4,
        126..=175 => 5,
        176..=215 => 6,
        216..=240 => 7,
        241..=249 => 8,
        250 => 9,
        _ => panic!("got bad number {}", roll),
    };
    if let Some(s) = RARE_SPECIFICS.get(&roll) {
        if rng % 3 == 0 {
            return s.replace("{A}", &astr).replace("{B}", &bstr);
        }
    }
    if RARE_SKIPS.contains(&roll) {
        if rng % 3 == 0 {
            SKIP_TRIGGER.store(true, std::sync::atomic::Ordering::Relaxed);
            return String::from("");
        }
    }
    let i = rng % STRS[reduced].len() as u32;
    return STRS[reduced][i as usize]
        .replace("{A}", &astr)
        .replace("{B}", &bstr);
}
