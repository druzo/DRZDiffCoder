-- Lua — metatable with __index for read-only default values.

local function readonly(defaults)
  return setmetatable({}, {
    __index = function(_, key)
      return defaults[key]
    end,
    __newindex = function(_, key, _)
      error("attempt to modify read-only field '" .. key .. "'", 2)
    end,
  })
end

local cfg = readonly({ host = "localhost", port = 8080, debug = false })

print("host:", cfg.host)
print("port:", cfg.port)
print("debug:", tostring(cfg.debug))

local ok, err = pcall(function() cfg.host = "evil" end)
if not ok then
  print("blocked:", err)
end