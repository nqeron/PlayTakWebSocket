use tokio::sync::{broadcast, RwLock};
use std::time::Duration;
use crate::model::user::User;
use uuid::Uuid;
use log::info;
use std::collections::HashMap;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::proto::{InputParcel, Input, JoinInput, OutputError, Output, OutputParcel, PostInput, JoinedOutput, MessageOutput};
use regex::Regex;
use tokio::time;

pub struct Hub {
    output_sender: broadcast::Sender<OutputParcel>,
    users: tokio::sync::RwLock<HashMap<Uuid, User>>,
}

const OUTPUT_CHANNEL_SIZE: usize = 16;
const MAX_MESSAGE_BODY_LENGTH: usize = 256;
lazy_static! {
    static ref USER_NAME_REGEX: Regex = Regex::new("[A-Za-z\\s]{4,24}").unwrap();
    static ref GUEST_NAME_REGEX: Regex = Regex::new(r"Guest\d+").unwrap();
}

impl Hub {
    pub fn new() -> Self{
        let (output_sender, _) = broadcast::channel(OUTPUT_CHANNEL_SIZE);
        Hub{
            output_sender,
            users: Default::default(),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OutputParcel> {
        self.output_sender.subscribe()
    }

    pub async fn run(&self, receiver: UnboundedReceiver<InputParcel>){
        // receiver.for_each(|input|{})
        let ticking_alive = self.tick_alive();
        let processing = receiver.for_each(|input_parcel| self.process(input_parcel));
        tokio::select!{
            // _ = ticking_alive => {},
            _ = processing => {},
        }
        //TODO is this right?
    }

    async fn process(&self, input_parcel: InputParcel){
        match input_parcel.input{
            Input::Join(input) => self.process_join(input_parcel.client_id, input).await,
            Input::Post(input) => self.process_post(input_parcel.client_id, input).await,
            _ => unreachable!()
        }
    }

    async fn process_join(&self, client_id: Uuid, input: JoinInput){
        let user_name = input.name.trim();
        let password = input.password.as_str();

        // Validate user name
        if !USER_NAME_REGEX.is_match(user_name) {
            self.send_error(client_id, OutputError::InvalidName);
            return;
        }

        //check to see if the user is already logged in
        if self.users
            .read()
            .await
            .values()
            .any(|user: &User| user.name == user_name){
                self.send_error(client_id, OutputError::NameTaken);
                return;
        }

        //Validate the password
        
        if !self.validate_password(user_name, password){
            self.send_error(client_id, OutputError::InvalidPassword);
            return;
        }

        //add them to the list of users
        let user = User::new(client_id, user_name);
        self.users.write().await.insert(client_id, user);

        self.send_targeted(client_id, Output::Joined(JoinedOutput::new(true)));

    }

    async fn process_post(&self, client_id: Uuid, input: PostInput){

        let user = if let Some(user) = self.users.read().await.get(&client_id){
            user.clone()
        } else{
            self.send_error(client_id, OutputError::NotJoined);
            return
        };

        if input.body.is_empty() || input.body.len() > MAX_MESSAGE_BODY_LENGTH {
            self.send_error(client_id, OutputError::InvalidMessageBody);
            return
        };

        // self.send_ignored(client_id, Output::UserPosted(UserPosted::new()))

        self.send_targeted(client_id, Output::Message(MessageOutput::new(input.body)));
        //TODO do something with the message


    }

    fn send_error(&self, client_id: Uuid, error: OutputError){
        self.send_targeted(client_id, Output::Error(error));
    }

    fn send_targeted(&self, client_id: Uuid, output: Output){
        if self.output_sender.receiver_count() > 0 {
            self.output_sender.send(OutputParcel::new(client_id, output)).unwrap();
        }
    }

    async fn send(&self, output: Output) {
        if self.output_sender.receiver_count() == 0 {
            return;
        }
        self.users.read().await.keys().for_each(|user_id| {
            self.output_sender
                .send(OutputParcel::new(*user_id, output.clone()))
                .unwrap();
        });
    }

    fn validate_password(&self, user_name: &str, password: &str) -> bool{
        if GUEST_NAME_REGEX.is_match(user_name) {
            true
        } else{
            false
        }
        //todo implement this, for the moment only accept guests
    }

    pub async fn on_disconnect(&self, client_id: Uuid){
        if self.users.write().await.remove(&client_id).is_some() {
            //TODO do something when the user is removed?
        }
    }

    async fn tick_alive(&self){
        loop{
            time::delay_for(Duration::from_secs(5)).await;
            self.send(Output::Alive).await;
        }
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}