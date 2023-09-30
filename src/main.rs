use async_std::{
    // TODO use async_channel instead of unstable+slower
    channel::{Receiver, Sender},
    io::BufReader,
    net::TcpStream,
    prelude::*,
    task,
};
use async_trait::async_trait;
use futures::{select, FutureExt};
use lazy_static::lazy_static;
use rand::{thread_rng, Rng};
use regex::Regex;
use serde_with::serde_as;
use serde_with::DefaultOnError;
use std::path::Path;
use std::time::Duration;
use std::time::SystemTime;
use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
};
use std::{io::Result, sync::Mutex};

use reqwest::{header, Client};
use rspotify::model::{AdditionalType, PlayableItem};
use rspotify::prelude::*;

use folderbot::audio::Audio;
use folderbot::command_tree::{CmdValue, CommandNode, CommandTree};
use folderbot::db::player::{Player, PlayerData, PlayerScratch};
use folderbot::enchants::roll_enchant;
use folderbot::game::Game;
use folderbot::responses::rare_trident;
use folderbot::spotify::SpotifyChecker;
use folderbot::trident::random_response;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct Artist {
    name: String,
}
#[derive(Serialize, Deserialize, Debug)]
struct Track {
    name: String,
    artists: Vec<Artist>,
}
#[derive(Serialize, Deserialize, Debug)]
struct APIResponse {
    item: Track,
}

#[derive(Serialize, Deserialize, Debug)]
struct MojangAPIResponse {
    name: String,
    id: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRRecord {
    #[serde_as(deserialize_as = "DefaultOnError")]
    win: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    lose: f64,
    #[serde_as(deserialize_as = "DefaultOnError")]
    draw: f64,
}
// USEFUL SERDE DOCS
// https://serde.rs/enum-representations.html
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
struct MCSRData {
    uuid: String,
    nickname: String,
    //#[serde_as(deserialize_as = "DefaultOnError")]
    //badge: i64,
    elo_rate: i64,
    elo_rank: i64,
    created_time: i64,
    latest_time: i64,
    total_played: i64,
    season_played: i64,
    highest_winstreak: i64,
    current_winstreak: i64,
    prev_elo_rate: i64,
    best_elo_rate: i64,
    best_record_time: i64,
    records: HashMap<String, MCSRRecord>,
}
#[derive(Serialize, Deserialize, Debug)]
struct MCSRAPIResponse {
    status: String,
    data: MCSRData,
}

impl MCSRData {
    fn win_loss(&self) -> String {
        match self.records.get("2") {
            /*
            Some(MR) => match (MR.win.parse::<f64>(), MR.lose.parse::<f64>()) {
                (Ok(w), Ok(l)) => {
                    format!("[{} - {} ({:.2}%)]", MR.win, MR.lose, w * 100.0 / (l + w))
                }
                _ => format!("[{} - {}]", MR.win, MR.lose),
            },
            */
            Some(mcsr_record) => format!(
                "[{} - {} ({:.2}%)]",
                mcsr_record.win,
                mcsr_record.lose,
                mcsr_record.win * 100.0 / (mcsr_record.lose + mcsr_record.win)
            ),
            None => "[No data]".to_string(),
        }
    }
}

// Temporary until I find the correct way to do this.
trait CaptureExt {
    fn str_at(&self, i: usize) -> String;
}

impl CaptureExt for regex::Captures<'_> {
    fn str_at(&self, i: usize) -> String {
        self.get(i).unwrap().as_str().to_string()
    }
}

/*
// Message filtering
enum FilterResult {
    Skip,
    Ban(String),
    Empty,
}

fn filter(_name: &String, message: &String) -> FilterResult {
    lazy_static! {
        static ref SPAM_RE_1: Regex =
            Regex::new(r"follower.{0,15}prime.{0,15}view.{0,25}bigfollows.{0,10}com").unwrap();
    }

    match SPAM_RE_1.captures(message.as_str()) {
        // TODO Use re search or something, this is offline code
        Some(_) => FilterResult::Ban(String::from("Your message has been marked as spam. To be unbanned, send a private message to DesktopFolder.")),
        _ => FilterResult::Empty,
    }
}
*/

enum Command {
    Stop,
    Continue,
}

struct IRCMessage(String);

#[async_trait]
trait IRCStream {
    async fn send(&mut self, text: IRCMessage) -> ();
}

#[async_trait]
impl IRCStream for TcpStream {
    async fn send(&mut self, text: IRCMessage) {
        println!("Sending: '{}'", text.0.trim());
        let _ = self.write(text.0.as_bytes()).await;
    }
}

struct TwitchFmt {}

