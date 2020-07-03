
extern crate bcrypt;

use uuid::Uuid;
use bcrypt::{DEFAULT_COST, hash, verify, BcryptResult};

#[derive(Clone, Default, Debug)]
pub struct Player {
    pub id: Uuid,
    pub name: String,
    pub password: String,
    pub email: String,

    pub is_guest: bool,
    is_mod: bool,
    client_id: Option<Uuid>,

    reset_token: Option<String>,
    // stats: Stats
}

impl Player{

    pub fn existing(id: Uuid, name: String, password: String, email: String) -> Self {
        Player {
            id,
            name,
            password,
            email,
            is_guest: false,
            is_mod: false,
            client_id: None,
            reset_token: None,
        }
    }

    pub fn new(name: &str, password: &str, email: &str, client_id: Uuid, is_guest: bool) -> Self{
        Player {
            id: Uuid::new_v4(),
            name: String::from(name),
            password: String::from(password),
            email: String::from(email),
            is_guest,
            is_mod: false,
            client_id: Some(client_id),
            reset_token: None,
        }
    }

    pub fn hash_password(password: String) -> BcryptResult<String>{
        hash(password, DEFAULT_COST)
    }

    pub fn verify_password(password: String, hash: &str) -> BcryptResult<bool>{
        verify(password, hash)
    }

    pub fn set_client(&mut self, client_id: Uuid){
        self.client_id = Some(client_id);
    }

    pub fn get_client(&self) -> Option<Uuid> {
        self.client_id
    }
}