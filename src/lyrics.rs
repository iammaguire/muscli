use select::document::Document;
use select::predicate::Class;

#[derive(Debug, Deserialize)]
struct SearchResult {
    url: String
}

#[derive(Debug, Deserialize)]
struct Hit {
    result: SearchResult
}

#[derive(Debug, Deserialize)]
struct Response {
    hits: Vec<Hit>
}

#[derive(Debug, Deserialize)]
struct Meta {
    status: u32
}

#[derive(Debug, Deserialize)]
struct GeniusResponse {
    meta: Meta,
    response: Response
}

pub struct LyricsGrabber {

}

impl LyricsGrabber {
    pub fn grab_lyrics(artist: String, song_name: String, access_token: &str) -> Option<String> {
        let client = reqwest::Client::new();
        let url = format!("https://api.genius.com/search?q={}%20{}&access_token={}", artist, song_name, access_token);
        match client.get(&url).send().unwrap().json::<GeniusResponse>() {
            Ok(response) => {
                match response.meta.status { 
                    200 => {
                        let lyrics_url = &response.response.hits[0].result.url;
                        let resp = reqwest::get(lyrics_url).unwrap();
                        let document = Document::from_read(resp).unwrap();
                        let mut lyrics = String::new();
                        
                        for n in document.find(Class("lyrics")) {
                            lyrics.push_str(&n.text());
                        }
                        Some(lyrics)
                    }
                    _ => None
                }
            }
            Err(_) => None
        }
    }
}