impl TwitchFmt {
    fn pass(pass: &String) -> IRCMessage {
        IRCMessage(format!("PASS {}\r\n", pass))
    }
    fn nick(nick: &String) -> IRCMessage {
        IRCMessage(format!("NICK {}\r\n", nick))
    }
    fn join(join: &String) -> IRCMessage {
        IRCMessage(format!("JOIN #{}\r\n", join))
    }
    fn text(text: &String) -> IRCMessage {
        IRCMessage(format!("{}\r\n", text))
    }
    fn privmsg(text: &String, channel: &String) -> IRCMessage {
        IRCMessage(format!("PRIVMSG #{} :{}\r\n", channel, text))
    }
    fn pong() -> IRCMessage {
        IRCMessage("PONG :tmi.twitch.tv\r\n".to_string())
    }
}

struct IRCBotClient {
    stream: TcpStream,
    nick: String,
    secret: String,
    reader: BufReader<TcpStream>,
    sender: Sender<IRCMessage>,
    channel: String,
    ct: CommandTree,
    game: Game,
    audio: Audio,
    autosave: bool,
    client: Option<Client>,
    rng: rand::rngs::ThreadRng,
    spotify: SpotifyChecker,
    player_data: PlayerData,
}

// Class that receives messages, then sends them.
struct IRCBotMessageSender {
    writer: TcpStream,
    queue: Receiver<IRCMessage>,
}

impl IRCBotMessageSender {
    async fn launch_write(&mut self) {
        loop {
            match self.queue.recv().await {
                Ok(s) => {
                    self.writer.send(s).await;
                }
                Err(e) => {
                    println!("Uh oh, queue receive error: {}", e);
                    break;
                }
            }
            task::sleep(Duration::from_millis(100)).await;
        }
    }
}

impl IRCBotClient {
    async fn connect(
        nick: String,
        secret: String,
        channel: String,
        ct: CommandTree,
    ) -> (IRCBotClient, IRCBotMessageSender) {
        // Creates the stream object that will go into the client.
        let stream = TcpStream::connect("irc.chat.twitch.tv:6667").await.unwrap();
        // Get a stream reference to use for reading.
        let reader = BufReader::new(stream.clone());
        let (s, r) = async_std::channel::unbounded(); // could use bounded(10) or sth
        (
            IRCBotClient {
                stream: stream.clone(),
                nick,
                secret,
                reader,
                sender: s,
                channel,
                ct,
                game: Game::new(),
                audio: Audio::new(),
                autosave: false,
                client: None,
                rng: rand::thread_rng(),
                spotify: SpotifyChecker::new().await,
                player_data: PlayerData::new(),
            },
            IRCBotMessageSender {
                writer: stream,
                queue: r,
            },
        )
        // return the async class for writing back down the TcpStream instead, which contains the
        // receiver + the tcpstream clone
    }

    async fn authenticate(&mut self) -> () {
        println!("Writing password...");
        self.stream.send(TwitchFmt::pass(&self.secret)).await;
        println!("Writing nickname...");
        self.stream.send(TwitchFmt::nick(&self.nick)).await;
        println!("Writing join command...");
        self.stream.send(TwitchFmt::join(&self.channel)).await;
    }

    /*
    async fn do_elevated(&mut self, mut cmd: String) -> Command {
        if cmd.starts_with("stop") {
            Command::Stop
        } else if cmd.starts_with("raw") {
            self.sender.send(cmd.split_off(4)).await;
            Command::Continue
        } else if cmd.starts_with("say") {
            self.privmsg(cmd.split_off(4)).await;
            Command::Continue
        } else {
            Command::Continue
        }
    }
    */

