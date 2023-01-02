# luajoin

I created this as a way to familiarize myself with
rust. It's a simple program that allows one to join
multiple lua files together into a single bundle.

JSON files are also supported.

# Installation

Download the binary from the releases page, and
place it in your path.

# Lua Usage

Each file is structured as modules. You can use
relative and absolute paths to import
modules.

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
