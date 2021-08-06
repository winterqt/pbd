use anyhow::{Context, Result};
use porkbun::Porkbun;
use serde::Deserialize;
use std::{collections::HashMap, env, fs};

use crate::porkbun::{Record, RecordType};

mod porkbun;

#[derive(Deserialize)]
struct Config {
    api_key: String,
    secret_api_key: String,
    #[serde(default = "default_ttl")]
    ttl: u32,
    domains: HashMap<String, Vec<String>>,
}

fn default_ttl() -> u32 {
    300
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("usage: {} <path/to/confg.json>", args[0]);
        return Ok(());
    }

    let config: Config = {
        let content = fs::read_to_string(&args[1])
            .with_context(|| format!("Failed to read config from {}", args[1]))?;

        serde_json::from_str(&content).with_context(|| "Failed to parse config")?
    };

    let porkbun = Porkbun::new(config.api_key, config.secret_api_key);

    let ip_address = porkbun
        .ping()
        .with_context(|| "Failed to authenticate with Porkbun")?;

    let ip_address = ip_address.as_str();

    for (domain, records) in config.domains {
        let curr_records = porkbun
            .records(&domain)
            .with_context(|| format!("Failed to get current records for {}", domain))?;

        for record in records {
            if let Some(record) = curr_records.iter().find(|r| {
                &r.name == if record == "@" { &domain } else { &record } && r.typ == RecordType::A
            }) {
                println!("existing record found: {}", record.id);

                if record.content == ip_address {
                    println!("\tOK");
                } else {
                    println!("\tneeds updating");

                    porkbun
                        .edit_record(&domain, record, ip_address.to_string())
                        .with_context(|| {
                            format!("Failed to update record {} on domain {}", record.id, domain)
                        })?;
                }

                continue;
            }

            println!("creating new record for {}", domain);

            porkbun
                .create_record(
                    &domain,
                    &Record {
                        id: 0,
                        name: if record == "@" {
                            String::new()
                        } else {
                            record.clone()
                        },
                        typ: RecordType::A,
                        content: ip_address.to_string(),
                        ttl: config.ttl,
                        priority: 0,
                    },
                )
                .with_context(|| format!("Failed to create record for {} on {}", record, domain))?;
        }
    }

    Ok(())
}
