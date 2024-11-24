use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::Write;
use crate::args::Args;

#[derive(Debug, Default)]
pub struct Config {
    pub dir: Option<String>,
    pub dbfilename: Option<String>,
}

impl Config {
    pub fn new() -> Result<Self> {
        let args = Args::try_parse().unwrap();
        
        if args.dir.is_none() && args.dbfilename.is_none() {
            Self::from_file()
        } else {
            let config = Config {
                dir: args.dir,
                dbfilename: args.dbfilename,
            };
            config.save_to_file()?;
            Ok(config)
        }
    }

    fn save_to_file(&self) -> Result<()> {
        let mut config_content = String::new();

        if let Some(dir) = self.dir.as_ref() {
            config_content.push_str(&format!("dir {}\n", dir));
        }

        if let Some(dbfilename) = self.dbfilename.as_ref() {
            config_content.push_str(&format!("dbfilename {}\n", dbfilename));
        }

        if !config_content.is_empty() {
            let mut file = File::create("redis.conf")?;
            file.write_all(config_content.as_bytes())?;
        }

        Ok(())
    }

    fn from_file() -> Result<Self> {
        let config = std::fs::read_to_string("redis.conf").unwrap_or_default();
        let mut dir = None;
        let mut dbfilename = None;

        for line in config.lines() {
            let mut parts = line.split_whitespace();
            match parts.next() {
                Some("dir") => dir = parts.next().map(String::from),
                Some("dbfilename") => dbfilename = parts.next().map(String::from),
                _ => continue,
            }
        }

        Ok(Config { dir, dbfilename })
    }
}
