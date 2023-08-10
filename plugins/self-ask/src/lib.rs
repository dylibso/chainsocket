#![no_main]

use std::str::from_utf8;
use extism_pdk::*;
use serde::{Serialize, Deserialize};
use serde_json::json;
use serde_json::Value;


#[derive(Serialize, Deserialize, Debug, Clone)]
struct LLMReq {
  name: String,
  systemprompt: String,
  inputprompt: String,
  stop: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ToolReq {
  name: String,
  input: String,
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
    fn call_tool(req: Json<ToolReq>) -> Json<ActionRep>;
}

#[host_fn]
extern "ExtismHost" {
    fn call_llm(req: Json<LLMReq>) -> Json<ActionRep>;
}

fn execute_tool(query: &str) -> FnResult<String> {
  let req = ToolReq { 
    name: "google_search".to_string(), 
    input: query.to_string(),
  };

  let Json(rep) = unsafe { call_tool(Json(req))? };
  info!("TOOL RESULTS: {:#?}", rep);

  Ok(rep.output)

}

#[plugin_fn]
pub unsafe fn call<'a>(Json(input): Json<AgentReq>) -> FnResult<String> {
  let llm_name = config::get("llm_name").expect("Could not find config key 'llm_name'");
  let name = config::get("name").expect("Could not find config key 'name'");

  let intermediate = "\nIntermediate answer:";
  let followup = "Follow up:";
  let finalans = "So the final answer is:";

  let mut sysprompt = get_prompt()[0].to_string();

  let mut query = format!("Question: {}\n{}", input.input, "Are follow up questions needed here:".to_string());

  // build up request to the configured LLM
  let mut req = LLMReq {
    name: llm_name.clone(),
    systemprompt: sysprompt.clone(),
    inputprompt: query.clone(),
    stop: vec![format!("{}", intermediate)],
  };

  // call the llm
  info!("AGENT {}: Calling LLM : {:#?}", name, req);
  let Json(mut rep) = unsafe { call_llm(Json(req.clone()))? };
  info!("AGENT {}: Response from LLM : {:#?}", name, rep);

  req.systemprompt.push_str(&format!("{}", &query));
  let mut ret_text = rep.output;

  while get_last_line(&ret_text).contains(followup) {

    req.systemprompt.push_str(&format!("{}", &ret_text));

    let extracted_question = extract_question(&ret_text);
    info!("AGENT {}: Calling Tool with extracted question : {}", name, extracted_question);
    let external_answer = execute_tool(&extracted_question);
    info!("AGENT {}: Response from Tool : {:#?}", name, external_answer);

    if let Ok(external_answer) = external_answer {
        info!("AGENT {}: Received Answer from Tool", name);
        req.inputprompt = format!("{} {}.", intermediate, &external_answer);
        req.stop = vec![intermediate.to_string()];
        info!("AGENT {}: Calling LLM : {:#?}", name, req);
        Json(rep) = unsafe { call_llm(Json(req.clone()))? };
        info!("AGENT {}: Response from LLM : {:#?}", name, rep);
        ret_text = rep.output;
    } else {
        // We only get here in the very rare case that Google returns no answer.
        info!("AGENT {}: Received NO Answer from Tool", name);
        req.systemprompt.push_str(&format!("{}", intermediate));
        req.stop = vec![format!("\n{}", followup), finalans.to_string()];
        info!("AGENT {}: Calling LLM : {:#?}", name, req);
        let Json(gpt_answer) = unsafe { call_llm(Json(req.clone()))? };
        info!("AGENT {}: Response from LLM : {:#?}", name, gpt_answer);
        req.systemprompt.push_str(&format!("{:#?}", gpt_answer));
    }
  }

  if !ret_text.contains(finalans) {
      info!("AGENT {}: Couldn't conclude a final answer", name);
      req.systemprompt.push_str(&format!("{}", finalans));
      req.stop = vec!["\n".to_string()];
      info!("AGENT {}: Calling LLM : {:#?}", name, req);
      Json(rep) = unsafe { call_llm(Json(req.clone()))? };
      info!("AGENT {}: Response from LLM : {:#?}", name, rep);
      ret_text = rep.output;
  }

  let clean = extract_answer(&ret_text);
  Ok(clean.to_string())
}

fn extract_answer(generated: &str) -> String {
  let last_line = if !generated.contains('\n') {
      generated.to_string()
  } else {
      generated.lines().last().unwrap().to_string()
  };

  let after_colon = if !last_line.contains(':') {
      last_line.to_string()
  } else {
      last_line.split(':').last().unwrap().to_string()
  };

  let after_colon = if after_colon.starts_with(' ') {
      after_colon[1..].to_string()
  } else {
      after_colon
  };

  let after_colon = if after_colon.ends_with('.') {
      after_colon[..after_colon.len() - 1].to_string()
  } else {
      after_colon
  };

  after_colon
}

fn extract_question(generated: &str) -> String {
  let last_line = if !generated.contains('\n') {
      generated.to_string()
  } else {
      generated.lines().last().unwrap().to_string()
  };

  assert!(last_line.contains("Follow up:"), "we probably should never get here...{}", generated);

  let after_colon = if !last_line.contains(':') {
      last_line.to_string()
  } else {
      last_line.split(':').last().unwrap().to_string()
  };

  let after_colon = if after_colon.starts_with(' ') {
      after_colon[1..].to_string()
  } else {
      after_colon
  };

  assert_eq!(after_colon.chars().last(), Some('?'), "we probably should never get here...{}", generated);

  after_colon
}

fn get_last_line(generated: &str) -> String {
  if !generated.contains('\n') {
      generated.to_string()
  } else {
      generated.lines().last().unwrap().to_string()
  }
}

fn get_prompt() -> Vec<&'static str> {
  let prompt: Vec<&str> = vec![
        "Question: Who lived longer, Muhammad Ali or Alan Turing?\n
        Are follow up questions needed here: Yes.
        Follow up: How old was Muhammad Ali when he died?
        Intermediate answer: Muhammad Ali was 74 years old when he died.
        Follow up: How old was Alan Turing when he died?
        Intermediate answer: Alan Turing was 41 years old when he died.
        So the final answer is: Muhammad Ali\n
        Question: When was the founder of craigslist born?\n
        Are follow up questions needed here: Yes.
        Follow up: Who was the founder of craigslist?
        Intermediate answer: Craigslist was founded by Craig Newmark.
        Follow up: When was Craig Newmark born?
        Intermediate answer: Craig Newmark was born on December 6, 1952.
        So the final answer is: December 6, 1952\n
        Question: Who was the maternal grandfather of George Washington?\n
        Are follow up questions needed here: Yes.
        Follow up: Who was the mother of George Washington?
        Intermediate answer: The mother of George Washington was Mary Ball Washington.
        Follow up: Who was the father of Mary Ball Washington?
        Intermediate answer: The father of Mary Ball Washington was Joseph Ball.
        So the final answer is: Joseph Ball\n
        Question: Are both the directors of Jaws and Casino Royale from the same country?\n
        Are follow up questions needed here: Yes.
        Follow up: Who is the director of Jaws?
        Intermediate Answer: The director of Jaws is Steven Spielberg.
        Follow up: Where is Steven Spielberg from?
        Intermediate Answer: The United States.
        Follow up: Who is the director of Casino Royale?
        Intermediate Answer: The director of Casino Royale is Martin Campbell.
        Follow up: Where is Martin Campbell from?
        Intermediate Answer: New Zealand.
        So the final answer is: No\n"
    ];
    return prompt
}

