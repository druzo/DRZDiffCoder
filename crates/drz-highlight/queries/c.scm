(comment) @comment
(string_literal) @string
(char_literal) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
(type_identifier) @type
(primitive_type) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
[
  "if" "else" "for" "while" "do" "switch" "case" "default" "break"
  "continue" "return" "goto" "sizeof" "struct" "union" "enum" "typedef"
  "static" "const" "volatile" "extern" "inline" "register" "unsigned"
  "signed"
] @keyword
