use std::{
    collections::BTreeMap,
    f64::consts::{E, EULER_GAMMA, PI},
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

use crate::functions;

pub const INSERTION_PREFIX: &[char] = &['V', 'A', 'R', ' '];
pub const DELETION_PREFIX: &[char] = &['D', 'E', 'L', ' '];

pub struct VarMap {
    pub map: BTreeMap<String, f64>,
}

impl VarMap {
    pub fn new() -> Result<VarMap, io::Error> {
        let mut var_map = VarMap {
            map: BTreeMap::from([
                (String::from("pi"), PI),
                (String::from("e"), E),
                (String::from("euler_gamma"), EULER_GAMMA),
            ]),
        };

        var_map.populate_from_config()?;

        Ok(var_map)
    }

    pub fn handle(&mut self, line: String) -> HandleResult {
        let line = line.trim();
        if let Some((key, val)) = parse_ins(line) {
            return if self.map.insert(key, val).is_none() {
                HandleResult::Insertion
            } else {
                HandleResult::Update
            };
        }

        if let Some(var_name) = parse_del(line) {
            return if self.map.remove(&var_name).is_none() {
                HandleResult::DeletionFail
            } else {
                HandleResult::DeletionSuccess
            };
        }

        HandleResult::GenericFail
    }

    fn populate_from_config(&mut self) -> io::Result<()> {
        let file = match File::open(crate::var_config_path()) {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(err) => return Err(err),
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            self.handle(line?);
        }

        Ok(())
    }
}

impl Drop for VarMap {
    fn drop(&mut self) {
        let Ok(mut file) = File::create(crate::var_config_path()) else {
            panic!("Couldn't open file to write variable config to!");
        };

        for (key, val) in &self.map {
            let _ = writeln!(file, "VAR {key}:{val}");
        }
    }
}

pub enum HandleResult {
    Insertion,
    DeletionFail,
    DeletionSuccess,
    Update,
    GenericFail,
}

fn parse_ins(line: &str) -> Option<(String, f64)> {
    if !line.starts_with(INSERTION_PREFIX) {
        return None;
    }

    let line: String = line.chars().skip(4).collect();
    if line.contains(' ')
        || line.contains(functions::COEFF_DELIM)
        || line.chars().filter(|ch| *ch == ':').count() != 1
    {
        return None;
    }

    let name: String = line.chars().take_while(|ch| *ch != ':').collect();
    if name.is_empty() {
        return None;
    }

    let mut comp_div = 1.0;
    let mut is_float = false;
    let mut acc = 0.0;

    for ch in line.chars().skip_while(|ch| *ch != ':').skip(1) {
        match ch {
            ch if let Some(digit) = ch.to_digit(10) => acc = acc * 10.0 + digit as f64,
            '.' if is_float == false => {
                is_float = true;
                continue;
            }
            _ => return None,
        }

        if is_float {
            comp_div *= 10.0;
        }
    }

    Some((name, acc / comp_div))
}

fn parse_del(line: &str) -> Option<String> {
    if !line.starts_with(DELETION_PREFIX) {
        return None;
    }

    let line: String = line.chars().skip(4).collect();
    if line.contains(' ') || line.contains(':') {
        return None;
    }

    Some(line)
}