    async fn do_command(&mut self, user: String, mut prefix: String, mut cmd: String) -> Command {
        let format_str = format!("[Name({}),Command({})] Result: ", user, cmd);
        let log_res = |s| println!("{}{}", format_str, s);

        // user data <3
        let pd: &mut Player = self.player_data.player(&user);
        let messager = self.sender.clone();
        let channel = self.channel.clone();

        // areweasyncyet? xd
        let send_msg = |msg: &String| {
            let msg = msg.clone();
            async move {
                match messager.send(TwitchFmt::privmsg(&msg, &channel)).await {
                    _ => {}
                };
            }
        };
        pd.sent_messages += 1;
        let tm = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0))
            .as_secs();
        if tm > (pd.last_message + /* 60s * 15m */ 60 * 15) {
            pd.last_message = tm;
            pd.files += 25;
        }

        // Compose the command
        // !todo -> prefix: !, cmd: todo
        // !!todo -> prefix: !!, cmd: todo
        // But, these need to map differently.
        // Recombine.
        if prefix == "folder " || prefix == "bot " {
            prefix = "!".to_string();
        }

        // println!("cmd({}) prefix({})", cmd, prefix);

        let node = match self.ct.find(&mut cmd) {
            Some(x) => x,
            None => {
                log_res("Skipped as no match was found.");
                return Command::Continue; // Not a valid command
            }
        };
        if prefix != node.prefix && !(prefix == "" && node.prefix == "^") {
            log_res("Skipped as prefix does not match.");
            return Command::Continue;
        }

        pd.sent_commands += 1;

        let args = cmd;
        println!("Arguments being returned -> '{}'", args);
        if node.admin_only
            && ((node.super_only && user != self.ct.superuser) || !(self.ct.admins.contains(&user)))
        {
            let _ = self
                .sender
                .send(TwitchFmt::privmsg(
                    &"Naughty naughty, that's not for you!".to_string(),
                    &self.channel,
                ))
                .await;
            log_res("Blocked as user is not bot administrator.");
            return Command::Continue;
        }
        let command = match &node.value {
            CmdValue::StringResponse(x) => {
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&x.clone(), &self.channel))
                    .await;
                log_res(format!("Returned a string response ({}).", x).as_str());
                if !node.sound.is_empty() {
                    self.audio.play_file(&node.sound)
                };
                return Command::Continue;
            }
            CmdValue::Alias(x) => {
                log_res(format!("! Didn't return an alias ({}).", x).as_str());
                return Command::Continue;
            }
            CmdValue::Generic(x) => {
                if x.as_str() == "debug:use_internal_mapping" {
                    &args
                } else {
                    x
                }
            }
        };
        lazy_static! {
            static ref COMMAND_RE: Regex = Regex::new(r"^([^\s\w]?)(.*?)\s+(.+)$").unwrap();
        }
        match command.as_str() {
            "meta:insert" | "meta:edit" => {
                // Let's ... try to get this to work I guess.
                let (mut newprefix, newcmdunc, newresp) = match COMMAND_RE.captures(args.as_str()) {
                    // there must be a better way...
                    Some(caps) => (caps.str_at(1), caps.str_at(2), caps.str_at(3)),
                    None => {
                        send_msg(
                            &"Nice try, but you have been thwarted by the command regex! Mwuahaha."
                                .to_string(),
                        )
                        .await;
                        return Command::Continue;
                    }
                };
                if newprefix == "" {
                    newprefix = "!".to_string();
                }
                let newcmd = (&newcmdunc.as_str()).to_lowercase();
                if newcmd != newcmdunc {
                    let _ = self
                        .sender
                        .send(TwitchFmt::privmsg(
                            &"Warning: Converting to case-insensitive.".to_string(),
                            &self.channel,
                        ))
                        .await;
                }

                if let Some(x) = self.ct.find(&mut newcmd.to_string()) {
                    if !x.editable {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &"Command is not editable.".to_string(),
                                &self.channel,
                            ))
                            .await;
                        return Command::Continue;
                    }
                };

                let keycmd = newcmd.to_string();
                if self.ct.contains(&keycmd) {
                    if command != "meta:edit" {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &"Command already exists. Use !edit instead.".to_string(),
                                &self.channel,
                            ))
                            .await;
                        return Command::Continue;
                    }
                    if let CmdValue::Generic(_) = self.ct.get_always(&keycmd).value {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &"You cannot edit Generic commands.".to_string(),
                                &self.channel,
                            ))
                            .await;
                        return Command::Continue;
                    }
                    self.ct
                        .set_value(&keycmd, CmdValue::StringResponse(newresp.to_string()));
                    self.ct.set_prefix(&keycmd, newprefix.clone());
                    println!(
                        "New prefix: {}, new value: {} for keycmd: {}",
                        newprefix, newresp, keycmd
                    );
                    self.ct.dump_file(Path::new("commands.json"));
                } else {
                    self.ct.insert(
                        newcmd.to_string(),
                        CommandNode::new(CmdValue::StringResponse(newresp.to_string()))
                            .with_prefix(newprefix),
                    );
                    log_res("Saving commands to commands.json");
                    self.ct.dump_file(Path::new("commands.json"));
                }
            }
            "meta:isadmin" => self
                .sender
                .send(TwitchFmt::privmsg(
                    &format!("Status of {}: {}", args, self.ct.admins.contains(&args)),
                    &self.channel,
                ))
                .await
                .unwrap(),
            "meta:issuper" => self
                .sender
                .send(TwitchFmt::privmsg(
                    &format!("Status of {}: {}", args, self.ct.superuser == args),
                    &self.channel,
                ))
                .await
                .unwrap(),
            "meta:help" => self
                .sender
                .send(TwitchFmt::privmsg(
                    &"No help for you, good sir!".to_string(),
                    &self.channel,
                ))
                .await
                .unwrap(),
            "meta:stop" => {
                log_res("Stopping as requested by command.");
                return Command::Stop;
            }
            "meta:playerdata" => {
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(
                        &format!(
                            "{}",
                            &self.player_data.player_or(&args.to_lowercase(), &user)
                        ),
                        &self.channel,
                    ))
                    .await
                    .unwrap();
            }
            "meta:say" => {
                log_res("Sent a privmsg.");
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&args, &self.channel))
                    .await;
            }
            "meta:say_raw" => {
                log_res("Send a raw message.");
                let _ = self.sender.send(TwitchFmt::text(&args)).await;
            }
            "meta:reload_commands" => {
                log_res("Reloaded commands from file.");
                self.ct = CommandTree::from_json_file(Path::new("commands.json"));
            }
            "meta:save_commands_test" => {
                log_res("Saving commands to commands.test.json");
                self.ct.dump_file(Path::new("commands.test.json"));
            }
            "meta:save_commands" => {
                log_res("Saving commands to commands.json");
                self.ct.dump_file(Path::new("commands.json"));
            }
            "game:bet_for" => {
                log_res("Bet that it works!");
                match self.game.bet_for(&user, &args) {
                    Err(e) => {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(&e, &self.channel))
                            .await;
                    }
                    _ => {}
                }
            }
            "game:bet_against" => {
                log_res("Bet that it fails!");
                match self.game.bet_against(&user, &args) {
                    Err(e) => {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(&e, &self.channel))
                            .await;
                    }
                    _ => {}
                }
            }
            "game:failed" => {
                log_res("Noted that it failed.");
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&self.game.failed(), &self.channel))
                    .await;
                if self.autosave {
                    self.game.save(); // Note: This should really be done in Game's code,
                                      // this is just a rushed impl
                }
            }
            "game:worked" => {
                log_res("Noted that it succeeded!");
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&self.game.worked(), &self.channel))
                    .await;
                if self.autosave {
                    self.game.save(); // Note: This should really be done in Game's code,
                                      // this is just a rushed impl
                }
            }
            "game:status" => {
                log_res("Returned a player's status.");
                let query = if args == "" { &user } else { &args };
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&self.game.status(query), &self.channel))
                    .await;
            }
            "game:reload" => {
                log_res("Reloaded the game.");
                self.game.reload();
            }
            "game:save" => {
                log_res("Saved the game.");
                self.game.save();
            }
            "game:autosave" => {
                log_res("Turned on autosave.");
                self.autosave = true;
            }
            "feature:rsg" => {
                log_res("Printing what RSG does.");
                if let Ok(get_resp) = reqwest::get("http://shnenanigans.pythonanywhere.com/").await
                {
                    if let Ok(get_text) = get_resp.text().await {
                        if get_text.len() > 100 {
                            let _ = self
                                .sender
                                .send(TwitchFmt::privmsg(
                                    &String::from(
                                        "@shenaningans this command be broken again :sob:",
                                    ),
                                    &self.channel,
                                ))
                                .await;
                        } else {
                            let _ = self
                                .sender
                                .send(TwitchFmt::privmsg(&get_text, &self.channel))
                                .await;
                        }
                    }
                }
            }
            "feature:tridentpb" => {
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(
                        &format!("{}'s trident pb is: {}", &user, pd.max_trident),
                        &self.channel,
                    ))
                    .await;
            }
            "feature:tridentlb" => {
                let lb = self.player_data.leaderboard();
                log_res(format!("Generated leaderboard: {}", &lb).as_str());
                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(
                        &format!("Trident Leaderboard: {}", &lb),
                        &self.channel,
                    ))
                    .await;
                return Command::Continue;
            }
            "feature:trident" => {
                // acc data
                pd.tridents_rolled += 1;
                let inner: i32 = self.rng.gen_range(0..=250);
                let res: i32 = self.rng.gen_range(0..=inner);
                let restr = res.to_string();
                // res is your roll

                let is_pb = pd.max_trident < (res as u64);
                let _prev_pb = pd.max_trident;
                if is_pb {
                    pd.max_trident = res as u64;
                }

                lazy_static! {
                    static ref SCRATCH: std::sync::Mutex<HashMap<String, PlayerScratch>> =
                        Mutex::new(HashMap::new());
                }
                let mut scratch = SCRATCH.lock().unwrap();
                let prev_roll = scratch
                    .entry(user.clone())
                    .or_insert_with(|| PlayerScratch::new())
                    .last_trident;
                scratch.get_mut(&user).unwrap().last_trident = res;

                pd.max_trident = std::cmp::max(pd.max_trident, res as u64);
                pd.trident_acc += res as u64;

                let norm_fmt = |s: &String| {
                    s.replace("{ur}", &pd.name())
                        .replace("{t.r}", &restr)
                        .replace("{t.rolled}", &pd.tridents_rolled.to_string())
                };

                // let's do a few things with this before we do anything crazy
                if is_pb && pd.tridents_rolled > 5 && res != 250
                /* don't overwrite 250 responses */
                {
                    send_msg(&norm_fmt(random_response("TRIDENT_PB_GENERIC"))).await;
                    return Command::Continue;
                }

                if pd.tridents_rolled <= 5 && res >= 100 && res != 250 {
                    send_msg(&norm_fmt(random_response("EARLY_HIGH_TRIDENT"))).await;
                    return Command::Continue;
                }

                if pd.tridents_rolled == 1 && res != 250 {
                    send_msg(&norm_fmt(random_response("FIRST_TRIDENT_GENERIC"))).await;
                    return Command::Continue;
                }

                if res < 5 && res == prev_roll {
                    send_msg(&norm_fmt(random_response("TRIDENT_DOUBLE_LOW"))).await;
                    return Command::Continue;
                }

                if res < 5 && thread_rng().gen_bool(1.0 / 6.0) {
                    let deduction = thread_rng().gen_range(12..32);
                    send_msg(&norm_fmt(&format!("Ew... a {{t.r}}. What a gross low roll, {{ur}}. I'm deducting {} files from you, just for that...", deduction))).await;
                    pd.files -= deduction;
                    return Command::Continue;
                }

                let selection = self.rng.gen_range(0..=100);
                if selection < 77 {
                    const LOSER_STRS: &'static [&'static str] = &["Wow, {} rolled a 0? What a loser!", "A 0... try again later, {} :/", "Oh look here, you rolled a 0. So sad! Alexa, play Despacito :sob:", "You rolled a 0. Everyone: Don't let {} play AA. They don't have the luck - er, skill - for it."];
                    const BAD_STRS: &'static [&'static str] = &["Hehe. A 1. So close, and yet so far, eh {}?", "{} rolled a 1. Everyone clap for {}. They deserve a little light in their life.", "A 1. Nice work, {}. I'm sure you did great in school.", "1. Do you know how likely that is, {}? You should ask PacManMVC. He has a spreadsheet, just to show how bad you are.", "Excuse me, officer? This 1-rolling loser {} keeps yelling 'roll trident!' at me and I can't get them to stop."];
                    const OK_STRS: &'static [&'static str] = &["{N}. Cool. That's not that bad.", "{N}! Wow, that's great! Last time, I rolled a 0, and everyone made fun of me :sob: I'm so jealous of you :sob:", "{N}... not terrible, I suppose.", "{N}. :/ <- That's all I have to say.", "{N}. Yeppers. Yep yep yep. Real good roll you got there, buddy.", "{N}! Whoa. A whole {N} more durability than 0, and you still won't get thunder, LOL!", "Cat fact cat fact! Did you know that the first {N} cats that spawn NEVER contain a Calico? ...seriously, where is my Calico??"];
                    const GOOD_STRS: &'static [&'static str] = &["{N}. Wow! I'm really impressed :)", "{N}! Cool, cool. Cool. Coooool.", "{N}... Hm. It's so good, and yet, really not that good.", "{N}. Here's a cat fact: Did you know they can eat up to 350 fish in a single day?!", "{N}. I lied about the cat fact, just FYI. I don't know anything about cats. He doesn't let me use the internet :(", "{N}. I want a cat. I'd treat it well and not abandon it in a random village.", "{N} temples checked before enchanted golden apple."];
                    const GREAT_STRS: &'static [&'static str] = &["{N}. Great work!!! That's going in your diary, I'm sure.", "{N}! Whoaaaaa. I'm in awe.", "{N}... Pretty great! You know what would be better? Getting outside ;) ;) ;)", "{N}. Oh boy! We got a high roller here!"];
                    if res == 0 {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &LOSER_STRS[self.rng.gen_range(0..LOSER_STRS.len())]
                                    .replace("{}", &pd.name()),
                                &self.channel,
                            ))
                            .await;
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &format!("/timeout {} 10", &user),
                                &self.channel,
                            ))
                            .await;
                    } else if res == 1 {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &BAD_STRS[self.rng.gen_range(0..BAD_STRS.len())]
                                    .replace("{}", &pd.name()),
                                &self.channel,
                            ))
                            .await;
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &format!("/timeout {} 15", &user),
                                &self.channel,
                            ))
                            .await;
                    } else if res < 100 {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &OK_STRS[self.rng.gen_range(0..OK_STRS.len())]
                                    .replace("{N}", &restr),
                                &self.channel,
                            ))
                            .await;
                    } else if res < 200 {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &GOOD_STRS[self.rng.gen_range(0..GOOD_STRS.len())]
                                    .replace("{N}", &restr),
                                &self.channel,
                            ))
                            .await;
                    } else if res < 250 {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &GREAT_STRS[self.rng.gen_range(0..GREAT_STRS.len())]
                                    .replace("{N}", &restr),
                                &self.channel,
                            ))
                            .await;
                    } else {
                        assert!(res == 250);
                        let _ = send_msg(&format!("You did it, {}! You rolled a perfect 250! NOW STOP SPAMMING MY CHAT, YOU NO LIFE TWITCH ADDICT!", &pd.name())).await;
                    }
                } else if selection < 82 && res != 250 {
                    send_msg(&norm_fmt(random_response("MISC_RARE_TRIDENTS"))).await;
                } else if selection < 85 && res < 10 {
                    send_msg(&norm_fmt(random_response("MISC_LOW_TRIDENTS"))).await;
                } else {
                    // ok, let's do this a bit better.
                    let _ = self
                        .sender
                        .send(TwitchFmt::privmsg(
                            &rare_trident(res, self.rng.gen_range(0..=4096), &pd.name()),
                            &self.channel,
                        ))
                        .await;
                }
            }
            "feature:enchant" => {
                const ROMAN_MAP: &[&str] = &["I", "II", "III", "IV", "V"];
                const GREAT_ROLLS: &'static [&'static str] = &["Impressive! You've got yourself a {0} {1} book for {2} levels with {3} bookshel{4}.", "A truly magical outcome! {0} {1} awaits you for {2} levels with {3} bookshel{4}.", "Your enchantment game is strong! {0} {1} for you for the price of {2} levels. Not bad for {3} bookshel{4}.", "Surely you must be RNG-manipulating! I mean, {0} {1} for {2} levels!? I guess it did take {3} bookshel{4} to get."];
                const GOOD_ROLLS: &'static [&'static str] = &["{0} {1} from {3} bookshel{4}? Not too shabby! Yours for {2} levels.", "A respectable roll! Can't go wrong with {0} {1} for {2} levels with {3} bookshel{4}.", "{0} {1} for {2} levels. Could be worse, lol. I like your {3} bookshel{4}.", "Wow, not bad! {0} {1} for {2} levels with {3} bookshel{4}."];
                const BAD_ROLLS: &'static [&'static str] = &["{0} {1} for {2} levels? Could be worse, I guess... Might need more than {3} bookshel{4}...", "You rolled {0} {1} for {2} levels with {3} bookshel{4}. Keep trying!", "You rolled {0}! Nice!! Oh wait, its only {0} {1}. Oh well, it's only {2} levels at least. Maybe try using more than {3} bookshel{4} or something."];
                const TERRIBLE_ROLLS: &'static [&'static str] = &["{0}.. you know what. I can't be bothered telling you the level, it's too embarrassing. Let's just pretend it's a good level.", "Wow.. a {0} {1}.. amazing.. I wouldn't spend {2} levels on that, {5}.", "{0} {1}... zzz... something something {2} levels something {3} bookshel{4} idk I can't be bothered anymore", "Jackpot! You scored a {0} {1}. What are the odds of being that bad?? {2} levels?? Honestly. Get more bookshelves, {3} isn't enough.", "Yeah I'm not saying the response. That's just embarassing, {5}. Almost as embarassing as misspelling embarrassing."];
                match roll_enchant().filter(|o| o.level > 0 && (o.level as usize) < ROMAN_MAP.len())
                {
                    Some(offer) => {
                        pd.enchants_rolled += 1;
                        let response = if offer.special_response {
                            let resp_list = if offer.bookshelves >= 13 && offer.row == 3 {
                                GREAT_ROLLS
                            } else if offer.bookshelves >= 10 && offer.row > 1 {
                                GOOD_ROLLS
                            } else if offer.bookshelves < 2 {
                                TERRIBLE_ROLLS
                            } else {
                                BAD_ROLLS
                            };
                            resp_list[self.rng.gen_range(0..resp_list.len())]
                                .replace("{0}", &offer.enchant.name)
                                .replace("{1}", ROMAN_MAP[offer.level as usize - 1])
                                .replace("{2}", &offer.cost.to_string())
                                .replace("{3}", &offer.bookshelves.to_string())
                                .replace("{4}", if offer.bookshelves == 1 { "f" } else { "ves" })
                                .replace("{5}", &pd.name())
                        } else {
                            format!(
                                "You rolled {0} {1} for {2} levels with {3} bookshel{4}!",
                                &offer.enchant.name,
                                ROMAN_MAP[offer.level as usize - 1],
                                offer.cost,
                                offer.bookshelves,
                                if offer.bookshelves == 1 { "f" } else { "ves" }
                            )
                        };
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(&response, &self.channel))
                            .await;
                    }
                    _ => {
                        let _ = self
                            .sender
                            .send(TwitchFmt::privmsg(
                                &"Somehow you rolled an impossible enchant... good for you"
                                    .to_string(),
                                &self.channel,
                            ))
                            .await;
                    }
                }
            }
            "feature:nick" => {
                log_res("Setting nick");
                if args.len() > 0 {
                    pd.nick = Some(args);
                }
                send_msg(&random_response("NICK_SET").replace("{ur}", &pd.name())).await;
                return Command::Continue;
            }
            "admin:nick" => {
                log_res("Setting nick (admin)");
                let v: Vec<&str> = args.splitn(2, "|").collect();
                if v.len() != 2 {
                    send_msg(&"Not enough arguments.".to_string()).await;
                    return Command::Continue;
                }
                let pde = self.player_data.player(&v[0].to_string());
                pde.nick = Some(v[1].to_string());
                return Command::Continue;
            }
            "feature:elo" => {
                log_res("Doing elo things");
                let un: String = match args.len() {
                    0..=2 => "DesktopFolder".into(),
                    _ => args.clone(),
                };
                if let Ok(r) =
                    reqwest::get(format!("https://mcsrranked.com/api/users/{}", un)).await
                {
                    match r.json::<MCSRAPIResponse>().await {
                                Ok(j) => {
                                    send_msg(&format!(
                                                "Elo for {0}: {1} (Rank #{2}) Season games: {3} {4} Graph -> https://disrespec.tech/elo/?username={0}",
                                                un,
                                                j.data.elo_rate,
                                                j.data.elo_rank,
                                                j.data.season_played,
                                                j.data.win_loss(),
                                            )
                                        )
                                        .await
                                }
                                Err(e) => {
                                    println!("{}", e);
                                    send_msg(
                                            &format!("Bad MCSR API response for {}.", un)
                                        )
                                        .await
                                }
                            };
                } else {
                    send_msg(&format!("Failed to query MCSR API.")).await;
                }
            }
            "core:play_audio" => {
                log_res("Tested audio.");
                self.audio.play();
            }
            "core:get_song" => {
                log_res("Attempting to contact Spotify Web Player...");
                if let None = self.client {
                    log_res("Attempting to create Spotify connection...");
                    let mut headers = header::HeaderMap::new();
                    let auth = match std::fs::read_to_string("auth/spotify.txt") {
                        Ok(s) => s.trim().to_string(),
                        Err(e) => {
                            log_res(&format!("Could not open: auth/spotify.txt (err: {})", e));
                            return Command::Continue;
                        }
                    };

                    // https://developer.spotify.com/console/get-users-currently-playing-track/
                    headers.insert(
                        "Authorization",
                        header::HeaderValue::try_from(format!("Bearer {}", auth)).unwrap(),
                    );
                    headers.insert(
                        "Accept",
                        header::HeaderValue::from_static("application/json"),
                    );
                    headers.insert(
                        "Content-Type",
                        header::HeaderValue::from_static("application/json"),
                    );
                    match Client::builder().default_headers(headers).build() {
                        Ok(try_client) => self.client.insert(try_client),
                        Err(e) => {
                            log_res(format!("Encountered error: {:?}", e).as_str());
                            return Command::Continue;
                        }
                    };
                }
                match self.client.as_ref() {
                    Some(spotify) => match spotify
                        .get("https://api.spotify.com/v1/me/player/currently-playing")
                        .send()
                        .await
                    {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.json::<APIResponse>().await {
                                    Ok(parsed) => {
                                        let artist = match parsed.item.artists.len() {
                                            0 => "".to_string(),
                                            _ => format!(" - {}", parsed.item.artists[0].name),
                                        };
                                        let _ = self
                                            .sender
                                            .send(TwitchFmt::privmsg(
                                                &format!("{}{}", parsed.item.name, artist),
                                                &self.channel,
                                            ))
                                            .await;
                                        return Command::Continue;
                                    }
                                    Err(_) => {
                                        return Command::Continue;
                                    }
                                }
                            } else {
                                let _ = self
                                    .sender
                                    .send(TwitchFmt::privmsg(
                                        &"Failed to get song (bad request).".to_string(),
                                        &self.channel,
                                    ))
                                    .await;
                            }
                        }
                        Err(_) => {
                            let _ = self
                                .sender
                                .send(TwitchFmt::privmsg(
                                    &"Failed to get song (bad request).".to_string(),
                                    &self.channel,
                                ))
                                .await;
                        }
                    },
                    None => {}
                }
            }
            "core:functioning_get_song" => {
                let song_response = self
                    .spotify
                    .spotify
                    .current_playing(None, Some([&AdditionalType::Track]))
                    .await;

                let message = match song_response {
                    Ok(playing) => match playing {
                        Some(playing) => match playing.item {
                            Some(playable_item) => match playable_item {
                                PlayableItem::Track(track) => {
                                    let artists = track.artists;

                                    let mut message = String::new();
                                    for (i, artist) in artists.iter().enumerate() {
                                        if i != artists.len() - 1 {
                                            message += &format!("{}, ", artist.name);
                                        } else {
                                            message += &format!("{} - ", artist.name);
                                        }
                                    }

                                    message += &track.name;
                                    message
                                }
                                _ => String::from(
                                    "no song, I'm just listening to Folding@Home podcast :)",
                                ),
                            },
                            None => String::from("Error: No song is currently playing."),
                        },
                        None => String::from("Error: No song is currently playing."),
                    },
                    Err(err) => {
                        println!("Error when getting the song: {:?}", err);
                        String::from("Error: Couldn't get the current song.")
                    }
                };

                let _ = self
                    .sender
                    .send(TwitchFmt::privmsg(&message, &self.channel))
                    .await;
            }
            "internal:cancel" => {
                self.audio.stop();
            }
            _ => {
                log_res("! Not yet equipped to handle this command.");
                return Command::Continue;
            }
        }
        log_res("Successfully executed command.");
        Command::Continue
    }

    async fn handle_twitch(&mut self, line: &String) -> Command {
        match line.trim() {
            "" => Command::Stop,
            "PING :tmi.twitch.tv" => {
                let _ = self.sender.send(TwitchFmt::pong()).await;
                Command::Continue
            }
            _ => Command::Continue,
        }
    }

    async fn launch_read(&mut self) -> Result<String> {
        lazy_static! {
            static ref COMMAND_RE: Regex =
                Regex::new(r"^(bot |folder |[^\s\w]|)\s*(.*?)\s*$").unwrap();
            static ref PRIV_RE: Regex =
                Regex::new(r":(\w*)!\w*@\w*\.tmi\.twitch\.tv PRIVMSG #\w* :\s*(.*)").unwrap();
        }
        let mut line = String::new();

        loop {
            line.clear();
            match self.reader.read_line(&mut line).await {
                Ok(_) => {
                    println!("[Received] Message: '{}'", line.trim());

                    // maybe save our game data real quick...
                    static LAST_SAVE: AtomicU64 = AtomicU64::new(0);

                    let tm = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or(Duration::from_secs(0))
                        .as_secs();
                    if (LAST_SAVE.load(Ordering::Relaxed) + 60 * 5) < tm {
                        LAST_SAVE.store(tm, Ordering::Relaxed);
                        println!("[Note] Autosaving player data.");
                        self.player_data.save();
                    }

                    // First, parse if it's a private message, or a skip/ping/etc.
                    let (name, message) = match PRIV_RE.captures(line.as_str()) {
                        // there must be a better way...
                        Some(caps) => (caps.str_at(1), caps.str_at(2)),
                        None => match self.handle_twitch(&line).await {
                            // todo - reconnect instead of stopping.
                            Command::Stop => return Ok("Stopped due to twitch.".to_string()),
                            _ => continue,
                        },
                    };

                    // Now we filter based on the username & the message sent.
                    //match filter(&name, &message) {
                    //    FilterResult::Skip => continue,
                    //    FilterResult::Ban(reason) => self.ban(&name, &reason).await,
                    //    _ => {}
                    //}

                    // Now, we parse the command out of the message.
                    let (prefix, command) = match COMMAND_RE.captures(message.as_str()) {
                        // there must be a better way...
                        Some(caps) => (caps.str_at(1), caps.str_at(2)),
                        // this never happens btw, we basically (?) always match (??)
                        None => continue,
                    };

                    // Finally, we actually take the command and maybe take action.
                    if let Command::Stop = self.do_command(name, prefix, command).await {
                        return Ok("Received stop command.".to_string());
                    }
                }
                Err(e) => {
                    println!("Encountered error: {}", e);
                    continue;
                }
            }
        }
    }
}

