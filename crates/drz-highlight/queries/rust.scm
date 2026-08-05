(line_comment) @comment
(block_comment) @comment
(string_literal) @string
(char_literal) @string
(integer_literal) @number
(float_literal) @number
(boolean_literal) @constant
(type_identifier) @type
(primitive_type) @type
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(mutable_specifier) @keyword
(crate) @keyword
(self) @keyword
(super) @keyword
[
  "fn" "let" "pub" "struct" "enum" "impl" "trait" "use" "mod" "match"
  "if" "else" "for" "while" "loop" "return" "const" "static"
  "async" "await" "move" "ref" "where" "type"
  "in" "as" "dyn" "unsafe" "break" "continue"
] @keyword
