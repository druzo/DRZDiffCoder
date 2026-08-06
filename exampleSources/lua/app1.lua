-- Lua — table iteration with ipairs + custom sort.

local function by_priority(a, b)
  return a.priority < b.priority
end

local backlog = {
  { title = "Write tests",    priority = 2 },
  { title = "Fix login bug",  priority = 5 },
  { title = "Refactor parser", priority = 3 },
}

table.sort(backlog, by_priority)

for _, task in ipairs(backlog) do
  print(string.format("%d  %s", task.priority, task.title))
end