fn get_file_trimmed(filename: &str) -> String {
    match std::fs::read_to_string(filename) {
        Ok(s) => s.trim().to_string(),
        Err(e) => panic!("Could not open file ({}):\n{}", filename, e),
    }
}

async fn async_main() {
    let nick = get_file_trimmed("auth/user.txt");
    let secret = get_file_trimmed("auth/secret.txt");
    let channel = get_file_trimmed("auth/id.txt");

    println!("Nick: {} | Secret: {} | Channel: {}", nick, secret, channel);

    // Supported commands, loaded from JSON.
    let ct = CommandTree::from_json_file(Path::new("commands.json"));
    //ct.dump_file(Path::new("commands.parsed.json"));
    let (mut client, mut forwarder) = IRCBotClient::connect(nick, secret, channel, ct).await;
    client.authenticate().await;

    select! {
        return_message = client.launch_read().fuse() => match return_message {
            Ok(message) => { println!("Quit (Read): {}", message); },
            Err(error) => { println!("Error (Read): {}", error); }
        },
        () = forwarder.launch_write().fuse() => {}
    }
}

fn main() {
    //println!("{}", rare_trident(17, 0, &String::from("hi")));
    //println!("{}", rare_trident(17, 0, &String::from("hi")));
    task::block_on(async_main())
}
