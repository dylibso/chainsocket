#![no_main]

use std::{str::from_utf8, fmt};
use extism_pdk::*;
use serde::{Serialize, Deserialize};
use serde_json::json;

#[derive(Serialize)]
struct PluginMetadata {
  name: String,
  version: String,
  entry: String,
  description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct LLMReq {
  name: String,
  systemprompt: String,
  inputprompt: String,
  stop: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct AgentReq {
  name: String,
  input: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct  ActionRep {
    output: String,
}

#[host_fn]
extern "ExtismHost" {
    fn call_llm(req: Json<LLMReq>) -> Json<ActionRep>;
}

#[host_fn]
extern "ExtismHost" {
    fn call_agent(req: Json<AgentReq>) -> Json<ActionRep>;
}

#[plugin_fn]
pub unsafe fn call<'a>(Json(mut input): Json<AgentReq>) -> FnResult<String> {

  let configprompt = config::get("prompt").expect("Could not find config key 'prompt'");
  let llm = config::get("llm_name").expect("Could not find config key 'llm_name'");
  //let delegate = config::get("delegate").expect("Could not find config key 'delegate'");
  let name = config::get("name").expect("Could not find config key 'name'");

  // input.name = delegate.clone();
  // info!("AGENT {}: Calling Delegate: {:#?}", name, input);
  // let Json(rep) = unsafe { call_agent(Json(input.clone()))? };
  // info!("AGENT {}: Response from Delegate: {:#?}", name, rep);

  let history = match var::get("memory") {
      Ok(Some(bytes)) => String::from_utf8(bytes)?,
      _ => String::from("\nHere is the history of the chat with the human you are assisting\n"), 
  };

  let systemprompt = format!("{} {}.", configprompt.clone(), history.clone());

  let req = LLMReq {
    name: llm.clone(),
    systemprompt: systemprompt,
    inputprompt: input.input.clone(),
    stop: vec![],
  };

  // call the configured LLM plugin via our custom Extism Host Function
  info!("AGENT {}: Calling LLM: {:#?}", name, req);
  let Json(rep) = unsafe { call_llm(Json(req))? };
  info!("AGENT {}: Response from LLM: {:#?}", name, rep);

  let mut systemprompt = history.clone();
  systemprompt.push_str("Human: ");
  systemprompt.push_str(&input.input);
  systemprompt.push_str("\n");
  systemprompt.push_str("Assistant: ");
  systemprompt.push_str(&rep.output);
  set_var!("memory", "{}", systemprompt.as_str())?;

  Ok(rep.output.clone())
}
