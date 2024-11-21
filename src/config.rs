use anyhow::Result;
use clap::Parser;
use std::fs::File;
use std::io::Write;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Directory path (optional)
    #[arg(long)]
    pub dir: Option<String>,

    /// DB filename (optional)
    #[arg(long)]
    pub dbfilename: Option<String>,
}

impl Args {
    pub fn load() -> Result<Self> {
        let args = Self::parse();

        if args.dir.is_none() && args.dbfilename.is_none() {
            Self::read_config()
        } else {
            args.save_config()?;
            Ok(args)
        }
    }

    fn save_config(&self) -> Result<()> {
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

    fn read_config() -> Result<Self> {
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

        Ok(Self { dir, dbfilename })
    }
}
