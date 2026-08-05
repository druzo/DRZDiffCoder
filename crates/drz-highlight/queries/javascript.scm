(comment) @comment
(string) @string
(template_string) @string
(number) @number
(true) @constant
(false) @constant
(null) @constant
(function_declaration name: (identifier) @function)
(call_expression function: (identifier) @function)
(super) @keyword
(this) @keyword
[
  "function" "const" "let" "var" "return" "if" "else" "for" "while"
  "do" "switch" "case" "default" "break" "continue" "new" "delete"
  "typeof" "instanceof" "in" "of" "class" "extends"
  "import" "export" "from" "async" "await" "try" "catch" "finally"
  "throw" "yield" "static" "get" "set"
] @keyword
