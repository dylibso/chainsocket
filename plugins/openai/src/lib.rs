#![no_main]

use std::str::from_utf8;
use extism_pdk::*;
use serde::{Serialize, Deserialize};
use serde_json::json;


#[derive(Deserialize)]
struct ChatMessage {
  content: String,
}

#[derive(Deserialize)]
struct ChatChoice {
  message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatResult {
  choices: Vec<ChatChoice>,
}

#[derive(Deserialize, Debug, Clone)]
struct LLMReq {
  name: String,
  systemprompt: String,
  inputprompt: String,
  stop: Vec<String>,
}

#[plugin_fn]
pub unsafe fn call<'a>(Json(input): Json<LLMReq>) -> FnResult<String> {

  let api_key = config::get("openai_apikey").expect("Could not find config key 'openai_apikey'");
  let name = config::get("name").expect("Could not find config key 'name'");

  info!("LLM {}: Received Request {:#?}", name, input);

  let req = HttpRequest::new("https://api.openai.com/v1/chat/completions")
      .with_header("Authorization", format!("Bearer {}", api_key))
      .with_header("Content-Type", "application/json")
      .with_method("POST");

  let req_body = json!({
    "model": "gpt-3.5-turbo",
    "temperature": 0,
    "stop": input.stop,
    "messages": [
        {   
            "role": "system",
            "content":  input.systemprompt

        },
        {
            "role": "user",
            "content": input.inputprompt,
        }
    ],
  });

  info!("LLM {}: Making Call to OpenAI {}", name, req_body);

  let res = http::request::<String>(&req, Some(req_body.to_string()))?;
  let body = res.body();
  let body = from_utf8(&body)?;
  let body: ChatResult = serde_json::from_str(body)?;

  info!("LLM {}: Received Response from  OpenAI {:#?}", name, body.choices[0].message.content);

  Ok(body.choices[0].message.content.clone())

}


