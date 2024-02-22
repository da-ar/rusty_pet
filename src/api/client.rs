use crate::{config, Context};
use log::debug;
use reqwest::StatusCode;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct LoginResp {
    pub data: Data,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    pub user: User,
    pub token: String,
}

#[derive(Deserialize, Debug)]
pub struct User {
    pub id: u32,
    pub email_address: String,
    pub first_name: String,
    pub last_name: String,
    pub country_id: u32,
    pub language_id: u32,
    pub marketing_opt_in: bool,
    pub terms_accepted: String,
    pub weight_units: u32,
    pub time_format: u32,
    pub version: u32,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Client<'a> {
    pub client: reqwest::Client,
    pub ctx: &'a Context,
}

impl Client<'_> {
    pub fn new(ctx: &Context) -> Self {
        Client {
            client: reqwest::Client::new(),
            ctx,
        }
    }

    pub async fn login(
        &self,
        username: &String,
        password: &String,
    ) -> Result<LoginResp, reqwest::Error> {
        let uuid: String = "a1b96664-399d-4c2f-8eaa-b6b5e47c6f31".to_string();
        let post_url: String = self.ctx.config.api.surepy_url.to_string() + "/auth/login";

        debug!("Posting to: {}", post_url);

        let mut map = HashMap::new();
        map.insert("email_address", username);
        map.insert("password", password);
        map.insert("device_id", &uuid);

        debug!("Body to post: {:?}", map);

        let resp = self
            .client
            .post(post_url)
            .header("Host", "app.api.surehub.io")
            .header("Accept-Encoding", "gzip, deflate, br")
            .header("Content-Type", "application/json")
            .header("Accept", "*/*")
            .header("User-Agent", "RustyPet")
            .header("Connection", "keep-alive")
            .header("X-Device-Id", &uuid)
            .json(&map)
            .send()
            .await?;

        debug!("Response Status: {:?}", resp.status());

        if resp.status() == StatusCode::OK {
            let text = resp.text().await?;
            debug!("Response Text: {}", &text);
            let login_resp: LoginResp = serde_json::from_str(&text).unwrap();

            return Ok(login_resp);
        }

        return Err(resp.error_for_status().err().unwrap());
    }
}
