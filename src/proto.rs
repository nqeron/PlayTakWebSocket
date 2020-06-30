use serde::{Deserialize, Serialize};
use uuid::Uuid;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "camelCase")]
pub enum Input {
    #[serde(rename = "join")]
    Join(JoinInput),
    #[serde(rename = "post")]
    Post(PostInput),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinInput {
    pub name: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostInput {
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct InputParcel{
    pub client_id: Uuid,
    pub input: Input,
}

impl InputParcel{
    pub fn new(client_id: Uuid, input: Input) -> Self{
        InputParcel{
            client_id,
            input,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Output {
    #[serde(rename = "error")]
    Error(OutputError),
    #[serde(rename = "alive")]
    Alive,
    #[serde(rename = "joined")]
    Joined(JoinedOutput),
    #[serde(rename = "message")]
    Message(MessageOutput),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageOutput{
    pub message: String,
}

impl MessageOutput{
    pub fn new(message: String) -> Self{
        MessageOutput{
            message
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinedOutput{
    pub success: bool,
}

impl JoinedOutput{
    pub fn new(success: bool) -> Self {
        JoinedOutput{
            success
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "code")]
pub enum OutputError {
    #[serde(rename = "name-taken")]
    NameTaken,
    #[serde(rename = "invalid-name")]
    InvalidName,
    #[serde(rename = "not-joined")]
    NotJoined,
    #[serde(rename = "invalid-message-body")]
    InvalidMessageBody,
    #[serde(rename = "invalid-password")]
    InvalidPassword,
}

#[derive(Debug, Clone)]
pub struct OutputParcel {
    pub client_id: Uuid,
    pub output: Output,
}

impl OutputParcel {
    pub fn new(client_id: Uuid, output: Output) -> Self {
        OutputParcel { client_id, output }
    }
}