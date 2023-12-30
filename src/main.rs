use std::collections::HashSet;

use clap::Parser;
use reqwest::{header, Client};
use serde::Deserialize;
use terminal_hyperlink::Hyperlink;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameSort {
    token: String,
    context_country_region_id: usize,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageContext {
    page_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameSortsResponse {
    sorts: Vec<GameSort>,
    page_context: PageContext,
}

#[derive(Debug, Hash, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameSearchResponse {
    name: String,
    place_id: u64,
}

impl Eq for GameSearchResponse {}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GamesSearchResponse {
    games: Vec<GameSearchResponse>,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// The maximum amount of games you want to show up
    #[arg(short, long)]
    limit: Option<usize>,

    /// The country region id you want to use
    #[arg(short, long)]
    region_id: Option<usize>,
}

async fn get_games_list(
    client: &Client,
    roblosecurity: &str,
    sort_token: &str,
    page_id: &str,
    region_id: usize,
) -> Result<GamesSearchResponse, reqwest::Error> {
    client
        .get(format!("https://games.roblox.com/v1/games/list?sortToken={}&startRows=0&maxRows=32&hasMoreRows=true&sortPosition=5&contextCountryRegionId={}&pageContext.pageId={}", sort_token, region_id, page_id))
        .header(header::COOKIE, format!(".ROBLOSECURITY={}", roblosecurity))
        .send()
        .await?
        .json::<GamesSearchResponse>()
        .await
}

async fn get_game_sorts(
    client: &Client,
    roblosecurity: &str,
) -> Result<GameSortsResponse, reqwest::Error> {
    client
        .get("https://games.roblox.com/v1/games/sorts?gameSortsContext=GamesDefaultSorts")
        .header(header::COOKIE, format!(".ROBLOSECURITY={}", roblosecurity))
        .send()
        .await?
        .json::<GameSortsResponse>()
        .await
}

fn log_game(game: &GameSearchResponse) {
    if supports_hyperlinks::on(supports_hyperlinks::Stream::Stdout) {
        println!(
            "{}",
            game.name
                .hyperlink(format!("https://www.roblox.com/games/{}/", game.place_id))
        );
    } else {
        println!("{} > {}", game.name, game.place_id);
    }
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    dotenvy::dotenv().ok();

    let args = Cli::parse();
    let has_limit = args.limit.is_some();
    let limit = args.limit.unwrap_or(0);

    let client = Client::new();
    let roblosecurity = std::env::var("ROBLOSECURITY").unwrap();
    let mut games = HashSet::new();

    let game_sorts = get_game_sorts(&client, &roblosecurity)
        .await
        .expect("Error fetching game sorts");

    let sponsored_sort = game_sorts
        .sorts
        .iter()
        .find(|sort| sort.name == "Sponsored")
        .expect("Error fetching sponsored game sort");

    while if has_limit { games.len() < limit } else { true } {
        let games_list = get_games_list(
            &client,
            &roblosecurity,
            &sponsored_sort.token,
            &game_sorts.page_context.page_id,
            args.region_id
                .unwrap_or(sponsored_sort.context_country_region_id),
        )
        .await
        .expect("Error fetching games");

        for game in games_list.games {
            if !games.contains(&game) {
                log_game(&game);
            }

            games.insert(game);
        }
    }

    Ok(())
}
