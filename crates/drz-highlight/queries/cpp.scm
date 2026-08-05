(comment) @comment
(string_literal) @string
(char_literal) @string
(number_literal) @number
(true) @constant
(false) @constant
(null) @constant
"nullptr" @constant
(type_identifier) @type
(primitive_type) @type
(function_declarator declarator: (identifier) @function)
(call_expression function: (identifier) @function)
(this) @keyword
(auto) @keyword
[
  "if" "else" "for" "while" "do" "switch" "case" "default" "break"
  "continue" "return" "goto" "sizeof" "struct" "union" "enum" "typedef"
  "static" "const" "volatile" "extern" "inline" "class" "namespace"
  "template" "typename" "using" "public" "private" "protected" "virtual"
  "override" "final" "new" "delete" "try" "catch" "throw"
  "noexcept" "constexpr" "concept" "requires" "unsigned"
] @keyword
