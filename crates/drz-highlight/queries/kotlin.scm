(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(multiline_string_literal) @string
(character_literal) @string

(number_literal) @number
(float_literal) @number

(function_declaration name: (identifier) @function)
(class_declaration name: (identifier) @type)
(object_declaration name: (identifier) @type)
(type_alias type: (identifier) @type)
(enum_entry (identifier) @constant)

(user_type) @type

[
  "abstract" "actual" "annotation" "as" "by" "catch" "class"
  "companion" "const" "constructor" "crossinline" "data" "delegate"
  "do" "dynamic" "else" "enum" "expect" "external" "field" "file"
  "final" "finally" "for" "fun" "get" "if" "import" "in" "infix"
  "init" "inline" "inner" "interface" "internal" "is" "lateinit"
  "noinline" "object" "open" "operator" "out" "override" "package"
  "param" "private" "property" "protected" "public" "receiver"
  "return" "sealed" "set" "setparam" "super" "suspend" "tailrec"
  "this" "throw" "try" "typealias" "val" "value" "var" "vararg"
  "when" "where" "while"
] @keyword