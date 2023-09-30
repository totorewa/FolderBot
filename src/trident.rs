use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    path::Path,
};

use rand::{self, seq::SliceRandom};

use lazy_static::lazy_static;
use regex::Regex;

// This should be more generic in the future, but it works for now.
struct ResponseDB {
    responses: HashMap<String, Vec<String>>,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

impl ResponseDB {
    fn new() -> ResponseDB {
        ResponseDB {
            responses: HashMap::new(),
        }
    }

    fn from_db(filename: &str) -> ResponseDB {
        lazy_static! {
            static ref CATEGORY_RE: Regex = Regex::new(r"^### *([\w_]+):\s*$").unwrap();
        }
        let mut ret = ResponseDB::new();
        let mut current_key = String::from("DEFAULT_KEY");
        for line in read_lines(filename).unwrap() {
            let line = line.unwrap();
            if line.trim().is_empty() {
                continue;
            }
            if line.starts_with("<!--") {
                continue;
            }
            let Some(caps) = CATEGORY_RE.captures(&line) else {
                ret.responses.entry(current_key.clone()).or_insert(Vec::new()).push(line);
                continue;
            };
            current_key = String::from(&caps[1]);
        }
        ret
    }

    fn get(&self, key: &str) -> &Vec<String> {
        self.responses.get(key).unwrap()
    }
}

fn get_db() -> &'static ResponseDB {
    lazy_static! {
        static ref DB: ResponseDB = ResponseDB::from_db("responses");
    }
    &DB
}

pub fn random_response(key: &str) -> &String {
    get_db().get(key).choose(&mut rand::thread_rng()).unwrap()
}

pub fn has_responses(key: &str) -> bool {
    get_db().responses.contains_key(key)
}
