use crate::models::MonitorInfo;
use crate::monitor::get_all_monitors;
use crate::window::{list_visible_windows, move_window_once};
use axum::{
    Router,
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[derive(Deserialize)]
#[serde(tag = "type")]
enum WebCommand {
    #[serde(rename = "get_monitors")]
    GetMonitors,
    #[serde(rename = "get_windows")]
    GetWindows,
    #[serde(rename = "move_window")]
    MoveWindow { hwnd: isize, monitor_idx: usize },
}

#[derive(Serialize)]
#[serde(tag = "type")]
enum WebResponse {
    #[serde(rename = "monitors")]
    Monitors { monitors: Vec<MonitorInfo> },
    #[serde(rename = "windows")]
    Windows {
        windows: Vec<crate::window::ProcessEntry>,
    },
    #[serde(rename = "error")]
    Error { message: String },
    #[serde(rename = "ack")]
    Ack { message: String },
}

pub async fn start_server() {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new().route("/ws", get(ws_handler)).layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 12345));
    println!("Web Bridge listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let response = match serde_json::from_str::<WebCommand>(&text) {
                    Ok(WebCommand::GetMonitors) => {
                        let monitors = get_all_monitors();
                        WebResponse::Monitors { monitors }
                    }
                    Ok(WebCommand::GetWindows) => {
                        let windows = list_visible_windows();
                        WebResponse::Windows { windows }
                    }
                    Ok(WebCommand::MoveWindow {
                        hwnd: hwnd_val,
                        monitor_idx,
                    }) => {
                        let monitors = get_all_monitors();
                        if let Some(mon) = monitors.get(monitor_idx) {
                            let target_rect = mon.rect;

                            tokio::task::spawn_blocking(move || {
                                let hwnd = windows::Win32::Foundation::HWND(hwnd_val as *mut _);
                                move_window_once(hwnd, target_rect.into());
                            });

                            WebResponse::Ack {
                                message: format!(
                                    "Move initiated for HWND {} to Monitor {}",
                                    hwnd_val, monitor_idx
                                ),
                            }
                        } else {
                            WebResponse::Error {
                                message: format!("Monitor index {} not found", monitor_idx),
                            }
                        }
                    }
                    Err(e) => WebResponse::Error {
                        message: format!("Invalid command: {}", e),
                    },
                };

                if let Ok(json) = serde_json::to_string(&response) {
                    if socket.send(Message::Text(json)).await.is_err() {
                        break;
                    }
                }
            }
            Message::Close(_) => break,
            _ => (),
        }
    }
}
