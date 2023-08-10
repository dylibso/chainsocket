import sys
import pathlib
import hashlib
import json
import os
from dataclasses import dataclass
from dataclasses_json import dataclass_json

import extism
from extism import Function, host_fn, ValType, Plugin, set_log_file

@dataclass_json
@dataclass
class PluginMetadata:
    name: str
    version: str
    entry: str
    description: str

class PluginManifest:
    path: str
    module_data: bytes
    metadata: PluginMetadata
    config: dict

    def __init__(self, filename: str, config) -> None:
        abs_path = os.path.dirname(__file__)
        rel_path = "plugins"
        self.path = os.path.join(abs_path, rel_path, filename)
        self.module_data = open(self.path, "rb").read()

        hash = hashlib.sha256(self.module_data).hexdigest()
        self.config = { "wasm": [{"data": self.module_data, "hash": hash}], "memory": {"max": 5}, "allowed_hosts": ["api.openai.com", "serpapi.com"] }
        
        if(config):
            self.config["config"] = config
        
class ChainSocketPlugin:
    manifest: PluginManifest

    def __init__(self, plugin_name: str, hostfuncs, config) -> None:
        self.manifest = PluginManifest(plugin_name, config)
        self.ctx = extism.Context()
        
        self.plugin = self.ctx.plugin(self.manifest.config, wasi=True, functions=hostfuncs)

    def call(self, func: str, data) -> any:
        return self.plugin.call(func, data)
    
    def execute(self, input: str):
        json_data = json.dumps(input)
        response = self.call("call", json_data)
        return response

    def free(self):
        self.ctx.free()

@dataclass_json
@dataclass
class Tool(ChainSocketPlugin):
    name: str
    description: str
    plugin_name: str 
    
    def load(self, hostfuncs, secrets):
        config = {
            'google_apikey': secrets.google_apikey,
            'name': self.name
        }
        super().__init__(self.plugin_name, hostfuncs, config)
    
@dataclass_json
@dataclass
class Llm(ChainSocketPlugin):
    name: str
    description: str
    plugin_name: str 
    
    def load(self, hostfuncs, secrets):
        config = {
            'openai_apikey': secrets.openai_apikey,
            'name': self.name
        }

        super().__init__(self.plugin_name, hostfuncs, config)

@dataclass_json
@dataclass
class Secrets:
    openai_apikey: str
    google_apikey: str

@dataclass_json
@dataclass
class Agent(ChainSocketPlugin):
    name: str
    description: str
    plugin_name: str
    prompt: str
    tools: list[str]
    llms: list[str]

    def load(self, hostfuncs, secrets):
        # set the plugin configuration with the contents read from the app.json
        config = {
            'prompt': self.prompt,
            'openai_apikey': secrets.openai_apikey,
            'llm_name': self.llms[0],
            'name': self.name
        }
        super().__init__(self.plugin_name, hostfuncs, config)

@dataclass_json
@dataclass
class App:
    tools: list[Tool]
    agents: list[Agent]
    llms: list[Llm]
    entry: str
    
    def get_plugin(self, type, name):
        entry = [ plugin for plugin in self.registry if isinstance(plugin, type) and plugin.name == name ]
        if not entry:
            return None
        else:
            return entry[0]
        
    def call_plugin(self, plugin_type, req):
        plugin = app.get_plugin(plugin_type, req["name"])

        if plugin is not None:
            rep = plugin.execute(req)
            data = { 'output': rep.decode('utf-8'), }
        else:
            data = { 'output': "None", }

        return json.dumps(data)

    def load(self, hostfuncs):

        with open("secrets.json", "r") as f:
            secrets = Secrets.schema().loads(f.read())

        self.registry = []

        for tool in self.tools:
            try:
                tool.load([], secrets)
                self.registry.append(tool)
            except Exception as e:
                print("unable to locate/load plugin: {} for tool: {} exception: {}".format(tool.plugin_name, tool.name, e))

        for agent in self.agents:
            try:
                agent.load(hostfuncs, secrets)
                self.registry.append(agent)
            except Exception as e:
                print("unable to locate/load plugin: {} for agent: {} exception: {}".format(agent.plugin_name, agent.name, e))

        for llm in self.llms:
            try:
                llm.load(hostfuncs, secrets)
                self.registry.append(llm)
            except Exception as e:
                print("unable to locate/load plugin: {} for agent: {} exception: {}".format(llm.plugin_name, llm.name, e))

# define the Extism Host Functions. These will be provided as exports to the plugins
@host_fn
def call_agent(plugin, input_, output, a_string):
    req = json.loads(plugin.input_string(input_[0]))
    rep = app.call_plugin(Agent, req)
    plugin.return_string(output[0], rep)

@host_fn
def call_tool(plugin, input_, output, a_string):
    req = json.loads(plugin.input_string(input_[0]))
    rep = app.call_plugin(Tool, req)
    plugin.return_string(output[0], rep)

@host_fn
def call_llm(plugin, input_, output, a_string):
    req = json.loads(plugin.input_string(input_[0]))
    rep = app.call_plugin(Llm, req)
    plugin.return_string(output[0], rep)

with open("app.json", "r") as f:
    app = App.schema().loads(f.read())

hostfuncs = [
    Function(
        "call_tool",
        [ValType.I64],
        [ValType.I64],
        call_tool,
        None
    ),
    Function(
        "call_llm",
        [ValType.I64],
        [ValType.I64],
        call_llm,
        None
    ),
    Function(
        "call_agent",
        [ValType.I64],
        [ValType.I64],
        call_agent,
        None
    )
]

app.load(hostfuncs)

def main(args):

    if app.entry is None:
        print("no entry agent found")
        sys.exit(0)

    set_log_file('chain.out', level='debug')

    while True:
        print("You > ", end='')
        human_input = input()
        if human_input == 'end':
            break

        agent = app.get_plugin(Agent, app.entry)
        data = {
            'name': agent.name,
            'input': human_input
        }
    
        rep = agent.execute(data)
        print("Agent: ", rep)

if  __name__ == "__main__":
    main(sys.argv)