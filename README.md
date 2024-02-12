# luajoin

A program (now abandoned) that allows multiple lua files to be joined together into a single bundle.
Previously used to learn more on Rust, and to develop single-file scripts, I am but no longer doing that anymore, so I am making this repository public.

JSON files are also supported.

# Installation

Download the binary from the releases page, and
place it in your path.

# Lua Usage

Each file is structured as modules. You can use
relative and absolute paths to import
modules.

## Features
- JSON and Lua module imports
- Supports relative and absolute paths
- Supports 'require(script.Parent. [...])' syntax
- Can use luau types, will be removed on compilation

Used nicely with Roblox LSP

## Example

file: `main.lua`

```lua
local text = _require("text") -- could also be ./text, since it's in the same dir
print(module:getText())
```

file: `text.lua`

```lua
local text = {}
function text:getText()
    return "Hello, World!"
end

return text
```

JSON files will automatically be converted to lua
tables.

# CLI Usage

## Creating a project

This will prompt you for the project's configuration,
and create the appropriate file and folders.

```
luajoin init
```

## Development

This will watch for file changes in your source
folder and automatically rebuild the bundle. It is a
really fast process, as files are cached.

```
luajoin serve
```

## Deployment

A longer process, as optimizations are applied to the
bundle. This is the command you should use when
deploying your project.

```
luajoin build
```
