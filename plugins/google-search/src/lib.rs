#![no_main]

use std::str::from_utf8;
use extism_pdk::*;
use serde::{Serialize, Deserialize};
use serde_json::json;
use serde_json::Value;


#[derive(Serialize)]
struct PluginMetadata {
  name: String,
  version: String,
  entry: String,
  description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct  ActionRep {
    output: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ToolReq {
  name: String,
  input: String,
}

#[plugin_fn]
pub unsafe fn call<'a>(Json(input): Json<ToolReq>) -> FnResult<String> {
    info!("plugin (google_search) function called");  
    let api_key = config::get("google_apikey").expect("Could not find config key 'google_apikey'");
    let name = config::get("name").expect("Could not find config key 'name'");

    let req = HttpRequest::new("https://serpapi.com/search")
        .with_header("Content-Type", "application/json")
        .with_method("GET");

    info!("TOOL {}: Request recevied: {:#?}", name, input.input);

    let req_body = json!({
        "engine": "google",
        "q": input.input,
        "google_domain": "google.com",
        "gl": "us",
        "hl": "en",
        "api_key": api_key,
    });

    info!("TOOL {}: Making request to Google: {:#?}", name, req_body);

    let res = http::request::<String>(&req, Some(req_body.to_string()))?;
    let body = res.body();
    let body = from_utf8(&body)?;

    let res: Value = serde_json::from_str(body)?; 

    info!("TOOL {}: Received results: {}", name, res);

    let toret: Option<&str>;

    if res.get("answer_box").and_then(|box_val| box_val.get("answer")).is_some() {
        toret = res["answer_box"]["answer"].as_str();
    } else if res.get("answer_box").and_then(|box_val| box_val.get("snippet")).is_some() {
        toret = res["answer_box"]["snippet"].as_str();
    } else if res.get("answer_box")
        .and_then(|box_val| box_val.get("snippet_highlighted_words"))
        .and_then(|highlighted_words| highlighted_words.get(0))
        .is_some()
    {
        toret = res["answer_box"]["snippet_highlighted_words"][0].as_str();
    } else if let Some(organic_results) = res.get("organic_results") {
        if let Some(first_result) = organic_results.get(0) {
            toret = first_result.get("snippet").and_then(|snippet| snippet.as_str());
        } else {
            toret = None;
        }
    } else {
        toret = None;
    }

    Ok(toret.unwrap().to_string())

}








