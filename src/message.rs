use crate::core::LogInfo;
use crate::parse::Encounter;

use std::fmt;
use std::fmt::Display;

use serde::Deserialize;
use serde::Serialize;

pub trait Generator {
    type Message: Display + Serialize;

    fn generate(&self, logs: &[LogInfo]) -> Self::Message;
}

pub struct TextGenerator {}
pub struct WebhookGenerator {}

impl Generator for TextGenerator {
    type Message = Text;

    fn generate(&self, logs: &[LogInfo]) -> Text {
        let content = logs.iter().fold(String::new(), |acc, log| {
            let duration = fmt_time2(log.encounter.phases[0].duration());
            let (success, sur) = if log.encounter.success {
                ("Success", "**")
            } else {
                ("Defeat", "**")
            };
            format!(
                "{}\n{} - {}{}{} in {}",
                acc, log.log.link, sur, success, sur, duration
            )
        });
        Text {
            content,
            username: "snek".to_string(),
            avatar_url: "https://i.imgur.com/IizO35l.png".to_string(),
        }
    }
}

impl Generator for WebhookGenerator {
    type Message = Webhook;

    fn generate(&self, logs: &[LogInfo]) -> Webhook {
        logs.iter().fold(Webhook::default(), |acc, log| {
            let embed = Embed::from_log(&log.log.link, &log.encounter);
            acc.add_embed(embed)
        })
    }
}

#[derive(Debug, Serialize)]
pub struct Text {
    username: String,
    avatar_url: String,
    content: String,
}

impl Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content.replace("*", ""))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Webhook {
    content: String,
    username: String,
    avatar_url: String,
    embeds: Vec<Embed>,
}

impl Display for Webhook {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.content)?;
        for embed in &self.embeds {
            writeln!(f)?;
            writeln!(f, "{} ({})", embed.title, embed.url)?;
            writeln!(f, "{}", embed.description.replace("*", ""))?;
        }
        Ok(())
    }
}

impl Webhook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_data(link: &str, encounter: &Encounter) -> Self {
        Self::new().add_embed(Embed::from_log(link, encounter))
    }

    pub fn with_content(mut self, content: &str) -> Self {
        self.content = content.to_owned();
        self
    }

    pub fn add_embed(mut self, embed: Embed) -> Self {
        self.embeds.push(embed);
        self
    }
}

impl Default for Webhook {
    fn default() -> Self {
        Self {
            content: String::new(),
            username: "snek".to_string(),
            avatar_url: "https://i.imgur.com/IizO35l.png".to_string(),
            embeds: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Embed {
    title: String,
    description: String,
    url: String,
    color: u32,
}

impl Embed {
    pub const fn new(title: String, description: String, url: String, color: u32) -> Self {
        Self {
            title,
            description,
            url,
            color,
        }
    }

    pub fn from_log(link: &str, encounter: &Encounter) -> Self {
        let color = if encounter.success {
            0x15_83_d1
        } else {
            0xd1_3e_15
        };
        Self::new(
            encounter.target.to_string(),
            describe(encounter),
            link.to_string(),
            color,
        )
    }
}

fn describe(encounter: &Encounter) -> String {
    let status_line = status_msg(encounter);

    let mut phase_line = if encounter.phases[0].name.is_some() {
        "".to_string()
    } else {
        "Phases - ".to_string()
    };

    let mut extra_phases: Vec<&Phase> = Vec::new();

    use crate::parse::Phase;
    use crate::target::Target;

    let phases = match encounter.target {
        Target::Siax => match encounter.phases.len() {
            1 => encounter.phases.iter().collect(),
            2 => encounter.phases.iter().collect(),
            4 => {
                extra_phases.push(encounter.phases.last().unwrap());
                encounter.phases.iter().take(3).collect::<Vec<&Phase>>()
            }
            5 => {
                extra_phases.push(encounter.phases.last().unwrap());
                encounter.phases.iter().take(4).collect::<Vec<&Phase>>()
            }
            6 => {
                extra_phases.push(encounter.phases.get(4).unwrap());
                extra_phases.push(encounter.phases.get(5).unwrap());
                encounter.phases.iter().take(4).collect::<Vec<&Phase>>()
            }
            _ => encounter.phases.iter().collect(),
        },
        _ => encounter.phases.iter().collect(),
    };

    for (idx, phase) in phases.iter().enumerate().skip(1) {
        let (sep, sur) = match idx {
            n if n == phases.len() - 1 => {
                if encounter.success {
                    ("", "**")
                } else {
                    ("", "*")
                }
            }
            _ => (" - ", "**"),
        };

        let time = fmt_time2(phase.duration());
        if let Some(name) = &phase.name {
            phase_line.push_str(&format!("{}: {}{}{}{}", name, sur, time, sur, sep));
        } else {
            phase_line.push_str(&format!("{}: {}{}{}{}", idx, sur, time, sur, sep));
        }
    }

    let mut extra = "".to_string();
    if encounter.target == Target::Siax {
        extra.push_str("\nSplits - ");
        for (n, phase) in extra_phases.iter().enumerate() {
            if n > 0 {
                extra.push_str(" - ");
            }

            let time = fmt_time2(phase.duration());
            extra.push_str(&format!("{}: **{}**", n + 1, time));
        }
    }

    format!(
        "{}\n{}{}",
        status_line,
        phase_line,
        if extra_phases.len() > 0 {
            extra
        } else {
            "".to_string()
        }
    )
}

fn status_msg(encounter: &Encounter) -> String {
    let success = if encounter.success {
        "Success"
    } else {
        "Defeat"
    };

    let duration = fmt_time3(encounter.phases[0].duration());
    format!("**{}** in {}", success, duration)
}

fn fmt_time2(time: u64) -> String {
    if time >= 60000 {
        let mins = time / 60000;
        let secs = (time % 60000) / 1000;
        let ten_ms = (time % 1000) / 10;
        format!("{}:{:02}.{:02}", mins, secs, ten_ms)
    } else {
        let secs = time / 1000;
        let ten_ms = (time % 1000) / 10;
        format!("{}.{:02}s", secs, ten_ms)
    }
}

fn fmt_time3(time: u64) -> String {
    if time >= 60000 {
        let mins = time / 60000;
        let secs = (time % 60000) / 1000;
        let ms = time % 1000;
        format!("{}:{:02}.{:03}", mins, secs, ms)
    } else {
        let secs = (time % 60000) / 1000;
        let ms = time % 1000;
        format!("{}.{:03}s", secs, ms)
    }
}
