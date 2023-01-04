repeat task.wait() until game:IsLoaded()

local WEBSOCKET_RETRY_DELAY = 3

local HttpService = game:GetService("HttpService")
local Players = game:GetService("Players")
local ScriptContext = game:GetService("ScriptContext")

local SocketWrapper = {}
SocketWrapper.__index = SocketWrapper

-- Constructor
function SocketWrapper.new(url)
    local self = setmetatable({
        connection = syn.websocket.connect(url),
        closed = false,
        listeners = {}
    }, SocketWrapper)

    -- Listen to close
    self.closeCon = self.connection.OnClose:Connect(function()
        self.closed = true
        self:Destroy()
    end)

    -- Listen to message
    self.messageCon = self.connection.OnMessage:Connect(function(message)
        if self.closed then return end

        local messageJson = HttpService:JSONDecode(message)

        local messageEvent = messageJson[1]
        local messageContent = messageJson[2]

        if self.listeners[messageEvent] then
            -- Dispatch the event
            for _, v in pairs(self.listeners[messageEvent]) do
                v(messageContent)
            end
        end
    end)

    return self
end

function SocketWrapper:Destroy()
    self.closeCon:Disconnect()
    self.messageCon:Disconnect()
end

-- Sends a message
function SocketWrapper:Send(event, content)
    if self.closed then return end

    local messageJson = HttpService:JSONEncode({ event, content })
    return self.connection:Send(messageJson)
end

-- Adds an event listener
function SocketWrapper:on(event, callback)
    if not self.listeners[event] then
        self.listeners[event] = {}
    end

    table.insert(self.listeners[event], callback)
    return { event, callback }
end

while task.wait(WEBSOCKET_RETRY_DELAY) do
    local success, socket = pcall(function()
        return SocketWrapper.new("ws://192.168.1.171:1338")
    end)

    -- Keep reconnecting to the websocket
    if not success then continue end
    socket:Send("connected", Players.LocalPlayer.Name)

    -- Listen to execution event
    socket:on("exec", function(str)
        -- Execute it
        loadstring(str)()
    end)

    -- Listen to errors
    for _, v in pairs(getconnections(ScriptContext.ErrorDetailed)) do
        v:Disable()
    end

    ScriptContext.ErrorDetailed:Connect(function(errMessage, stackTrace, errScript)
        if errScript then return end

        -- Find the line number in the error message
        local messageLines = {}
        for num in errMessage:gmatch(":(%d+):") do
            table.insert(messageLines, tonumber(num))
        end

        -- Find the error message content
        local messageContent = errMessage
        if #messageLines > 0 then
            messageContent = errMessage:match(messageLines[#messageLines] .. ": (.+)$")
        end

        -- Find all line numbers in the stack trace
        local stackTraceLines = {}
        for _, line in pairs(stackTrace:split("\n")) do
            local lineNum = line:match("line (%d+)$")
            if not lineNum then continue end

            table.insert(stackTraceLines, tonumber(lineNum))
        end

        -- Send the packet
        socket:Send("error", HttpService:JSONEncode({
            message_lines = messageLines,
            stack_trace_lines = stackTraceLines,
            message_content = messageContent,
        }))
    end)

    socket.connection.OnClose:Wait()
end