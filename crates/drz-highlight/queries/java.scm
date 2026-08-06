; Java — adapted from tree-sitter-java upstream highlights.scm,
; mapped to drz-highlight's plain capture set (keyword, string, comment,
; function, type, number, constant, attribute).

(line_comment) @comment
(block_comment) @comment

(string_literal) @string
(character_literal) @string

[
  (decimal_integer_literal)
  (hex_integer_literal)
  (octal_integer_literal)
  (decimal_floating_point_literal)
  (hex_floating_point_literal)
] @number

[
  (true)
  (false)
  (null_literal)
] @constant

(annotation name: (identifier) @attribute)
(marker_annotation name: (identifier) @attribute)

(type_identifier) @type

[
  (boolean_type)
  (integral_type)
  (floating_point_type)
  (void_type)
] @type

(method_declaration name: (identifier) @function)
(method_invocation name: (identifier) @function)

[
  "abstract" "assert" "break" "case" "catch" "class" "continue"
  "default" "do" "else" "enum" "extends" "final" "finally" "for"
  "if" "implements" "import" "instanceof" "interface" "module"
  "native" "new" "package" "private" "protected" "public" "record"
  "return" "static" "strictfp" "switch" "synchronized" "throw"
  "throws" "to" "transient" "try" "volatile" "while" "with" "yield"
  "sealed" "non-sealed" "permits" "exports" "opens" "provides"
  "requires" "uses" "when"
] @keyword