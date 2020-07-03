use tokio::sync::{broadcast, RwLock};
use std::time::Duration;
use crate::model::user::User;
use crate::tak::player::Player;
use crate::database::Database;
use uuid::Uuid;
use log::{info, error};
use std::collections::HashMap;
use futures::StreamExt;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::proto::{InputParcel, Input, RegisterInput, OutputError, Output, OutputParcel,
     PostInput, JoinedOutput, MessageOutput, SignInInput};
use regex::Regex;
use tokio::time;

pub struct Hub {
    output_sender: broadcast::Sender<OutputParcel>,
    players: tokio::sync::RwLock<HashMap<Uuid, Player>>,
    database: Database,
}

const OUTPUT_CHANNEL_SIZE: usize = 16;
const MAX_MESSAGE_BODY_LENGTH: usize = 256;
lazy_static! {
    static ref USER_NAME_REGEX: Regex = Regex::new("[A-Za-z\\s]{4,24}").unwrap();
    static ref GUEST_NAME_REGEX: Regex = Regex::new(r"Guest\d+").unwrap();
    static ref VALID_EMAIL_REGEX: Regex = Regex::new(r"^[\w!#$%&’*+/=?`{|}~^-]+(?:\.[\w!#$%&’*+/=?`{|}~^-]+)*@(?:[a-zA-Z0-9-]+\.)+[a-zA-Z]{2,6}$").unwrap();
}

impl Hub {
    pub fn new() -> Self{
        let (output_sender, _) = broadcast::channel(OUTPUT_CHANNEL_SIZE);
        Hub{
            output_sender,
            players: Default::default(),
            database: Database::new(),
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
            Input::Register(input) => self.process_register(input_parcel.client_id, input).await,
            Input::SignIn(input) => self.process_sign_in(input_parcel.client_id, input).await,
            Input::Post(input) => self.process_post(input_parcel.client_id, input).await,
            _ => unreachable!()
        }
    }

    async fn process_register(&self, client_id: Uuid, input: RegisterInput){
        let user_name = input.name.trim();
        let password = input.password.as_str();
        let email = input.email.trim();

        // Validate user name
        if !USER_NAME_REGEX.is_match(user_name) {
            self.send_error(client_id, OutputError::InvalidName);
            return;
        }

        // Validate email
        if !VALID_EMAIL_REGEX.is_match(email) {
            self.send_error(client_id, OutputError::InvalidEmail);
            return;
        }

        //check to see if the user name exists in database
        if let Ok(_) = self.database.get_user(user_name) {
            self.send_error(client_id, OutputError::NameTaken);
            return;
        }

        if self.players
            .read()
            .await
            .values()
            .any(|user: &Player| user.name == user_name){
                self.send_error(client_id, OutputError::NameTaken);
                return;
        }

        //Validate the password
        let player = Player::new(user_name, password, email, client_id, true);
        match self.database.write_player(player.clone()) {
            Ok(written) => {
                if !written {
                    self.send_error(client_id, OutputError::FailedWritingPlayer);
                    return;
                }
            },
            Err(err) => {
                error!("Error writing to database: {}", err);
                self.send_error(client_id, OutputError::FailedWritingPlayer);
                return;
            }
        }

        //add them to the list of users
        // let user = User::new(client_id, user_name);
        self.players.write().await.insert(client_id, player);

        self.send_targeted(client_id, Output::Joined(JoinedOutput::new(true)));

    }

    async fn process_post(&self, client_id: Uuid, input: PostInput){

        let user = if let Some(user) = self.players.read().await.get(&client_id){
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

    async fn process_sign_in(&self, client_id: Uuid, input: SignInInput){
        let user_name = input.name.trim();
        let password = input.password;
        
        // Validate user name
        if !USER_NAME_REGEX.is_match(user_name) {
            self.send_error(client_id, OutputError::InvalidName);
            return;
        }

        //if the user is a guest, skip the rest of the validation.
        if GUEST_NAME_REGEX.is_match(user_name) {
            let guest = Player::new(user_name, "", "", client_id, true);
            self.players.write().await.insert(client_id, guest);
            self.send_targeted(client_id, Output::Joined(JoinedOutput::new(true)));
            return;
        }

        let mut player = if let Ok(player) = self.database.get_user(user_name){
            info!("Found player {:?}", player);
            player
        } else {
            self.send_error(client_id, OutputError::PlayerNotFound);
            return;
        };

        if let Ok(is_pass_valid) = Player::verify_password(password, &player.password) {
            if !is_pass_valid {
                self.send_error(client_id, OutputError::InvalidPassword);
                return;
            }
        } else {
            self.send_error(client_id, OutputError::UnableToVerifyPassword);
            return;
        }

        if let Some(_) =  player.get_client() {
            self.send_error(client_id, OutputError::LoginOnOtherClient);
            return;
        }

        if self.players.read().await.values().any(|player: &Player| {
            player.name == user_name
        }) {
            self.send_error(client_id, OutputError::NameTaken);
            return;
        }

        player.set_client(client_id); //set the player's client id
        self.players.write().await.insert(client_id, player);

        self.send_targeted(client_id, Output::Joined(JoinedOutput::new(true)));

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
        self.players.read().await.keys().for_each(|client_id| {
            self.output_sender
                .send(OutputParcel::new(*client_id, output.clone()))
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
        if self.players.write().await.remove(&client_id).is_some() {
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