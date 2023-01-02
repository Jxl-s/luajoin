local __LUAJOIN_CACHE = {}
local __LUAJOIN_FILES = {}
local __LUAJOIN_DIRECTORIES = {}

local function __LUAJOIN_split(str, sep)
	if string.split then
		return string.split(str, sep)
	end

	-- Non roblox
	if sep == nil then
		sep = "%s"
	end

	local t = {}
	for s in string.gmatch(str, "([^"..sep.."]+)") do
		table.insert(t, s)
	end

	return t
end

local function __LUAJOIN_parsePath(path, current)
	if not current then current = "" end

	-- Split the paths into parts
	local pathParts = __LUAJOIN_split(path, "/")
	local curPathParts = __LUAJOIN_split(path, "/")

	-- Determine if it's a relative path to find
	local isRelative = false

	for _, v in pairs(pathParts) do
		if v == ".." or v == "." then
			isRelative = true
			break
		end
	end

	if isRelative then
		local newPath = {}

		-- if it starts with a .. or ., it's relative to current path
		if pathParts[1] == ".." or pathParts[1] == "." then
			-- add every current path
			for _, v in pairs(curPathParts) do
				table.insert(newPath, v)
			end

			if DIRECTORIES[current] then
				table.insert(newPath, "init")
			end
		end

		-- go through the path parts and do whatever operation
		for _, v in pairs(pathParts) do
			if v == ".." then
				table.remove(newPath, #newPath)
				table.remove(newPath, #newPath)
			elseif v == "." then
				table.remove(newPath, #newPath)
			else
				table.insert(newPath, v)
			end
		end

		path = table.concat(newPath, "/")
	end

	return path
end

function __LUAJOIN_require(path, current)
    path = __LUAJOIN_parsePath(path, current)

    if __LUAJOIN_CACHE[path] then
        return __LUAJOIN_CACHE[path]
    end

    local target = __LUAJOIN_FILES[path]
    assert(target, "Could not find the module " .. path)

    __LUAJOIN_CACHE[path] = target(function(p)
        return __LUAJOIN_require(p, path)
    end)

    return __LUAJOIN_CACHE[path]
end