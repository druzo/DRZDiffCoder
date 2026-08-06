; Comments, strings, numbers
(comment) @comment
(string) @string
(number) @number
(nil) @constant
(true) @constant
(false) @constant

; Lua keywords (anonymous tokens from tree-sitter-lua 0.5.0)
"and" @keyword
"do" @keyword
"else" @keyword
"elseif" @keyword
"end" @keyword
"for" @keyword
"function" @keyword
"goto" @keyword
"if" @keyword
"in" @keyword
"local" @keyword
"not" @keyword
"or" @keyword
"repeat" @keyword
"return" @keyword
"then" @keyword
"until" @keyword
"while" @keyword

; Function declarations and calls
(function_declaration
  name: (identifier) @function)

(method_index_expression
  method: (identifier) @function)

(dot_index_expression
  field: (identifier) @variable)

; Variables and tables
(identifier) @variable
(table_constructor) @constructor
(field) @variable