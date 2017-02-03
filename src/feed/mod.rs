pub mod listeners;

extern crate csv;
extern crate hyper;
extern crate regex;

use std::error::Error;
use std::io::Read;
use self::hyper::client::Client;
use self::regex::Regex;

enum FeedSource {
    Top,
    State(u8),
}

#[derive(Debug)]
pub struct Feed {
    pub id:        i32,
    pub name:      String,
    pub listeners: u32,
    pub alert:     Option<String>,
}

impl PartialEq for Feed {
    fn eq(&self, other: &Feed) -> bool {
        self.id == other.id
    }
}

fn parse(html: &str, source: FeedSource) -> Result<Vec<Feed>, Box<Error>> {
    lazy_static! {
        static ref TOP: Regex =
            Regex::new(
                r#"(?s)<td class="c m">(?P<listeners>\d+)</td>.+?/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>(?:<br /><br />.<div class="messageBox">(?P<alert>.+?)</div>)?"#)
                .unwrap();

        static ref STATE: Regex =
            Regex::new(
                r#"(?s)w1p">.+?<a href="/listen/feed/(?P<id>\d+)">(?P<name>.+?)</a>.+?(?:bold">(?P<alert>.+?)</font>.+?)?<td class="c m">(?P<listeners>\d+)</'td>"#)
                .unwrap();
    }

    let regex = match source {
        FeedSource::Top      => &*TOP,
        FeedSource::State(_) => &*STATE,
    };

    let mut feeds = Vec::new();

    for cap in regex.captures_iter(&html) {
        feeds.push(
            Feed {
                id:        cap["id"].parse()?,
                name:      cap["name"].to_string(),
                listeners: cap["listeners"].parse()?,
                alert:     cap.name("alert").map(|s| s.as_str().to_string()),
            }
        );
    }

    Ok(feeds)
}

fn download_feed_data(client: &Client, source: FeedSource) -> Result<String, Box<Error>> {
    let url = match source {
        FeedSource::Top => "http://broadcastify.com/listen/top".to_string(),
        FeedSource::State(id) => format!("http://www.broadcastify.com/listen/stid/{}", id),
    };

    let mut resp = client.get(&url).send()?;

    let mut body = String::new();
    resp.read_to_string(&mut body)?;

    Ok(body)
}

pub fn get_latest(state_feed: Option<u8>) -> Result<Vec<Feed>, Box<Error>> {
    use self::FeedSource::*;
    
    let client = Client::new();
    let mut feeds = parse(&download_feed_data(&client, Top)?, Top)?;

    if let Some(id) = state_feed {
        feeds.extend(parse(&download_feed_data(&client, State(id))?, State(id))?);
        
        // Remove any state feeds that show up in the top 50 list
        feeds.sort_by_key(|f| f.id);
        feeds.dedup();
    }

    Ok(feeds)
}