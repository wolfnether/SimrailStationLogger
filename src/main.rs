use std::collections::HashMap;
use web_time::SystemTime;

use data::*;
use seed::{prelude::*, *};

#[macro_use]
extern crate gloo_console;

mod data;

pub type Result<T> = core::result::Result<T, Box<dyn std::error::Error>>;

fn init(_: Url, order: &mut impl Orders<Msg>) -> Model {
    order.stream(streams::interval(1000, || Msg::Refresh));
    order.send_msg(Msg::LoadServer);
    Model {
        servers: vec![],
        selected_server: "".into(),
        player_on_station: HashMap::new(),
        dark: true,
        filter: "".into(),
    }
}

struct Model {
    servers: Vec<Server>,
    selected_server: String,
    player_on_station: HashMap<String, Vec<(SystemTime, String)>>,
    dark: bool,
    filter: String,
}

enum Msg {
    LoadServer,
    ServerLoaded(Vec<Server>),
    ServerChanged(String),
    Refresh,
    StationLoaded(Vec<Station>),
    StationChanged(String),
    ToggleDark,
}

fn update(msg: Msg, model: &mut Model, orders: &mut impl Orders<Msg>) {
    match msg {
        Msg::LoadServer => {
            orders.perform_cmd(async { Msg::ServerLoaded(get_servers().await.unwrap()) });
        }
        Msg::ServerLoaded(servers) => {
            model.servers = servers;
            orders.send_msg(Msg::ServerChanged(
                model
                    .servers
                    .first()
                    .expect("no server")
                    .server_code
                    .clone(),
            ));
        }
        Msg::ServerChanged(selected_server) => {
            model.player_on_station.clear();
            model.selected_server = selected_server;
            orders.send_msg(Msg::Refresh);
        }
        Msg::Refresh => {
            if !model.selected_server.is_empty() {
                let selected_server = model.selected_server.clone();
                orders.perform_cmd(async {
                    Msg::StationLoaded(get_stations(selected_server).await.unwrap())
                });
            }
        }
        Msg::StationLoaded(stations) => {
            for station in stations {
                if model.player_on_station.get_mut(&station.prefix).is_none() {
                    model
                        .player_on_station
                        .insert(station.prefix.clone(), vec![]);
                }
                let station_log = model.player_on_station.get_mut(&station.prefix).unwrap();

                if let Some(log) = station_log.last() {
                    if let Some(player) = station.dispatched_by.first() {
                        if log.1 != player.steam_id {
                            station_log.push((SystemTime::now(), player.steam_id.clone()))
                        }
                    } else {
                        if log.1 != "BOT" {
                            station_log.push((SystemTime::now(), "BOT".into()))
                        }
                    }
                } else {
                    if let Some(player) = station.dispatched_by.first() {
                        station_log.push((SystemTime::now(), player.steam_id.clone()))
                    } else {
                        station_log.push((SystemTime::now(), "BOT".into()))
                    }
                }
            }
        }
        Msg::ToggleDark => model.dark = !model.dark,
        Msg::StationChanged(filter) => model.filter = filter,
    }
}

async fn get_stations(selected_server: String) -> crate::Result<Vec<Station>> {
    Ok(reqwest::get(format!(
        "https://panel.simrail.eu:8084/stations-open?serverCode={}",
        selected_server
    ))
    .await?
    .json::<StationResponse>()
    .await?
    .data)
}

async fn get_servers() -> crate::Result<Vec<Server>> {
    Ok(reqwest::get("https://panel.simrail.eu:8084/servers-open")
        .await?
        .json::<ServerResponse>()
        .await?
        .data)
}

fn view(model: &Model) -> Node<Msg> {
    div![style!(St::BackgroundColor => if model.dark {"#2b2b2b"} else {"#dfdfdf"},St::Color => if model.dark {"#dfdfdf"} else {"black"}),
        select![
            model
                .servers
                .iter()
                .filter(|s| s.is_active)
                .map(|s| option![attrs!(At::Value=> s.server_code), s.server_name.clone()]),
            input_ev(Ev::Input, Msg::ServerChanged)
        ],
        button![if model.dark {"☀"} else {"☽"}, input_ev(Ev::Click, |_| Msg::ToggleDark),
        ],
        div![
            select![
                option![attrs!(At::Value=> ""), ""],
                model.player_on_station.iter().map(|(station, _)| option![attrs!(At::Value=> station.clone()), station.clone()]),
                input_ev(Ev::Input, Msg::StationChanged)
            ],
            model.player_on_station.iter().filter(|(station,_)| model.filter.is_empty() || &&model.filter == station).enumerate().map(|(i,(station, log))| div![
            style!(St::BackgroundColor => if i % 2 == 0 {if model.dark {"#2b2b2b"} else {"#dfdfdf"}} else {if model.dark {"#3b3b3b"} else {"#cfcfcf"}}),
            p!(station.clone()),
            log.iter().map(|l| p!(
                style!(),
                {
                    let tz = js_sys::Date::new_0().get_timezone_offset() as i64;
                    let d = l.0.duration_since(web_time::UNIX_EPOCH).unwrap().as_secs() as i64 - tz * 60;
                    let s = d % 60;
                    let m_rem = (d - s) / 60;
                    let m = m_rem % 60;
                    let h_rem = (m_rem - m) / 60;
                    let h = h_rem % 24;
                    format!("{h:02}:{m:02}:{s:02}")
                },
                " ",
                if l.1 == "BOT" {a!("BOT")} else{
                    a!(attrs!(At::Href=> format!("https://steamcommunity.com/profiles/{}",l.1.clone()), At::Target => "_blank"),l.1.clone())
                }
            ))
        ])]
    ]
}

pub fn main() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    App::start("app", init, update, view);
}
