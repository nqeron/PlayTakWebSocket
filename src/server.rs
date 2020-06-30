use warp::ws::WebSocket;
use warp::Filter;
use futures::{future, StreamExt, TryStreamExt};
use tokio::sync::mpsc;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::broadcast;
use log::{info,error};
use std::sync::Arc;
use crate::client::Client;
use crate::hub::Hub;
use crate::proto::{InputParcel, Input, OutputParcel};
use crate::error::Error;

pub struct ExServer {
    port: u16,
    hub: Arc<Hub>,
}

impl ExServer{
    pub fn new(port: u16) -> Self{
        ExServer{
            port,
            hub: Arc::new(Hub::new()),
        }
    }

    pub async fn run(&self){
        let (input_sender, input_receiver) = mpsc::unbounded_channel::<InputParcel>();

        // let feed = warp::path("test").map(|| {});
        let hub = self.hub.clone();
        let socket = warp::ws()
            .and(warp::any().map(move || input_sender.clone()))
            .and(warp::any().map(move || hub.clone()))
            .map( move |ws : warp::ws::Ws, input_sender: UnboundedSender<InputParcel>, hub: Arc<Hub>| {
            ws.on_upgrade( move |websocket| async move{
                tokio::spawn(Self::process_client(hub, websocket, input_sender));
            })
        });

        let running_hub = self.hub.run(input_receiver);
        let server = warp::serve(socket).run(([127, 0, 0, 1], self.port));
        tokio::select! {
            _ = server => {},
            _ = running_hub => {},
        }
    }

    async fn process_client(hub: Arc<Hub>,websocket : WebSocket, input_sender: UnboundedSender<InputParcel>){
        let (ws_sink, ws_stream) = websocket.split();
        // let (tx, rx) = mpsc::unbounded_channel();
        // tokio::spawn(rx.forward(wsSink));
        let client = Client::new();
        info!("client id: {}", client.id);

        let output_receiver: broadcast::Receiver<OutputParcel>  = hub.subscribe();

        let reading = ws_stream.take_while(|message|{
            future::ready(if let Ok(message) = message {
                message.is_text()
            } else{
                false
            })
        }).map(move |message| {
            match message {
                Err(err) => Err(Error::System(err.to_string())),
                Ok(message) => {
                    let input = serde_json::from_str(message.to_str().unwrap())?;
                    Ok(InputParcel::new(client.id, input))
                }
            }
        })
        .try_for_each(|input_parcel: InputParcel| async{
            //Process input and send to hub. 
            //TODO eventually delegaate some of this to client?
            // let mess = message;
            // let input: Input = serde_json::from_str(mess.to_str().unwrap()).unwrap();
            input_sender.send(input_parcel).unwrap();
            Ok(())
        });

        let (tx, rx) = mpsc::unbounded_channel();
        tokio::spawn(rx.forward(ws_sink));

        let writing = output_receiver
            .into_stream() //grab broadcast output
            .try_filter(|out_parcel: &OutputParcel| {
                future::ready(out_parcel.client_id == client.id)
            }) //filter by client
            .map_ok( |out_parcel: OutputParcel| {
                //parse the data from the parcel
                let data = serde_json::to_string(&out_parcel.output).unwrap();
                warp::ws::Message::text(data)
            }).map_err(|err| Error::System(err.to_string()) )//  Error::System(err.to_string()))
            .try_for_each(|message| async{
                //send message to client
                tx.send(Ok(message)).unwrap();
                Ok(())
            });

        if let Err(err) = tokio::select! {
            result = reading => result,
            result = writing => result,
        } {
           error!("Error: {}", err); 
        }

        hub.on_disconnect(client.id).await;

    }
}