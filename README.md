
## Overview
This proof of concept aims to demonstrate a versatile software platform inspired by LangChain, facilitating the creation of generative AI applications through portable plugins. These plugins, written in various languages, can run on different host platforms with support for cross-environment compatibility and security through the use of [WebAssembly](https://webassembly.org/) and [Extism](https://extism.org/).

## Setup
Create a "secrets.json" file in the root directory with your [SerpApi](https://serpapi.com/) and [OpenAI](https://openai.com/) API keys

```json
{
    "openai_apikey": "<YOUR_OPENAI_APIKEY>",
    "google_apikey": "<YOUR_SERP_APIKEY>"
}
```

## Run
```
python3 chainsocket.py
```
Start chatting with the bot! Type 'end' to exit the conversation

## Modify a Plug-in
- cd into the plugin directory (e.g. cd plugins/self-ask)
- modify the lib.rs as desired
```
cargo build --release --target wasm32-unknown-unknown
```
- copy the the .wasm in "./target/wasm32-unknown-unknown/release to the top level of the plugins directory

## Create a new Plug-in 
Follow the instructions at https://extism.org/docs/category/write-a-plug-in

> Each plug-in must implement a "call" function


