(comment) @comment
(string) @string
(integer) @number
(float) @number
(true) @constant
(false) @constant
(none) @constant
(function_definition name: (identifier) @function)
(call function: (identifier) @function)
[
  "def" "class" "return" "if" "elif" "else" "for" "while" "import"
  "from" "as" "with" "try" "except" "finally" "raise" "pass" "break"
  "continue" "lambda" "yield" "async" "await" "global" "nonlocal"
  "assert" "del" "in" "is" "not" "and" "or"
] @keyword
