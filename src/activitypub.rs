use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    #[serde(rename = "@context")]
    pub context: String,
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    #[serde(rename = "preferredUsername")]
    pub preferred_username: String,
    pub inbox: String,
    pub outbox: String,
}

// TODO: should note have context
#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub content: String,
    #[serde(rename = "attributedTo")]
    pub attributed_to: String,
    pub to: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateActivity {
    #[serde(rename = "@context")]
    pub context: String,
    pub id: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub actor: String,
    pub object: Note,
}

pub fn create_actor(username: &str, domain: &str) -> Person {
    let user_id = format!("http://{}/users/{}", domain, username);
    let inbox_url = format!("{}/inbox", user_id);
    let outbox_url = format!("{}/outbox", user_id);

    Person {
        context: "https://www.w3.org/ns/activitystreams".to_string(),
        id: user_id,
        type_field: "Person".to_string(),
        preferred_username: username.to_string(),
        inbox: inbox_url,
        outbox: outbox_url,
    }
}
