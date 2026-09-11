//! Local WebSocket transport for clients of the shared rimv runtime.
use crate::EngineRuntime;
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use futures_util::{SinkExt, StreamExt};
use std::{net::SocketAddr, sync::Arc, thread};

#[derive(Clone)]
struct Bridge {
    runtime: EngineRuntime,
}
pub fn attach_local_websocket(runtime: EngineRuntime, port: u16) -> Result<(), String> {
    thread::Builder::new()
        .name("rimv-websocket".into())
        .spawn(move || {
            let Ok(async_runtime) = tokio::runtime::Runtime::new() else {
                return;
            };
            async_runtime.block_on(async move {
                let app = Router::new()
                    .route("/", get(upgrade))
                    .with_state(Arc::new(Bridge { runtime }));
                if let Ok(listener) =
                    tokio::net::TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], port))).await
                {
                    let _ = axum::serve(listener, app).await;
                }
            });
        })
        .map_err(|error| error.to_string())?;
    Ok(())
}
async fn upgrade(ws: WebSocketUpgrade, State(bridge): State<Arc<Bridge>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| client(socket, bridge))
}
async fn client(socket: WebSocket, bridge: Arc<Bridge>) {
    let Ok(subscription) = bridge.runtime.subscribe() else {
        return;
    };
    let (mut output, mut input) = socket.split();
    let (events, mut incoming) = tokio::sync::mpsc::channel::<String>(64);
    let producer = events.clone();
    tokio::task::spawn_blocking(move || {
        while let Ok(event) = subscription.recv() {
            if let Ok(json) = serde_json::to_string(&event)
                && producer.blocking_send(json).is_err()
            {
                break;
            }
        }
    });
    loop {
        tokio::select! {Some(event)=incoming.recv()=>{if output.send(Message::Text(event.into())).await.is_err(){break}},Some(Ok(Message::Text(command)))=input.next()=>{if let Ok(command)=serde_json::from_str(&command){let runtime=bridge.runtime.clone();let _=tokio::task::spawn_blocking(move||runtime.send(command)).await;}},else=>break}
    }
}
