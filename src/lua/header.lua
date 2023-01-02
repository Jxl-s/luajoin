local LUAJOIN = {}
LUAJOIN.CACHE = {}

LUAJOIN.FILES = {}
LUAJOIN.DIRECTORIES = {}

function LUAJOIN._split(str, sep)
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

function LUAJOIN._parsePath(path, current)
	if not current then current = "" end

	-- Split the paths into parts
	local pathParts = LUAJOIN._split(path, "/")
	local curPathParts = LUAJOIN._split(path, "/")

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

function LUAJOIN._require(path, current)
    path = LUAJOIN._parsePath(path, current)

    if LUAJOIN.CACHE[path] then
        return LUAJOIN.CACHE[path]
    end

    local target = LUAJOIN.FILES[path]
    assert(target, "Could not find the module " .. path)

    LUAJOIN.CACHE[path] = target(function(p)
        return LUAJOIN._require(p, path)
    end)

    return LUAJOIN.CACHE[path]
end