use rusqlite::{params, Connection, Result};
use log::error;
use uuid::Uuid;
use std::str::FromStr;
// use std::error::Error;
use crate::error::Error;
use crate::tak::player::Player;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use log::info;

#[derive(Debug, Clone)]
pub struct Database{
    path: String,
    is_setup: bool,
}

impl Database{
    pub fn new() -> Self{
        let path = String::from("playtak_data.db");
        if let Err(err) = Database::set_up(path.clone()){
            error!("Error setting up database: {}", err);
            return Database{
                path,
                is_setup: false,
            }
        }
        Database{
            path,
            is_setup: true,
        }
    }

    pub fn set_up(path: String) -> Result<()>{
        let db = Connection::open(path)?;
        db.execute("CREATE TABLE if not exists players (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            uuid VARCHAR UNIQUE,
            name VARCHAR UNIQUE,
            password VARCHAR,
            email VARCHAR UNIQUE
        )", params![])?;

        //TODO set up game table
        Ok(())
    }

    // fn update_setup(&mut self, val: bool) {
    //     self.is_setup = val;
    // }
    
    pub fn get_user(&self, user_name: &str) -> std::result::Result<Player, Error> {
        if !self.is_setup {
            Err(Error::System(String::from("Database not yet setup!")))
        } else {
            let db = Connection::open(&self.path).unwrap();
            let mut players_with_id = db.prepare("SELECT uuid, name, password, email FROM players WHERE name LIKE (?1)").unwrap();

            let mut players = players_with_id.query_map(params![user_name], |row| {
                let uuid_string: String = row.get(0)?; 
                Ok(
                    Player::existing(Uuid::from_str(&uuid_string).unwrap(),
                     row.get(1)?, row.get(2)?, row.get(3)?)
                )
            })?;

            if let Some(player) = players.next(){
                Ok(player?)
            } else {
                Err(Error::System(String::from("Could not find player in database")))
            }
        }
    }

    pub fn write_player(&self, player: Player) -> std::result::Result<bool,Error> {
        if !self.is_setup {
            Err(Error::System(String::from("Database not yet setup!")))
        } else {
            let db = Connection::open(&self.path).unwrap();

            let mut insert_player = db.prepare("INSERT INTO players 
                (uuid, name, password, email) VALUES
                (?1, ?2, ?3, ?4)").unwrap();

            if let Ok(hashed_pass) = Player::hash_password(player.password){

                let res = insert_player.execute(params![player.id.to_string(), player.name, hashed_pass, player.email])?;
                
                Ok(res > 0)
            } else {
                Err(Error::System(String::from("error hashing password")))
            }
        }
    }

}
// Connection::open(path: P)