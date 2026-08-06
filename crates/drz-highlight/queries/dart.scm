; Dart — adapted from tree-sitter-dart upstream highlights.scm,
; mapped to drz-highlight's plain capture set (keyword, string, comment,
; function, type, number, constant, attribute).

(comment) @comment
(block_comment) @comment
(documentation_block_comment) @comment

(string_literal) @string

[
  (decimal_integer_literal)
  (hex_integer_literal)
  (decimal_floating_point_literal)
] @number

(true) @constant
(false) @constant
(null_literal) @constant

(type_identifier) @type
(void_type) @type
(class_declaration name: (identifier) @type)
(mixin_declaration (identifier) @type)
(extension_declaration name: (identifier) @type)
(enum_declaration name: (identifier) @type)

(function_signature name: (identifier) @function)
(method_signature (function_signature name: (identifier) @function))

[
  "abstract" "as" "assert" "async" "await" "base" "break" "case"
  "catch" "class" "const" "continue" "covariant" "default"
  "deferred" "do" "else" "enum" "export" "extends" "extension"
  "external" "factory" "final" "finally" "for" "get" "hide" "if"
  "implements" "import" "in" "interface" "is" "late" "library"
  "mixin" "native" "new" "on" "operator" "part" "required"
  "return" "sealed" "set" "show" "static" "super" "switch"
  "this" "throw" "try" "type" "typedef" "var" "when" "while"
  "with" "yield"
] @keyword