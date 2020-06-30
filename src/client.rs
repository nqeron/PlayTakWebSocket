use uuid::Uuid;

#[derive(Clone, Copy, Default)]
pub struct Client {
    pub id: Uuid,
}

impl Client{
    pub fn new() -> Self {
        Client{
            id: Uuid::new_v4()
        }
    }
}